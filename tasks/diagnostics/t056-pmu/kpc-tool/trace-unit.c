/* Synthetic kernel/API failures. Never linked into the installed helper. */
#include "tool.h"
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/sysctl.h>
#include <unistd.h>
static unsigned mask,kperf_writes,reset_calls,remove_calls,force_calls,config_calls;
static int owner;static bool initialized,enabled,mismatch,fail_get,uninitialized_nolog_zero;
static uint32_t sampling,actions,timers,action_bits,old_count,old_thread;
static uint64_t registers[MAX_COUNTERS],period;
static const char *failure;
pid_t test_getpid(void){return 42;}
static int inject(const char *name){if(failure&&!strcmp(name,failure)){errno=EIO;return -1;}return 0;}
static int perf_write(void){if(owner&&owner!=42){errno=EBUSY;return -1;}owner=42;mask|=2;kperf_writes++;return 0;}
int test_sysctl(int *name,u_int n,void *out,size_t *length,void *in,size_t size) {
 (void)n;(void)in;(void)size;
 switch(name[2]){
 case KERN_KDGETBUF:{
  if(inject("info"))return -1;
  struct {int buffers,nolog;unsigned flags;int threads,pid;} b={initialized?32768:0,(enabled||uninitialized_nolog_zero)?0:1,initialized?0x80000000u:0,0,owner?owner:-1};
  assert(out&&length&&*length==sizeof(b));memcpy(out,&b,sizeof(b));return 0;
 }
 case KERN_KDSETBUF:if(inject("setbuf"))return -1;assert(!owner||owner==42);owner=42;mask|=1;return 0;
 case KERN_KDSETUP:if(inject("setup"))return -1;initialized=true;return 0;
 case KERN_KDSETREG:if(inject("filter"))return -1;return 0;
 case KERN_KDENABLE:assert(owner==42);enabled=name[3]!=0;return 0;
 case KERN_KDREMOVE:assert(owner==42);remove_calls++;mask&=~1u;initialized=false;enabled=false;if(!mask)owner=0;return 0;
 default:assert(0);return -1;
 }
}
int test_sysctlbyname(const char *name,void *out,size_t *length,void *in,size_t size){
 (void)out;(void)length;assert(!strcmp(name,"kperf.lightweight_pet")&&size==4&&in);return perf_write();
}
static int force_get(int *v){*v=0;return 0;}
static int force_set(int value){force_calls++;return inject(value?"force":"release");}
static uint32_t count_get(void){if(fail_get){errno=EACCES;return 0;}return old_count;}
static uint32_t thread_get(void){return old_thread;}
static int get_config(uint32_t classes,uint64_t *out){(void)classes;if(inject("getconfig"))return -1;memcpy(out,registers,sizeof(registers));if(mismatch&&config_calls==1)out[0]^=1;return 0;}
static int set_config(uint32_t classes,uint64_t *in){(void)classes;config_calls++;memcpy(registers,in,sizeof(registers));return inject("setconfig");}
static int set_count(uint32_t classes){old_count=classes;return 0;}
static int set_thread(uint32_t classes){old_thread=classes;return 0;}
static int sample_get(uint32_t *v){*v=sampling;return 0;}
static int sample_set(uint32_t v){if(perf_write())return -1;sampling=v;return inject("sample");}
static int action_get(uint32_t *v){*v=actions;return 0;}
static int action_set(uint32_t v){if(perf_write())return -1;actions=v;return inject("actions");}
static int timer_get(uint32_t *v){*v=timers;return 0;}
static int timer_set(uint32_t v){if(perf_write())return -1;timers=v;return 0;}
static int samplers_get(uint32_t id,uint32_t *v){assert(id==1);*v=action_bits;return 0;}
static int samplers_set(uint32_t id,uint32_t v){assert(id==1);if(perf_write())return -1;action_bits=v;return inject("samplers");}
static int filter_set(uint32_t id,int32_t pid){assert(id==1&&pid==43);return perf_write();}
static int period_get(uint32_t id,uint64_t *v){assert(id==0);*v=period;return 0;}
static int period_set(uint32_t id,uint64_t v){assert(id==0);if(perf_write())return -1;period=v;return 0;}
static int timer_action(uint32_t timer,uint32_t action){assert(timer==0&&action==1);return perf_write();}
static int timer_pet(uint32_t id){assert(id==0);return perf_write();}
static uint64_t ticks(uint64_t n){return n;}
static int reset(void){assert(owner==42);reset_calls++;sampling=0;actions=0;timers=0;action_bits=0;period=0;mask&=~2u;if(!mask)owner=0;return inject("reset");}
static Api api(void){
 Api a={0};a.kpc_force_all_ctrs_get=force_get;a.kpc_force_all_ctrs_set=force_set;
 a.kpc_get_counting=count_get;a.kpc_get_thread_counting=thread_get;a.kpc_get_config=get_config;a.kpc_set_config=set_config;a.kpc_set_counting=set_count;a.kpc_set_thread_counting=set_thread;
 a.kperf_sample_get=sample_get;a.kperf_sample_set=sample_set;a.kperf_action_count_get=action_get;a.kperf_action_count_set=action_set;a.kperf_timer_count_get=timer_get;a.kperf_timer_count_set=timer_set;
 a.kperf_action_samplers_get=samplers_get;a.kperf_action_samplers_set=samplers_set;a.kperf_action_filter_set_by_pid=filter_set;a.kperf_timer_period_get=period_get;a.kperf_timer_period_set=period_set;a.kperf_timer_action_set=timer_action;a.kperf_timer_pet_set=timer_pet;a.kperf_ns_to_ticks=ticks;a.kperf_reset=reset;return a;
}
static void clear(void){
 mask=kperf_writes=reset_calls=remove_calls=force_calls=config_calls=0;owner=0;initialized=enabled=mismatch=fail_get=uninitialized_nolog_zero=false;sampling=actions=timers=action_bits=old_count=old_thread=0;memset(registers,0,sizeof(registers));period=0;failure=NULL;
}
int main(void){
 Api a=api();Events e={0};e.classes=1;e.counter_count=3;e.register_count=1;e.config[0]=0x22;
 const char *fails[]={"setbuf","setup","filter","force","getconfig","setconfig","actions","samplers","sample"};
 for(size_t i=0;i<sizeof(fails)/sizeof(fails[0]);i++){
  clear();failure=fails[i];Run r={0};r.pid=43;assert(!trace_begin(&a,&e,&r)&&r.failed_step);const char *original=r.failed_step;int rc=r.failed_return,err=r.failed_errno;
  failure=NULL;assert(trace_finish(&a,&e,&r));assert(r.failed_step==original&&r.failed_return==rc&&r.failed_errno==err&&owner==0&&mask==0);
  assert(!actions&&!timers&&!action_bits&&!sampling&&!period);
 }
 clear();mismatch=true;Run r={0};r.pid=43;
 assert(!trace_begin(&a,&e,&r));assert(!strcmp(r.failed_step,"setup.config_equal")&&r.failed_return==EPROTO&&r.failed_errno==0&&r.readback_available&&r.readback[0]==0x23);
 assert(trace_finish(&a,&e,&r));assert(kperf_writes==0&&reset_calls==0&&remove_calls==1&&owner==0);
 clear();fail_get=true;memset(&r,0,sizeof(r));r.pid=43;assert(!trace_begin(&a,&e,&r));assert(r.failed_errno==EACCES&&!strcmp(r.failed_step,"setup.kpc_get_counting"));assert(trace_finish(&a,&e,&r));
 clear();memset(&r,0,sizeof(r));r.pid=43;assert(trace_begin(&a,&e,&r));assert(trace_finish(&a,&e,&r));assert(reset_calls==1&&owner==0&&mask==0&&registers[0]==0);
 /* A second session succeeds only if action/timer state and BOTH ownership masks reset. */
 memset(&r,0,sizeof(r));r.pid=43;assert(trace_begin(&a,&e,&r)&&trace_finish(&a,&e,&r)&&reset_calls==2);
 clear();owner=99;mask=3;initialized=true;enabled=true;memset(&r,0,sizeof(r));r.pid=43;
 assert(!trace_begin(&a,&e,&r));assert(trace_finish(&a,&e,&r)&&owner==99&&mask==3&&reset_calls==0&&remove_calls==0&&force_calls==0);
 clear();uninitialized_nolog_zero=true;memset(&r,0,sizeof(r));r.pid=43;assert(trace_begin(&a,&e,&r)&&trace_finish(&a,&e,&r));
 clear();memset(&r,0,sizeof(r));r.pid=43;assert(trace_begin(&a,&e,&r));failure="reset";assert(!trace_finish(&a,&e,&r));assert(r.cleanup_failed_step&&!strcmp(r.cleanup_failed_step,"cleanup.kperf_reset")&&r.cleanup_errno==EIO&&owner==0);
 puts("trace-unit: API failure+errno/readback, pre-PERF cleanup, full reset, repeat session, foreign owner: pass");return 0;
}
