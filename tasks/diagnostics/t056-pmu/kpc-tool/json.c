/* One JSON record on stdout. Counter integers are observed intervals, not totals. */
#include "tool.h"
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
static void json_bytes(const char *s,size_t length) {
 putchar('"');for(size_t i=0;i<length;i++){const unsigned char *p=(const unsigned char *)s+i;
  if(*p=='"'||*p=='\\'){putchar('\\');putchar(*p);}
  else if(*p<32||*p>=127)printf("\\u%04x",(unsigned)*p);
  else putchar(*p);
 }putchar('"');
}
void json_string(const char *s) {json_bytes(s,strlen(s));}
int fail(const char *message) {printf("{\"version\":\"" TOOL_VERSION "\",\"error\":");json_string(message);puts(",\"capture_complete\":false,\"counters_valid\":false}");return 1;}
void run_json(const Api *a,const Events *e,const Run *r,bool complete) {
 size_t cycle_index=0,instruction_index=0;for(size_t i=0;i<e->event_count;i++){if(!strcmp(e->names[i],"CPU_CLK_UNHALTED.THREAD"))cycle_index=e->map[i];if(!strcmp(e->names[i],"INST_RETIRED.ANY"))instruction_index=e->map[i];}
 uint64_t sums[MAX_COUNTERS];bool valid=complete&&aggregate_total(&r->aggregate,sums);
 int raw_status=r->status;
 int status=WIFEXITED(raw_status)?WEXITSTATUS(raw_status):WIFSIGNALED(raw_status)?128+WTERMSIG(raw_status):255;
 printf("{\"counter_backend\":\"%s\",\"version\":\"" TOOL_VERSION "\",\"pid\":%d,\"capture_complete\":%s,\"counters_valid\":%s,\"child_status\":%d,\"timed_out\":%s,\"elapsed_ns\":%llu,",e->fixed_diagnostic?"fixed_diagnostic":"kpep_programmable_user",r->pid,complete?"true":"false",valid?"true":"false",status,r->timed_out?"true":"false",(unsigned long long)r->elapsed_ns);
 printf("\"child_window_ns\":%llu,\"scope\":\"sum_of_observed_thread_intervals\",\"all_threads_covered\":false,\"full_lifetime_counts\":false,\"sample_period_ns\":1000000,\"user_only_requested\":true,\"child_stdout\":",(unsigned long long)r->child_window_ns);json_bytes(r->output,r->out_len);printf(",\"child_stderr\":");json_bytes(r->error,r->err_len);
 printf(",\"events\":[");
 for(size_t i=0;i<e->event_count;i++){
  if(i)putchar(',');printf("{\"name\":");json_string(e->names[i]);printf(",\"index\":%zu,\"count\":",e->map[i]);
  if(valid)printf("%llu",(unsigned long long)sums[e->map[i]]);else printf("null");putchar('}');
 }
 printf("],\"failed_step\":");if(r->failed_step)json_string(r->failed_step);else printf("null");
 printf(",\"failed_return\":%d,\"failed_errno\":%d,\"cleanup_failed_step\":",r->failed_return,r->failed_errno);
 if(r->cleanup_failed_step)json_string(r->cleanup_failed_step);else printf("null");
 printf(",\"cleanup_return\":%d,\"cleanup_errno\":%d,\"completed_setup_steps\":%u,\"initial_trace_owner\":%d,\"initial_trace_nolog\":%d,\"initial_trace_flags\":%u,\"final_trace_owner\":%d,\"readback\":",r->cleanup_return,r->cleanup_errno,r->completed_setup_steps,r->initial_owner,r->initial_nolog,r->initial_flags,r->final_owner);
 if(r->readback_available){putchar('[');for(size_t i=0;i<e->register_count;i++){if(i)putchar(',');printf("\"0x%llx\"",(unsigned long long)r->readback[i]);}putchar(']');}else printf("null");
 printf(",\"configuration_readback_verified\":%s,\"cycles\":",r->config_verified?"true":"false");if(valid)printf("%llu",(unsigned long long)sums[cycle_index]);else printf("null");
 printf(",\"instructions\":");if(valid)printf("%llu",(unsigned long long)sums[instruction_index]);else printf("null");
 printf(",\"config\":[");for(size_t i=0;i<e->register_count;i++){if(i)putchar(',');printf("\"0x%llx\"",(unsigned long long)e->config[i]);}
 printf("],\"encoding_crosswalk_applied\":true,\"threads\":[");
 for(size_t i=0;i<r->aggregate.thread_count;i++){
  const Thread *t=&r->aggregate.threads[i];if(i)putchar(',');
  printf("{\"tid\":%llu,\"samples\":%llu,\"observed_span_ns\":%llu}",(unsigned long long)t->tid,(unsigned long long)t->samples,(unsigned long long)a->kperf_ticks_to_ns(t->last_tick-t->first_tick));
 }
 printf("],\"records\":%llu,\"samples\":%llu,\"cleanup_ok\":%s}\n",(unsigned long long)r->aggregate.records,(unsigned long long)r->aggregate.samples,r->cleanup_ok?"true":"false");
}
