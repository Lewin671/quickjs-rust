/* Fault injection exercises real counting lifecycle, never loads private APIs. */
#include "tool.h"
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static int forced,foreign,bad_shape,falling,bad_readback,cleanup_fault,operations,fail_at,ncpu=2;
static uint32_t mask,thread_mask;static uint64_t config[MAX_COUNTERS];
static int fault(void){operations++;if(operations==fail_at){errno=EIO;return -1;}return 0;}
int test_sysctlbyname(const char *name,void *old,size_t *size,void *value,size_t length){
 (void)value;(void)length;if(fault())return -1;assert(!strcmp(name,"hw.ncpu")||!strcmp(name,"hw.logicalcpu_max"));assert(*size==sizeof(int));*(int *)old=ncpu;return 0;
}
static int force_get(int *n){if(fault())return -1;*n=foreign?1:forced;return 0;}
static int force_set(int n){if(fault()||(cleanup_fault==1&&!n)){errno=EIO;return -1;}forced=n;return 0;}
static uint32_t get_count(void){if(fault())return 0;return mask;}
static uint32_t get_thread(void){if(fault())return 0;return thread_mask;}
static uint32_t counter_count(uint32_t classes){assert(classes==2);return 4;}
static int get_config(uint32_t classes,uint64_t *v){assert(classes==2);if(fault())return -1;memcpy(v,config,sizeof(config));if(forced&&bad_readback)v[0]++;return 0;}
static int set_config(uint32_t classes,uint64_t *v){assert(classes==2);if(fault())return -1;memcpy(config,v,sizeof(config));return 0;}
static int set_count(uint32_t n){if(fault()||(cleanup_fault==2&&!n)){errno=EIO;return -1;}mask=n;return 0;}
static int get_cpu(bool all,uint32_t classes,int *cpu,uint64_t *v){
 assert(all&&classes==2);if(fault())return -1;*cpu=0;
 for(int i=0;i<ncpu*4+bad_shape;i++)v[i]=(uint64_t)(falling?1:operations*100+i);return 0;
}
static Api api={.kpc_get_counting=get_count,.kpc_get_thread_counting=get_thread,.kpc_get_counter_count=counter_count,.kpc_force_all_ctrs_get=force_get,.kpc_force_all_ctrs_set=force_set,.kpc_get_config=get_config,.kpc_set_config=set_config,.kpc_set_counting=set_count,.kpc_get_cpu_counters=get_cpu};
static Events events={.classes=2,.counter_count=4,.register_count=4,.config={0x41003c,0x4100c0,0,0}};
static void reset(Run *r){memset(r,0,sizeof(*r));memset(config,0,sizeof(config));forced=foreign=bad_shape=falling=bad_readback=cleanup_fault=operations=fail_at=0;mask=thread_mask=0;ncpu=2;}
static bool capture(Run *r){return count_begin(&api,&events,r)&&count_read(&api,&events,r,&r->before)&&count_read(&api,&events,r,&r->after)&&count_delta(&events,r->cpu_count,&r->before,&r->after,r->sums);}
int main(void){
 Run *r=calloc(1,sizeof(*r));assert(r);reset(r);assert(capture(r));int steps=operations;assert(r->sums[0]>0);assert(count_finish(&api,&events,r)&&!forced&&!mask);
 /* Every setup/read API failure leaves no owned counters, including failures
    after force/config/count enable; first error remains after cleanup. */
 for(int i=1;i<=steps;i++){reset(r);fail_at=i;assert(!capture(r));assert(r->failed_step);const char *first=r->failed_step;fail_at=0;assert(count_finish(&api,&events,r));assert(!forced&&!mask&&first==r->failed_step);}
 reset(r);bad_readback=1;assert(!capture(r));assert(!strcmp(r->failed_step,"setup.config_equal"));assert(count_finish(&api,&events,r)&&!forced&&!mask);
 reset(r);ncpu=257;assert(!capture(r)&&!forced);assert(count_finish(&api,&events,r));
 reset(r);foreign=1;assert(!capture(r)&&!forced);int prior=operations;assert(count_finish(&api,&events,r)&&operations==prior);
 reset(r);mask=2;assert(!capture(r)&&!forced&&mask==2);
 reset(r);thread_mask=2;assert(!capture(r)&&!forced);
 reset(r);mask=1;assert(capture(r)&&count_finish(&api,&events,r)&&mask==1&&!forced);
 reset(r);assert(count_begin(&api,&events,r));bad_shape=1;assert(!count_read(&api,&events,r,&r->before));assert(count_finish(&api,&events,r)&&!forced&&!mask);
 reset(r);assert(count_begin(&api,&events,r));assert(count_read(&api,&events,r,&r->before));ncpu=1;assert(!count_read(&api,&events,r,&r->after));assert(count_finish(&api,&events,r));
 reset(r);assert(count_begin(&api,&events,r));assert(count_read(&api,&events,r,&r->before));falling=1;assert(count_read(&api,&events,r,&r->after));assert(!count_delta(&events,r->cpu_count,&r->before,&r->after,r->sums));assert(count_finish(&api,&events,r));
 reset(r);assert(capture(r));r->before.values[0]=r->before.values[4]=0;r->after.values[0]=r->after.values[4]=UINT64_MAX-1;assert(!count_delta(&events,2,&r->before,&r->after,r->sums));assert(count_finish(&api,&events,r));
 for(int i=1;i<=2;i++){reset(r);assert(capture(r));cleanup_fault=i;assert(!count_finish(&api,&events,r)&&r->cleanup_failed_step);if(i==2)assert(!forced);cleanup_fault=0;/* Retry only in mock to reset its state. */forced=0;mask=0;}
 reset(r);assert(capture(r)&&count_finish(&api,&events,r));assert(capture(r)&&count_finish(&api,&events,r));
 free(r);puts("count-unit: all setup/read failures release; foreign ownership, baseline restore, bounds, overflow, cleanup diagnostics, repeat: pass");return 0;
}
