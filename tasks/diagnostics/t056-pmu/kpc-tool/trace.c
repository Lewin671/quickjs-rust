/* Own one idle ktrace session. Stage diagnostics preserve the FIRST failure. */
#include "tool.h"
#include <errno.h>
#include <sys/sysctl.h>
#include <unistd.h>
#include <string.h>
typedef struct {int nkdbufs,nolog;unsigned flags;int nkdthreads,bufid;} BufferInfo;
typedef struct {unsigned type,value1,value2,value3,value4;} Filter;
#define TRACE_ENTRIES 32768
#define WRAPPED 0x0008u
#define BUFFER_INITIALIZED 0x80000000u
_Static_assert(sizeof(BufferInfo)==20,"Monterey kbufinfo ABI");
bool trace_result(Run *r,const char *step,int value,int error,bool cleanup) {
 if(value==0){if(!cleanup)r->completed_setup_steps++;return true;}
 if(cleanup){if(!r->cleanup_failed_step){r->cleanup_failed_step=step;r->cleanup_return=value;r->cleanup_errno=error;}}
 else if(!r->failed_step){r->failed_step=step;r->failed_return=value;r->failed_errno=error;}
 return false;
}
/* Clear errno at the call, not after another API may have overwritten it. */
#define REQUIRE(step,call) do{errno=0;int rc=(call);int call_errno=errno;if(!trace_result(r,step,rc,call_errno,false))return false;}while(0)
#define CLEAN(step,call) do{errno=0;int rc=(call);int call_errno=errno;if(!trace_result(r,step,rc,call_errno,true))ok=false;}while(0)
static int info(BufferInfo *b) {
 int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDGETBUF};size_t n=sizeof(*b);
 int result=sysctl(mib,3,b,&n,NULL,0);
 if(result==0&&n!=sizeof(*b)){errno=EPROTO;return -1;}return result;
}
static int control(int op,int value,bool with_value) {
 int mib[]={CTL_KERN,KERN_KDEBUG,op,value};return sysctl(mib,with_value?4u:3u,NULL,NULL,NULL,0);
}
bool trace_idle(Api *a,Run *r) {
 BufferInfo b={0};uint32_t sampling=0;int forced=0;
 REQUIRE("idle.KDGETBUF",info(&b));r->initial_owner=b.bufid;r->initial_nolog=b.nolog;r->initial_flags=b.flags;
 /* nolog is a slow-path bit, not an initialized/active-state boolean. */
 REQUIRE("idle.foreign_owner",b.bufid>0&&b.bufid!=getpid()?EBUSY:0);
 REQUIRE("idle.active_buffer",(b.flags&BUFFER_INITIALIZED)&&!b.nolog?EBUSY:0);
 REQUIRE("idle.kperf_sample_get",a->kperf_sample_get(&sampling));
 REQUIRE("idle.sampling_disabled",sampling?EBUSY:0);
 REQUIRE("idle.kpc_force_all_ctrs_get",a->kpc_force_all_ctrs_get(&forced));
 REQUIRE("idle.counters_unforced",forced?EBUSY:0);
 uint32_t n=0;
 REQUIRE("idle.kperf_action_count_get",a->kperf_action_count_get(&n));
 REQUIRE("idle.action_count_bound",n>32?EPROTO:0);
 for(uint32_t i=1;i<=n;i++){
  uint32_t mask=0;REQUIRE("idle.kperf_action_samplers_get",a->kperf_action_samplers_get(i,&mask));
  REQUIRE("idle.action_inactive",mask?EBUSY:0);
 }
 REQUIRE("idle.kperf_timer_count_get",a->kperf_timer_count_get(&n));
 REQUIRE("idle.timer_count_bound",n>8?EPROTO:0);
 for(uint32_t i=0;i<n;i++){
  uint64_t ticks=0;REQUIRE("idle.kperf_timer_period_get",a->kperf_timer_period_get(i,&ticks));
  REQUIRE("idle.timer_inactive",ticks?EBUSY:0);
 }
 return true;
}
bool trace_begin(Api *a,const Events *e,Run *r) {
 if(!trace_idle(a,r))return false;
 REQUIRE("setup.KDSETBUF",control(KERN_KDSETBUF,TRACE_ENTRIES,true));r->trace_owned=true;
 BufferInfo b={0};REQUIRE("setup.KDGETBUF",info(&b));
 REQUIRE("setup.own_trace",b.bufid==getpid()?0:EBUSY);
 REQUIRE("setup.KDSETUP",control(KERN_KDSETUP,0,false));
 Filter filter={0x00200000u,0x25060020u,0,0,0};
 int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDSETREG};size_t size=sizeof(filter);
 REQUIRE("setup.KDSETREG",sysctl(mib,3,&filter,&size,NULL,0));
 errno=0;r->old_count=a->kpc_get_counting();int get_error=errno;
 if(!trace_result(r,"setup.kpc_get_counting",get_error?-1:0,get_error,false))return false;
 errno=0;r->old_thread=a->kpc_get_thread_counting();get_error=errno;
 if(!trace_result(r,"setup.kpc_get_thread_counting",get_error?-1:0,get_error,false))return false;
 REQUIRE("setup.kpc_get_config.before",a->kpc_get_config(e->classes,r->old_config));
 REQUIRE("setup.kpc_force_all_ctrs_set",a->kpc_force_all_ctrs_set(1));r->forced=true;r->changed=true;
 uint64_t requested[MAX_COUNTERS];memcpy(requested,e->config,sizeof(requested));
 REQUIRE("setup.kpc_set_config",a->kpc_set_config(e->classes,requested));
 REQUIRE("setup.kpc_get_config.readback",a->kpc_get_config(e->classes,r->readback));r->readback_available=true;
 REQUIRE("setup.config_equal",memcmp(r->readback,e->config,e->register_count*sizeof(uint64_t))?EPROTO:0);r->config_verified=true;
 REQUIRE("setup.kpc_set_counting",a->kpc_set_counting(e->classes));
 REQUIRE("setup.kpc_set_thread_counting",a->kpc_set_thread_counting(e->classes));
 uint32_t actions=0,timers=0;
 REQUIRE("setup.kperf_action_count_get",a->kperf_action_count_get(&actions));
 REQUIRE("setup.kperf_timer_count_get",a->kperf_timer_count_get(&timers));
 /* Even a failed KPERF write may acquire its ownership mask. Reset only after
    OUR write attempt, never create KPERF ownership just to clean KDEBUG. */
 r->perf_touched=true;
 if(!actions)REQUIRE("setup.kperf_action_count_set",a->kperf_action_count_set(1));
 if(!timers)REQUIRE("setup.kperf_timer_count_set",a->kperf_timer_count_set(1));
 REQUIRE("setup.kperf_action_samplers_set",a->kperf_action_samplers_set(1,1u<<4));
 REQUIRE("setup.kperf_filter_pid",a->kperf_action_filter_set_by_pid(1,r->pid));
 REQUIRE("setup.kperf_timer_period",a->kperf_timer_period_set(0,a->kperf_ns_to_ticks(SAMPLE_NS)));
 REQUIRE("setup.kperf_timer_action",a->kperf_timer_action_set(0,1));
 REQUIRE("setup.kperf_timer_pet",a->kperf_timer_pet_set(0));
 unsigned enabled=1;
 REQUIRE("setup.lightweight_pet",sysctlbyname("kperf.lightweight_pet",NULL,NULL,&enabled,sizeof(enabled)));
 REQUIRE("setup.KDENABLE",control(KERN_KDENABLE,1,true));
 REQUIRE("setup.kperf_sample_set",a->kperf_sample_set(1));
 r->aggregate.counter_count=e->counter_count;return true;
}
bool trace_drain(Run *r) {
 BufferInfo b={0};REQUIRE("drain.KDGETBUF",info(&b));
 REQUIRE("drain.own_trace",b.bufid==getpid()?0:EBUSY);
 REQUIRE("drain.not_wrapped",b.flags&WRAPPED?ENOBUFS:0);
 TraceRow rows[4096];size_t count=sizeof(rows);int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDREADTR};
 REQUIRE("drain.KDREADTR",sysctl(mib,3,rows,&count,NULL,0));
 REQUIRE("drain.record_bound",count>4096?EPROTO:0);
 for(size_t i=0;i<count;i++)aggregate_row(&r->aggregate,&rows[i]);
 REQUIRE("drain.record_integrity",r->aggregate.invalid?EPROTO:0);return true;
}
bool trace_finish(Api *a,const Events *e,Run *r) {
 bool ok=true;BufferInfo b={0};errno=0;int result=info(&b);int saved=errno;
 bool owned=result==0&&b.bufid==getpid();
 if(r->trace_owned||r->perf_touched){
  if(!trace_result(r,"cleanup.KDGETBUF",result,saved,true))ok=false;
  if(!trace_result(r,"cleanup.own_trace",owned?0:EBUSY,0,true))ok=false;
 }
 if(owned&&r->perf_touched)CLEAN("cleanup.kperf_reset",a->kperf_reset());
 if(owned&&r->trace_owned)CLEAN("cleanup.KDENABLE",control(KERN_KDENABLE,0,true));
 if(r->changed){
  CLEAN("cleanup.kpc_set_thread_counting",a->kpc_set_thread_counting(r->old_thread));
  CLEAN("cleanup.kpc_set_counting",a->kpc_set_counting(r->old_count));
  CLEAN("cleanup.kpc_set_config",a->kpc_set_config(e->classes,r->old_config));
 }
 if(r->forced)CLEAN("cleanup.kpc_force_all_ctrs_set",a->kpc_force_all_ctrs_set(0));
 if(owned&&r->trace_owned)CLEAN("cleanup.KDREMOVE",control(KERN_KDREMOVE,0,false));
 if(r->trace_owned||r->perf_touched){
  BufferInfo after={0};errno=0;result=info(&after);saved=errno;r->final_owner=after.bufid;
  if(!trace_result(r,"cleanup.final_KDGETBUF",result,saved,true))ok=false;
  if(result==0&&!trace_result(r,"cleanup.owner_released",after.bufid==getpid()?EBUSY:0,0,true))ok=false;
 }
 r->trace_owned=false;r->forced=false;r->changed=false;r->perf_touched=false;return ok;
}
