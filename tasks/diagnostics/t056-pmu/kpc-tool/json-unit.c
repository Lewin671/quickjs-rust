/* Real JSON serialization checks; all inputs synthetic, no PMU operations. */
#include "tool.h"
#include <errno.h>
#include <stdlib.h>
int main(void){
 Api a={0};Events e={0};e.event_count=2;e.counter_count=4;e.register_count=4;
 e.names[0]="CPU_CLK_UNHALTED.THREAD";e.map[0]=0;e.names[1]="INST_RETIRED.ANY";e.map[1]=1;
 Run *r=calloc(1,sizeof(*r));if(!r)return 1;r->cleanup_ok=true;r->config_verified=true;r->counts_valid=true;r->cpu_count=1;
 r->sums[0]=50;r->sums[1]=100;r->idle_sums[0]=2;r->idle_sums[1]=4;
 r->before.valid=r->after.valid=r->idle_before.valid=r->idle_after.valid=true;
 r->after.values[0]=50;r->after.values[1]=100;r->idle_after.values[0]=2;r->idle_after.values[1]=4;
 run_json(&a,&e,r,true);
 r->failed_step="setup.config_equal";r->failed_return=EPROTO;r->readback_available=true;r->readback[0]=0x23;
 r->cleanup_failed_step="cleanup.kpc_force_all_ctrs_set";r->cleanup_return=-1;r->cleanup_errno=EIO;r->cleanup_ok=false;
 run_json(&a,&e,r,false);free(r);return 0;
}
