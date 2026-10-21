/* Sum only observed first-to-last thread intervals; never claim whole lifetimes. */
#include "tool.h"
#include <limits.h>
#include <string.h>
_Static_assert(sizeof(TraceRow)==64,"LP64 kdebug record ABI");
#define KPC_THREAD_EVENT UINT32_C(0x25060020)
void aggregate_row(Aggregate *a,const TraceRow *r) {
 if((r->debugid&~3u)!=KPC_THREAD_EVENT)return;
 a->records++;
 uint64_t tid=r->arg[4];if(!tid){a->invalid=true;return;}
 size_t i=0;for(;i<a->thread_count;i++)if(a->threads[i].tid==tid)break;
 if(i==a->thread_count){if(i==MAX_THREADS){a->invalid=true;return;}a->threads[i].tid=tid;a->thread_count++;}
 Thread *t=&a->threads[i];unsigned function=r->debugid&3u;
 if(function==1){if(t->pending_count)a->invalid=true;t->pending_count=0;t->pending_tick=r->timestamp;}
 else if(!t->pending_count){a->invalid=true;return;}
 unsigned left=a->counter_count-t->pending_count;
 unsigned n=left<4?left:4;
 for(unsigned j=0;j<n;j++)t->pending[t->pending_count++]=r->arg[j];
 if(t->pending_count<a->counter_count){if(function==2)a->invalid=true;return;}
 if(a->counter_count>4&&function!=2)a->invalid=true;
 if(!t->samples){t->first_tick=t->pending_tick;memcpy(t->first,t->pending,sizeof(t->first));}
 else {
  if(t->pending_tick<=t->last_tick)a->invalid=true;
  for(unsigned j=0;j<a->counter_count;j++)if(t->pending[j]<t->last[j])a->invalid=true;
 }
 t->last_tick=t->pending_tick;memcpy(t->last,t->pending,sizeof(t->last));t->samples++;a->samples++;t->pending_count=0;
}
bool aggregate_total(const Aggregate *a,uint64_t *sums) {
 memset(sums,0,MAX_COUNTERS*sizeof(*sums));
 if(a->invalid||!a->thread_count)return false;
 for(size_t i=0;i<a->thread_count;i++){
  const Thread *t=&a->threads[i];if(t->pending_count||t->samples<2)return false;
  for(unsigned j=0;j<a->counter_count;j++){
   if(t->last[j]<t->first[j]||UINT64_MAX-sums[j]<t->last[j]-t->first[j])return false;
   sums[j]+=t->last[j]-t->first[j];
  }
 }
 return true;
}
