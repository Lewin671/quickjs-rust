/* Exclusive forced KPC counting, without any KDEBUG/KPERF sampler ownership.
   Snapshot ABI is checked against hw.ncpu and the KPC class count each read. */
#include "tool.h"
#include <errno.h>
#include <string.h>
#include <sys/sysctl.h>
#include <time.h>
uint64_t monotonic_ns(void) {
 struct timespec t;if(clock_gettime(CLOCK_MONOTONIC,&t))return 0;
 return (uint64_t)t.tv_sec*UINT64_C(1000000000)+(uint64_t)t.tv_nsec;
}
bool count_result(Run *r,const char *step,int value,int error,bool cleanup) {
 if(!value){if(!cleanup)r->completed_setup_steps++;return true;}
 if(cleanup){if(!r->cleanup_failed_step){r->cleanup_failed_step=step;r->cleanup_return=value;r->cleanup_errno=error;}}
 else if(!r->failed_step){r->failed_step=step;r->failed_return=value;r->failed_errno=error;}
 return false;
}
#define REQUIRE(step,call) do{errno=0;int rc=(call);int call_errno=errno;if(!count_result(r,step,rc,call_errno,false))return false;}while(0)
#define CLEAN(step,call) do{errno=0;int rc=(call);int call_errno=errno;if(!count_result(r,step,rc,call_errno,true))ok=false;}while(0)
static int cpu_count(unsigned *result) {
 int n=0,maximum=0;size_t size=sizeof(n);
 if(sysctlbyname("hw.ncpu",&n,&size,NULL,0))return -1;
 if(size!=sizeof(n)){errno=EPROTO;return -1;}
 size=sizeof(maximum);if(sysctlbyname("hw.logicalcpu_max",&maximum,&size,NULL,0))return -1;
 if(size!=sizeof(maximum)||n<1||n>MAX_CPUS||maximum!=n){errno=EPROTO;return -1;}
 *result=(unsigned)n;return 0;
}
bool count_begin(Api *a,const Events *e,Run *r) {
 int forced=0;REQUIRE("setup.cpu_count",cpu_count(&r->cpu_count));
 REQUIRE("setup.kpc_force_all_ctrs_get",a->kpc_force_all_ctrs_get(&forced));
 REQUIRE("setup.counters_unforced",forced?EBUSY:0);
 errno=0;r->old_count=a->kpc_get_counting();int saved=errno;
 if(!count_result(r,"setup.kpc_get_counting",saved?-1:0,saved,false))return false;
 /* An unforced fixed-only baseline may belong to power management. Preserve it;
    never take over an existing programmable or per-thread profiler. */
 REQUIRE("setup.no_programmable_profiler",r->old_count&~1u?EBUSY:0);
 errno=0;uint32_t thread=a->kpc_get_thread_counting();saved=errno;
 if(!count_result(r,"setup.kpc_get_thread_counting",saved?-1:0,saved,false))return false;
 REQUIRE("setup.no_thread_profiler",thread?EBUSY:0);
 REQUIRE("setup.kpc_get_config.before",a->kpc_get_config(e->classes,r->old_config));
 REQUIRE("setup.kpc_force_all_ctrs_set",a->kpc_force_all_ctrs_set(1));r->forced=true;
 uint64_t requested[MAX_COUNTERS];memcpy(requested,e->config,sizeof(requested));
 REQUIRE("setup.kpc_set_config",a->kpc_set_config(e->classes,requested));
 REQUIRE("setup.kpc_get_config.readback",a->kpc_get_config(e->classes,r->readback));r->readback_available=true;
 REQUIRE("setup.config_equal",memcmp(r->readback,e->config,e->register_count*sizeof(uint64_t))?EPROTO:0);r->config_verified=true;
 REQUIRE("setup.kpc_set_counting",a->kpc_set_counting(e->classes));return true;
}
bool count_read(Api *a,const Events *e,Run *r,Snapshot *s) {
 unsigned cpus=0;REQUIRE("read.cpu_count",cpu_count(&cpus));
 REQUIRE("read.cpu_shape",cpus!=r->cpu_count?EPROTO:0);
 REQUIRE("read.counter_shape",a->kpc_get_counter_count(e->classes)!=e->counter_count?EPROTO:0);
 errno=0;uint32_t mask=a->kpc_get_counting();int saved=errno;
 if(!count_result(r,"read.counting_mask",saved?-1:mask!=e->classes?EPROTO:0,saved,false))return false;
 for(size_t i=0;i<MAX_CPUS*MAX_COUNTERS;i++)s->values[i]=UINT64_MAX;
 s->begin_ns=monotonic_ns();REQUIRE("read.clock",s->begin_ns?0:EIO);
 REQUIRE("read.kpc_get_cpu_counters",a->kpc_get_cpu_counters(true,e->classes,&s->current_cpu,s->values));
 s->end_ns=monotonic_ns();REQUIRE("read.clock_order",s->end_ns>=s->begin_ns?0:EPROTO);
 REQUIRE("read.current_cpu",s->current_cpu<0||(unsigned)s->current_cpu>=cpus?EPROTO:0);
 size_t used=(size_t)cpus*e->counter_count;
 for(size_t i=0;i<MAX_CPUS*MAX_COUNTERS;i++)REQUIRE("read.buffer_shape",(i<used)==(s->values[i]==UINT64_MAX)?EPROTO:0);
 s->valid=true;return true;
}
bool count_delta(const Events *e,unsigned cpus,const Snapshot *a,const Snapshot *b,uint64_t *sum) {
 memset(sum,0,MAX_COUNTERS*sizeof(*sum));
 if(!cpus||cpus>MAX_CPUS||!e->counter_count||e->counter_count>MAX_COUNTERS||!a->valid||!b->valid||b->begin_ns<a->end_ns)return false;
 for(unsigned cpu=0;cpu<cpus;cpu++)for(unsigned i=0;i<e->counter_count;i++){
  size_t index=(size_t)cpu*e->counter_count+i;
  if(b->values[index]<a->values[index])return false;
  uint64_t delta=b->values[index]-a->values[index];if(UINT64_MAX-sum[i]<delta)return false;sum[i]+=delta;
 }
 return true;
}
bool count_finish(Api *a,const Events *e,Run *r) {
 bool ok=true;if(!r->forced)return true;
 /* Try every release even after an earlier cleanup error. A failed release is
    reported, never called clean. Kernel task teardown is the final backstop. */
 CLEAN("cleanup.kpc_set_counting.zero",a->kpc_set_counting(0));
 CLEAN("cleanup.kpc_set_config",a->kpc_set_config(e->classes,r->old_config));
 if(r->old_count)CLEAN("cleanup.kpc_set_counting.restore",a->kpc_set_counting(r->old_count));
 CLEAN("cleanup.kpc_force_all_ctrs_set",a->kpc_force_all_ctrs_set(0));
 int forced=1;CLEAN("cleanup.kpc_force_all_ctrs_get",a->kpc_force_all_ctrs_get(&forced));
 CLEAN("cleanup.counters_released",forced?EBUSY:0);
 errno=0;uint32_t mask=a->kpc_get_counting();int saved=errno;
 if(!count_result(r,"cleanup.counting_restored",saved?-1:mask!=r->old_count?EPROTO:0,saved,true))ok=false;
 r->forced=false;return ok;
}
