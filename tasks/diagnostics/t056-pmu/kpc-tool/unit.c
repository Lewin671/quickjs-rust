/* Runs unprivileged; synthetic records are parser tests, never PMU evidence. */
#include "tool.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/resource.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
static void sample(Aggregate *a,uint64_t tid,uint64_t tick,uint64_t base) {
 TraceRow r={0};r.timestamp=tick;r.arg[4]=tid;r.debugid=0x25060021;
 for(unsigned i=0;i<4;i++)r.arg[i]=base+i;
 aggregate_row(a,&r);r.timestamp++;r.debugid=0x25060022;
 for(unsigned i=0;i<3;i++)r.arg[i]=base+4+i;
 aggregate_row(a,&r);
}
int main(void) {
 uint32_t uid=0;assert(parse_identity("501",&uid)&&uid==501);
 const char *bad[]={NULL,"","0","-1","+501","501x","4294967295","999999999999999999999"};
 for(size_t i=0;i<sizeof(bad)/sizeof(bad[0]);i++)assert(!parse_identity(bad[i],&uid));
 Aggregate a={0};a.counter_count=7;uint64_t sums[MAX_COUNTERS];
 sample(&a,UINT64_C(0x100000001),100,10);sample(&a,1,200,20);
 assert(!aggregate_total(&a,sums)); /* Single-sample threads cannot disappear. */
 sample(&a,UINT64_C(0x100000001),300,110);sample(&a,1,400,220);
 assert(aggregate_total(&a,sums)&&a.thread_count==2&&a.samples==4);
 for(unsigned i=0;i<7;i++)assert(sums[i]==300);
 Aggregate decreased=a;sample(&decreased,1,500,1);assert(!aggregate_total(&decreased,sums));
 TraceRow missing={0};missing.timestamp=600;missing.arg[4]=1;missing.debugid=0x25060021;
 aggregate_row(&a,&missing);assert(!aggregate_total(&a,sums));
 Aggregate malformed={0};malformed.counter_count=7;missing.debugid=0x25060022;aggregate_row(&malformed,&missing);assert(malformed.invalid);
 Aggregate one={0};one.counter_count=3;sample(&one,2,100,1); /* Extra continuation is invalid at3 counters. */
 assert(one.invalid);
 int fd=open("/dev/null",O_RDONLY);assert(fd>=0);int high=fcntl(fd,F_DUPFD,200);assert(high>=200);
 struct rlimit old,small;assert(getrlimit(RLIMIT_NOFILE,&old)==0);small=old;small.rlim_cur=128;assert(setrlimit(RLIMIT_NOFILE,&small)==0);
 assert(close_extra());errno=0;assert(fcntl(high,F_GETFD)==-1&&errno==EBADF);assert(setrlimit(RLIMIT_NOFILE,&old)==0);
 puts("unit: identity bounds,64-bit TIDs, interleaved/split records, missing/falling counters: pass");return 0;
}
