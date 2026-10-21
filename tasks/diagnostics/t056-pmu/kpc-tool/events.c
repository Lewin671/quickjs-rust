/* Resolve names from the fixed system CPU database. No raw-register CLI input. */
#include "tool.h"
#include "events-approved.h"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/sysctl.h>

#define LOAD(handle,name) do {void *symbol=dlsym(handle,#name);if(!symbol)return false;memcpy(&a->name,&symbol,sizeof(symbol));} while(0)
bool api_load(Api *a) {
 void *k=dlopen("/System/Library/PrivateFrameworks/kperf.framework/kperf",RTLD_NOW|RTLD_LOCAL);
 void *d=dlopen("/System/Library/PrivateFrameworks/kperfdata.framework/kperfdata",RTLD_NOW|RTLD_LOCAL);
 if(!k||!d)return false;
 LOAD(k,kpc_get_counting);LOAD(k,kpc_get_thread_counting);LOAD(k,kpc_get_counter_count);LOAD(k,kpc_get_config_count);
 LOAD(k,kpc_force_all_ctrs_get);LOAD(k,kpc_force_all_ctrs_set);LOAD(k,kpc_get_config);LOAD(k,kpc_set_config);
 LOAD(k,kpc_set_counting);LOAD(k,kpc_set_thread_counting);LOAD(k,kperf_sample_get);LOAD(k,kperf_reset);LOAD(k,kperf_sample_set);
 LOAD(k,kperf_action_count_get);LOAD(k,kperf_action_count_set);LOAD(k,kperf_timer_count_get);LOAD(k,kperf_timer_count_set);
 LOAD(k,kperf_action_samplers_set);LOAD(k,kperf_action_samplers_get);LOAD(k,kperf_action_filter_set_by_pid);LOAD(k,kperf_timer_period_set);LOAD(k,kperf_timer_period_get);
 LOAD(k,kperf_timer_action_set);LOAD(k,kperf_timer_pet_set);LOAD(k,kperf_ns_to_ticks);LOAD(k,kperf_ticks_to_ns);
 LOAD(d,kpep_db_create);LOAD(d,kpep_db_free);LOAD(d,kpep_db_name);LOAD(d,kpep_db_events_count);LOAD(d,kpep_db_events);
 LOAD(d,kpep_db_event);LOAD(d,kpep_event_name);LOAD(d,kpep_config_create);LOAD(d,kpep_config_free);
 LOAD(d,kpep_config_force_counters);LOAD(d,kpep_config_add_event);LOAD(d,kpep_config_kpc_classes);
 LOAD(d,kpep_config_kpc_count);LOAD(d,kpep_config_kpc_map);LOAD(d,kpep_config_kpc);
 return true;
}
static bool database(Api *a,kpep_db **db) {
 int family=0,model=0,step=0;size_t n=sizeof(int);
 if(sysctlbyname("machdep.cpu.family",&family,&n,NULL,0)||family!=6)return false;
 n=sizeof(int);if(sysctlbyname("machdep.cpu.model",&model,&n,NULL,0)||model!=165)return false;
 n=sizeof(int);if(sysctlbyname("machdep.cpu.stepping",&step,&n,NULL,0)||step!=5)return false;
 if(!secure_path("/usr/share/kpep/cometlake.plist",false)||a->kpep_db_create("cometlake",db))return false;
 const char *name=NULL;
 return !a->kpep_db_name(*db,&name)&&name&&(!strcmp(name,"cometlake")||!strcmp(name,"Intel Comet Lake"));
}
bool events_prepare(Api *a,Events *e,const char *list) {
 if(!list||!*list||strlen(list)>MAX_EVENTS*96)return false;
 char text[MAX_EVENTS*96+1];strcpy(text,list);char *cursor=text;
 while(cursor){
  char *next=strchr(cursor,',');if(next)*next++=0;
  size_t n=strlen(cursor);if(!n||n>=96||e->event_count==MAX_EVENTS)return false;
  for(size_t i=0;i<n;i++)if(!((cursor[i]>='A'&&cursor[i]<='Z')||(cursor[i]>='a'&&cursor[i]<='z')||(cursor[i]>='0'&&cursor[i]<='9')||cursor[i]=='.'||cursor[i]=='_'))return false;
  strcpy(e->requested[e->event_count++],cursor);cursor=next;
 }
 if(!database(a,&e->db)||a->kpep_config_create(e->db,&e->cfg)||a->kpep_config_force_counters(e->cfg))return false;
 bool cycles=false,instructions=false;
 for(size_t i=0;i<e->event_count;i++){
  const char *name=e->requested[i];
  if(!strcmp(name,"cycles"))name="CPU_CLK_UNHALTED.THREAD";
  if(!strcmp(name,"instructions"))name="INST_RETIRED.ANY";
  cycles|=!strcmp(name,"CPU_CLK_UNHALTED.THREAD");instructions|=!strcmp(name,"INST_RETIRED.ANY");
  kpep_event *event=NULL;
  if(a->kpep_db_event(e->db,name,&event)||!event||a->kpep_event_name(event,&e->names[i]))return false;
  for(size_t j=0;j<i;j++)if(!strcmp(e->names[i],e->names[j]))return false;
  if(e->fixed_diagnostic&&!strcmp(name,"CPU_CLK_UNHALTED.THREAD")){e->map[i]=1;continue;}
  if(e->fixed_diagnostic&&!strcmp(name,"INST_RETIRED.ANY")){e->map[i]=0;continue;}
  bool allowed=!strcmp(name,"CPU_CLK_UNHALTED.THREAD")||!strcmp(name,"INST_RETIRED.ANY");
  for(size_t j=0;j<sizeof(approved)/sizeof(approved[0]);j++)if(!strcmp(name,approved[j].name))allowed=true;
  if(!allowed||a->kpep_config_add_event(e->cfg,&event,1,NULL))return false;
 }
 if(!cycles||!instructions)return false;
 uint32_t programmable_classes=0;size_t programmable_count=0;
 if(a->kpep_config_kpc_classes(e->cfg,&programmable_classes)||(programmable_classes&~2u))return false;
 if(a->kpep_config_kpc_count(e->cfg,&programmable_count)||programmable_count>4)return false;
 size_t mapping[MAX_COUNTERS]={0};uint64_t config[MAX_COUNTERS]={0};
 if(programmable_count&&(a->kpep_config_kpc_map(e->cfg,mapping,sizeof(mapping))||a->kpep_config_kpc(e->cfg,config,sizeof(config))))return false;
 size_t prefix=e->fixed_diagnostic?1:0;size_t counter_prefix=e->fixed_diagnostic?3:0;
 e->classes=(e->fixed_diagnostic?1u:0u)|programmable_classes;e->register_count=prefix+programmable_count;
 if(!e->classes||a->kpc_get_config_count(e->classes)!=e->register_count)return false;
 if(e->fixed_diagnostic)e->config[0]=0x22;
 for(size_t i=0;i<programmable_count;i++)e->config[i+prefix]=config[i];
 size_t extra=0;
 for(size_t i=0;i<e->event_count;i++){
  bool cycle=!strcmp(e->names[i],"CPU_CLK_UNHALTED.THREAD"),instruction=!strcmp(e->names[i],"INST_RETIRED.ANY");
  if(e->fixed_diagnostic&&(cycle||instruction))continue;
  size_t index=mapping[extra++];if(index>=programmable_count)return false;e->map[i]=counter_prefix+index;
  /* Named architectural *_P fallbacks selected by KPEP, made explicit here;
     no assumption that KPC fixed config is IA32_FIXED_CTR_CTRL passthrough. */
  Approved fallback={cycle?"CPU_CLK_UNHALTED.THREAD_P":"INST_RETIRED.ANY_P",cycle?0x3cu:0xc0u,0,0,0,0};
  const Approved *rule=cycle||instruction?&fallback:NULL;
  for(size_t j=0;j<sizeof(approved)/sizeof(approved[0]);j++)if(!strcmp(e->names[i],approved[j].name))rule=&approved[j];
  if(!rule||(e->config[index+prefix]&0xffffu)!=(rule->number|(rule->umask<<8)))return false;
  e->config[index+prefix]=(e->config[index+prefix]&0xffffu)|(UINT64_C(1)<<16)|(UINT64_C(1)<<22)|((uint64_t)rule->edge<<18)|((uint64_t)rule->invert<<23)|((uint64_t)rule->cmask<<24);
 }
 e->counter_count=a->kpc_get_counter_count(e->classes);
 if(!e->counter_count||e->counter_count>MAX_COUNTERS||a->kpc_get_counter_count(1)!=3||a->kpc_get_counter_count(2)!=4)return false;
 for(size_t i=0;i<e->event_count;i++)if(e->map[i]>=e->counter_count)return false;
 return true;
}
bool events_list(Api *a) {
 kpep_db *db=NULL;size_t count=0;
 if(!database(a,&db)||a->kpep_db_events_count(db,&count)||count>4096)return false;
 kpep_event **events=calloc(count,sizeof(*events));if(!events)return false;
 if(a->kpep_db_events(db,events,count*sizeof(*events)))return false;
 printf("{\"database\":\"cometlake\",\"fixed\":3,\"programmable\":4,\"events\":[");
 for(size_t i=0;i<count;i++){const char *name=NULL;if(a->kpep_event_name(events[i],&name)||!name)return false;if(i)putchar(',');json_string(name);}
 puts("]}");free(events);a->kpep_db_free(db);return true;
}
void events_describe(const Events *e) {
 printf("{\"counter_backend\":\"%s\",\"version\":\"" TOOL_VERSION "\",\"database\":\"cometlake\",\"user_only_requested\":true,\"classes\":%u,\"counter_count\":%u,\"events\":[",e->fixed_diagnostic?"fixed_diagnostic":"kpep_programmable_user",e->classes,e->counter_count);
 for(size_t i=0;i<e->event_count;i++){if(i)putchar(',');printf("{\"name\":");json_string(e->names[i]);printf(",\"index\":%zu}",e->map[i]);}
 printf("],\"config\":[");for(size_t i=0;i<e->register_count;i++){if(i)putchar(',');printf("\"0x%llx\"",(unsigned long long)e->config[i]);}puts("]}");
}
