/* Execute the real gate/output/timeout/cleanup loop as the ordinary test user.
   Counter and credential functions are mocks; not a root security proof. */
#include "tool.h"
#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
static int releases,reads;static bool begin_ok=true;
uint64_t monotonic_ns(void){struct timespec t;assert(!clock_gettime(CLOCK_MONOTONIC,&t));return (uint64_t)t.tv_sec*UINT64_C(1000000000)+(uint64_t)t.tv_nsec;}
bool drop_identity(uid_t uid,gid_t gid){return uid==getuid()&&gid==getgid()&&uid!=0;}
bool close_extra(void){for(int i=3;i<1024;i++)close(i);return true;}
bool count_begin(Api *a,const Events *e,Run *r){(void)a;(void)e;r->forced=begin_ok;return begin_ok;}
bool count_read(Api *a,const Events *e,Run *r,Snapshot *s){(void)a;(void)e;(void)r;s->begin_ns=s->end_ns=monotonic_ns();s->valid=true;reads++;return true;}
bool count_delta(const Events *e,unsigned cpus,const Snapshot *a,const Snapshot *b,uint64_t *v){(void)e;(void)cpus;(void)v;return a->valid&&b->valid;}
bool count_finish(Api *a,const Events *e,Run *r){(void)a;(void)e;r->forced=false;releases++;return true;}
bool count_result(Run *r,const char *step,int value,int error,bool cleanup){(void)error;(void)cleanup;if(value)r->failed_step=step;return !value;}
int main(int argc,char **argv){
 assert(argc==2&&getuid()!=0);Api a={0};Events e={0};Run *r=calloc(1,sizeof(*r));assert(r);
 char *identity[]={"/usr/bin/id","-u",NULL},*env[]={"/usr/bin/env",NULL},*sleeping[]={"/bin/sleep","1",NULL},*overflow[]={"/usr/bin/yes",NULL};
 if(!strcmp(argv[1],"success")){assert(run_child(&a,&e,r,identity,getuid(),getgid(),1000));assert(reads==4&&r->out_len&&r->counts_valid&&r->idle_window_ns>=r->child_window_ns);}
 else if(!strcmp(argv[1],"env")){assert(run_child(&a,&e,r,env,getuid(),getgid(),1000));assert(!strcmp(r->output,"TZ=UTC\nLC_ALL=C\n"));}
 else if(!strcmp(argv[1],"timeout")){assert(!run_child(&a,&e,r,sleeping,getuid(),getgid(),10));assert(r->timed_out&&reads==1);}
 else if(!strcmp(argv[1],"overflow")){assert(!run_child(&a,&e,r,overflow,getuid(),getgid(),1000));assert(reads==1);}
 else {begin_ok=false;assert(!run_child(&a,&e,r,identity,getuid(),getgid(),1000));assert(!reads&&!r->out_len);}
 assert(releases==1&&!r->forced&&r->cleanup_ok);free(r);puts("process-unit: real unprivileged gate/process cleanup: pass");return 0;
}
