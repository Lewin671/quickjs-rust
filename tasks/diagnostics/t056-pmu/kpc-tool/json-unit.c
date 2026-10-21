/* Serialize both programmable counter indices and separate setup/cleanup errors. */
#include "tool.h"
#include <errno.h>
static uint64_t ticks(uint64_t n){return n;}
int main(void){
 Api a={0};a.kperf_ticks_to_ns=ticks;Events e={0};e.event_count=2;e.counter_count=4;e.register_count=4;
 e.names[0]="CPU_CLK_UNHALTED.THREAD";e.map[0]=0;e.names[1]="INST_RETIRED.ANY";e.map[1]=1;
 Run r={0};r.cleanup_ok=true;r.config_verified=true;r.aggregate.counter_count=4;r.aggregate.thread_count=1;
 Thread *t=&r.aggregate.threads[0];t->tid=UINT64_C(0x100000001);t->samples=2;t->first_tick=1;t->last_tick=2;t->first[0]=10;t->last[0]=60;t->first[1]=20;t->last[1]=120;
 run_json(&a,&e,&r,true);
 r.failed_step="setup.config_equal";r.failed_return=EPROTO;r.readback_available=true;r.readback[0]=0x23;
 r.cleanup_failed_step="cleanup.kperf_reset";r.cleanup_return=-1;r.cleanup_errno=EIO;r.cleanup_ok=false;
 run_json(&a,&e,&r,false);return 0;
}
