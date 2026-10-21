/* Diagnostic-only Intel Monterey counter tool. No engine ABI or dependencies. */
#ifndef QJS_KPC_TOOL_H
#define QJS_KPC_TOOL_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>
#define MAX_EVENTS 6
#define MAX_COUNTERS 32
#define MAX_THREADS 256
#define TEXT_LIMIT 32768
#define SAMPLE_NS UINT64_C(1000000)
#define TOOL_VERSION "qjs-kpc-2"
/* Private kperfdata objects are opaque; exported accessors define the ABI used. */
typedef struct kpep_db kpep_db;
typedef struct kpep_event kpep_event;
typedef struct kpep_config kpep_config;
typedef struct {
 uint32_t (*kpc_get_counting)(void),(*kpc_get_thread_counting)(void);
 uint32_t (*kpc_get_counter_count)(uint32_t),(*kpc_get_config_count)(uint32_t);
 int (*kpc_force_all_ctrs_get)(int *),(*kpc_force_all_ctrs_set)(int);
 int (*kpc_get_config)(uint32_t,uint64_t *),(*kpc_set_config)(uint32_t,uint64_t *);
 int (*kpc_set_counting)(uint32_t),(*kpc_set_thread_counting)(uint32_t);
 int (*kperf_sample_get)(uint32_t *),(*kperf_sample_set)(uint32_t);
 int (*kperf_reset)(void);
 int (*kperf_action_count_get)(uint32_t *),(*kperf_action_count_set)(uint32_t);
 int (*kperf_timer_count_get)(uint32_t *),(*kperf_timer_count_set)(uint32_t);
 int (*kperf_action_samplers_set)(uint32_t,uint32_t),(*kperf_action_samplers_get)(uint32_t,uint32_t *);
 int (*kperf_action_filter_set_by_pid)(uint32_t,int32_t);
 int (*kperf_timer_period_set)(uint32_t,uint64_t),(*kperf_timer_period_get)(uint32_t,uint64_t *);
 int (*kperf_timer_action_set)(uint32_t,uint32_t),(*kperf_timer_pet_set)(uint32_t);
 uint64_t (*kperf_ns_to_ticks)(uint64_t),(*kperf_ticks_to_ns)(uint64_t);
 int (*kpep_db_create)(const char *,kpep_db **);
 void (*kpep_db_free)(kpep_db *),(*kpep_config_free)(kpep_config *);
 int (*kpep_db_name)(kpep_db *,const char **);
 int (*kpep_db_events_count)(kpep_db *,size_t *);
 int (*kpep_db_events)(kpep_db *,kpep_event **,size_t);
 int (*kpep_db_event)(kpep_db *,const char *,kpep_event **);
 int (*kpep_event_name)(kpep_event *,const char **);
 int (*kpep_config_create)(kpep_db *,kpep_config **);
 int (*kpep_config_force_counters)(kpep_config *);
 int (*kpep_config_add_event)(kpep_config *,kpep_event **,uint32_t,uint32_t *);
 int (*kpep_config_kpc_classes)(kpep_config *,uint32_t *);
 int (*kpep_config_kpc_count)(kpep_config *,size_t *);
 int (*kpep_config_kpc_map)(kpep_config *,size_t *,size_t);
 int (*kpep_config_kpc)(kpep_config *,uint64_t *,size_t);
} Api;
typedef struct {
 char requested[MAX_EVENTS][96];const char *names[MAX_EVENTS];size_t map[MAX_EVENTS];
 size_t event_count,register_count;uint32_t classes,counter_count;
 uint64_t config[MAX_COUNTERS];bool fixed_diagnostic;kpep_db *db;kpep_config *cfg;
} Events;
/* LP64 XNU8020 kd_buf ABI. Do not truncate 64-bit thread IDs. */
typedef struct {uint64_t timestamp,arg[5];uint32_t debugid,cpuid;uint64_t unused;} TraceRow;
typedef struct {
 uint64_t tid,first_tick,last_tick,samples,first[MAX_COUNTERS],last[MAX_COUNTERS];
 uint64_t pending[MAX_COUNTERS],pending_tick;unsigned pending_count;
} Thread;
typedef struct {
 Thread threads[MAX_THREADS];size_t thread_count;uint32_t counter_count;
 uint64_t records,samples;bool invalid;
} Aggregate;
typedef struct {
 pid_t pid;int status;bool timed_out;uint64_t elapsed_ns,child_window_ns;
 char output[TEXT_LIMIT+1],error[TEXT_LIMIT+1];size_t out_len,err_len;
 Aggregate aggregate;bool trace_owned,forced,changed,cleanup_ok,config_verified,perf_touched;
 const char *failed_step,*cleanup_failed_step;int failed_return,failed_errno,cleanup_return,cleanup_errno;
 uint64_t readback[MAX_COUNTERS];bool readback_available;
 int initial_owner,initial_nolog;unsigned initial_flags;int final_owner;
 unsigned completed_setup_steps;uint32_t old_count,old_thread;
 uint64_t old_config[MAX_COUNTERS];
} Run;
void json_string(const char *);
int fail(const char *);
bool secure_path(const char *,bool);
bool close_extra(void);
bool parse_identity(const char *,uint32_t *);
bool drop_identity(uid_t,gid_t);
bool api_load(Api *);
bool events_prepare(Api *,Events *,const char *);
bool events_list(Api *);
void events_describe(const Events *);
void aggregate_row(Aggregate *,const TraceRow *);
bool aggregate_total(const Aggregate *,uint64_t *);
bool trace_idle(Api *,Run *);
bool trace_result(Run *,const char *,int,int,bool);
bool trace_begin(Api *,const Events *,Run *);
bool trace_drain(Run *);
bool trace_finish(Api *,const Events *,Run *);
bool run_child(Api *,const Events *,Run *,char *const *,uid_t,gid_t,unsigned);
void run_json(const Api *,const Events *,const Run *,bool);
#endif
