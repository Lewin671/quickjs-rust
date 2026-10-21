/* One stdout JSON record: system-wide raw snapshots, deltas and idle controls. */
#include "tool.h"
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
static void json_bytes(const char *s,size_t length) {
 putchar('"');for(size_t i=0;i<length;i++){const unsigned char *p=(const unsigned char *)s+i;
  if(*p=='"'||*p=='\\'){putchar('\\');putchar(*p);}
  else if(*p<32||*p>=127)printf("\\u%04x",(unsigned)*p);else putchar(*p);
 }putchar('"');
}
void json_string(const char *s){json_bytes(s,strlen(s));}
int fail(const char *message){printf("{\"version\":\"" TOOL_VERSION "\",\"error\":");json_string(message);puts(",\"capture_complete\":false,\"counters_valid\":false}");return 1;}
static void number(uint64_t n,bool valid){if(valid)printf("%llu",(unsigned long long)n);else printf("null");}
static void config(const uint64_t *v,size_t n){putchar('[');for(size_t i=0;i<n;i++){if(i)putchar(',');printf("\"0x%llx\"",(unsigned long long)v[i]);}putchar(']');}
static void snapshot(const Snapshot *s,const Events *e,unsigned cpus){
 if(!s->valid){printf("null");return;}
 printf("{\"begin_ns\":%llu,\"end_ns\":%llu,\"current_cpu\":%d,\"per_cpu\":[",(unsigned long long)s->begin_ns,(unsigned long long)s->end_ns,s->current_cpu);
 for(unsigned cpu=0;cpu<cpus;cpu++){if(cpu)putchar(',');putchar('[');for(unsigned i=0;i<e->counter_count;i++){if(i)putchar(',');number(s->values[(size_t)cpu*e->counter_count+i],true);}putchar(']');}printf("]}");
}
void run_json(const Api *a,const Events *e,const Run *r,bool complete){
 (void)a;bool valid=complete&&r->counts_valid&&r->cleanup_ok;int raw=r->status;
 int status=WIFEXITED(raw)?WEXITSTATUS(raw):WIFSIGNALED(raw)?128+WTERMSIG(raw):255;
 printf("{\"counter_backend\":\"kpep_programmable_user\",\"version\":\"" TOOL_VERSION "\",\"scope\":\"system_wide_user_cpu_delta\",\"pid\":%d,\"capture_complete\":%s,\"counters_valid\":%s,\"child_status\":%d,\"timed_out\":%s,\"elapsed_ns\":%llu,\"child_window_ns\":%llu,\"idle_window_ns\":%llu,\"cpu_count\":%u,\"counter_count\":%u,\"initial_counting_mask\":%u,",r->pid,complete?"true":"false",valid?"true":"false",status,r->timed_out?"true":"false",(unsigned long long)r->elapsed_ns,(unsigned long long)r->child_window_ns,(unsigned long long)r->idle_window_ns,r->cpu_count,e->counter_count,r->old_count);
 printf("\"user_only_requested\":true,\"pid_filtered\":false,\"full_lifetime_counts\":false,\"child_stdout\":");json_bytes(r->output,r->out_len);printf(",\"child_stderr\":");json_bytes(r->error,r->err_len);
 size_t cycles=0,instructions=0;printf(",\"events\":[");
 for(size_t i=0;i<e->event_count;i++){
  size_t index=e->map[i];if(!strcmp(e->names[i],"CPU_CLK_UNHALTED.THREAD"))cycles=index;if(!strcmp(e->names[i],"INST_RETIRED.ANY"))instructions=index;
  if(i)putchar(',');printf("{\"name\":");json_string(e->names[i]);printf(",\"index\":%zu,\"count\":",index);number(r->sums[index],valid);printf(",\"idle_count\":");number(r->idle_sums[index],valid);
  printf(",\"per_cpu\":[");for(unsigned cpu=0;cpu<r->cpu_count;cpu++){if(cpu)putchar(',');size_t k=(size_t)cpu*e->counter_count+index;number(r->after.values[k]-r->before.values[k],valid);}printf("],\"idle_per_cpu\":[");
  for(unsigned cpu=0;cpu<r->cpu_count;cpu++){if(cpu)putchar(',');size_t k=(size_t)cpu*e->counter_count+index;number(r->idle_after.values[k]-r->idle_before.values[k],valid);}printf("]}");
 }
 printf("],\"cycles\":");number(r->sums[cycles],valid);printf(",\"instructions\":");number(r->sums[instructions],valid);
 printf(",\"failed_step\":");if(r->failed_step)json_string(r->failed_step);else printf("null");
 printf(",\"failed_return\":%d,\"failed_errno\":%d,\"cleanup_failed_step\":",r->failed_return,r->failed_errno);if(r->cleanup_failed_step)json_string(r->cleanup_failed_step);else printf("null");
 printf(",\"cleanup_return\":%d,\"cleanup_errno\":%d,\"completed_setup_steps\":%u,\"configuration_readback_verified\":%s,\"readback\":",r->cleanup_return,r->cleanup_errno,r->completed_setup_steps,r->config_verified?"true":"false");if(r->readback_available)config(r->readback,e->register_count);else printf("null");
 printf(",\"config\":");config(e->config,e->register_count);printf(",\"encoding_crosswalk_applied\":true,\"snapshots\":{");
 const Snapshot *shots[]={&r->before,&r->after,&r->idle_before,&r->idle_after};const char *names[]={"before","after","idle_before","idle_after"};
 for(unsigned i=0;i<4;i++){if(i)putchar(',');json_string(names[i]);putchar(':');snapshot(shots[i],e,r->cpu_count);}
 printf("},\"cleanup_ok\":%s}\n",r->cleanup_ok?"true":"false");
}
