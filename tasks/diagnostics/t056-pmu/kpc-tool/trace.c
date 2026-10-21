/* Own one idle ktrace session. No takeover/reset of another profiler's state. */
#include "tool.h"
#include <errno.h>
#include <sys/sysctl.h>
#include <unistd.h>
#include <string.h>
typedef struct {int nkdbufs,nolog;unsigned flags;int nkdthreads,bufid;} BufferInfo;
typedef struct {unsigned type,value1,value2,value3,value4;} Filter;
#define TRACE_ENTRIES 32768
#define WRAPPED 0x0008u
static bool info(BufferInfo *b) {
 int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDGETBUF};size_t n=sizeof(*b);
 return sysctl(mib,3,b,&n,NULL,0)==0&&n==sizeof(*b);
}
static bool control(int op,int value,bool with_value) {
 int mib[]={CTL_KERN,KERN_KDEBUG,op,value};return sysctl(mib,with_value?4u:3u,NULL,NULL,NULL,0)==0;
}
static bool lightweight(unsigned value) {return sysctlbyname("kperf.lightweight_pet",NULL,NULL,&value,sizeof(value))==0;}
bool trace_idle(Api *a) {
 BufferInfo b={0};uint32_t sampling=0;int forced=0;
 if(!info(&b)||b.bufid>0||!b.nolog||a->kperf_sample_get(&sampling)||sampling)return false;
 if(a->kpc_force_all_ctrs_get(&forced)||forced)return false;
 /* Allocated idle action/timer slots survive kernel reset; check their contents. */
 uint32_t n=0;
 if(a->kperf_action_count_get(&n)||n>32)return false;
 for(uint32_t i=1;i<=n;i++){
  uint32_t mask=0;
  if(a->kperf_action_samplers_get(i,&mask)||mask)return false;
 }
 if(a->kperf_timer_count_get(&n)||n>8)return false;
 for(uint32_t i=0;i<n;i++){
  uint64_t ticks=0;
  if(a->kperf_timer_period_get(i,&ticks)||ticks)return false;
 }
 return true;
}
bool trace_begin(Api *a,const Events *e,Run *r) {
 if(!trace_idle(a))return false;
 /* KDSETBUF acquires ktrace ownership atomically; do not KDREMOVE first. */
 if(!control(KERN_KDSETBUF,TRACE_ENTRIES,true))return false;
 r->trace_owned=true;
 BufferInfo b={0};if(!info(&b)||b.bufid!=getpid())return false;
 if(!control(KERN_KDSETUP,0,false))return false;
 Filter filter={0x00200000u,0x25060020u,0,0,0};
 int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDSETREG};size_t size=sizeof(filter);
 if(sysctl(mib,3,&filter,&size,NULL,0))return false;
 r->old_count=a->kpc_get_counting();r->old_thread=a->kpc_get_thread_counting();
 if(a->kpc_get_config(e->classes,r->old_config))return false;
 if(a->kpc_force_all_ctrs_set(1))return false;
 r->forced=true;r->changed=true;
 uint64_t requested[MAX_COUNTERS];memcpy(requested,e->config,sizeof(requested));
 if(a->kpc_set_config(e->classes,requested))return false;
 uint64_t readback[MAX_COUNTERS]={0};
 if(a->kpc_get_config(e->classes,readback)||memcmp(readback,e->config,e->register_count*sizeof(uint64_t)))return false;
 r->config_verified=true;
 if(a->kpc_set_counting(e->classes)||a->kpc_set_thread_counting(e->classes))return false;
 uint32_t actions=0,timers=0;
 if(a->kperf_action_count_get(&actions)||a->kperf_timer_count_get(&timers))return false;
 if(!actions&&a->kperf_action_count_set(1))return false;
 if(!timers&&a->kperf_timer_count_set(1))return false;
 if(a->kperf_action_samplers_set(1,1u<<4)||a->kperf_action_filter_set_by_pid(1,r->pid))return false;
 if(a->kperf_timer_period_set(0,a->kperf_ns_to_ticks(SAMPLE_NS))||a->kperf_timer_action_set(0,1)||a->kperf_timer_pet_set(0)||!lightweight(1))return false;
 if(!control(KERN_KDENABLE,1,true)||a->kperf_sample_set(1))return false;
 r->aggregate.counter_count=e->counter_count;return true;
}
bool trace_drain(Run *r) {
 BufferInfo b={0};if(!info(&b)||b.bufid!=getpid()||(b.flags&WRAPPED))return false;
 TraceRow rows[4096];size_t count=sizeof(rows);
 int mib[]={CTL_KERN,KERN_KDEBUG,KERN_KDREADTR};
 /* XNU KDREADTR takes bytes and returns number of records. */
 if(sysctl(mib,3,rows,&count,NULL,0)||count>4096)return false;
 for(size_t i=0;i<count;i++)aggregate_row(&r->aggregate,&rows[i]);
 return !r->aggregate.invalid;
}
bool trace_finish(Api *a,const Events *e,Run *r) {
 bool ok=true;BufferInfo b={0};bool owned=r->trace_owned&&info(&b)&&b.bufid==getpid();
 if(r->trace_owned&&!owned)ok=false;
 if(owned){if(a->kperf_sample_set(0))ok=false;if(!lightweight(0))ok=false;if(!control(KERN_KDENABLE,0,true))ok=false;}
 if(r->changed){
  if(a->kpc_set_thread_counting(r->old_thread))ok=false;
  if(a->kpc_set_counting(r->old_count))ok=false;
  if(a->kpc_set_config(e->classes,r->old_config))ok=false;
 }
 if(r->forced&&a->kpc_force_all_ctrs_set(0))ok=false;
 /* Kernel resets timers/actions only after verifying this process owns ktrace. */
 if(owned&&!control(KERN_KDREMOVE,0,false))ok=false;
 r->trace_owned=false;r->forced=false;r->changed=false;return ok;
}
