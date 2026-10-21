/* Only the forked child execs user input, after dropping every root credential. */
#include "tool.h"
#include <errno.h>
#include <fcntl.h>
#include <mach/mach_time.h>
#include <signal.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>
static volatile sig_atomic_t stopping,alarm_seen;
static volatile sig_atomic_t child_pid;
static void stop(int signo) {
 stopping=1;
 if(signo==SIGALRM){if(alarm_seen)_exit(124);alarm_seen=1;if(child_pid>0)(void)kill((pid_t)child_pid,SIGKILL);alarm(2);}
}
static uint64_t now(void) {
 struct timespec t;if(clock_gettime(CLOCK_MONOTONIC,&t))return 0;
 return (uint64_t)t.tv_sec*UINT64_C(1000000000)+(uint64_t)t.tv_nsec;
}
static bool drain(int fd,char *text,size_t *used) {
 char buf[4096];
 for(;;){
  ssize_t n=read(fd,buf,sizeof(buf));
  if(n>0){if((size_t)n>TEXT_LIMIT-*used)return false;memcpy(text+*used,buf,(size_t)n);*used+=(size_t)n;text[*used]=0;}
  else if(n==0||errno==EAGAIN||errno==EWOULDBLOCK)return true;
  else if(errno!=EINTR)return false;
 }
}
static void close_pipe(int p[2]) {if(p[0]>=0)close(p[0]);if(p[1]>=0)close(p[1]);}
bool run_child(Api *a,const Events *e,Run *r,char *const *argv,uid_t uid,gid_t gid,unsigned timeout_ms) {
 int output[2]={-1,-1},error[2]={-1,-1},gate[2]={-1,-1};bool ok=false,exited=false;
 uint64_t command_begin=0;uint64_t begin=now();if(!begin)return false;
 struct sigaction action={0};action.sa_handler=stop;sigemptyset(&action.sa_mask);
 if(sigaction(SIGTERM,&action,NULL)||sigaction(SIGINT,&action,NULL)||sigaction(SIGHUP,&action,NULL)||sigaction(SIGALRM,&action,NULL))return false;
 alarm((timeout_ms+999)/1000+1);
 signal(SIGPIPE,SIG_IGN);
 if(pipe(output)||pipe(error)||pipe(gate))goto done;
 r->pid=fork();if(r->pid<0)goto done;
 if(r->pid>0)child_pid=(sig_atomic_t)r->pid;
 if(r->pid==0){
  /* No allocator, dynamic loader, caller-controlled lookup or stdio after fork. */
  if(setpgid(0,0)||dup2(output[1],STDOUT_FILENO)<0||dup2(error[1],STDERR_FILENO)<0)_exit(125);
  int input=open("/dev/null",O_RDONLY);if(input<0||dup2(input,STDIN_FILENO)<0)_exit(125);
  if(input>2)close(input);
  close(gate[1]);
  if(!drop_identity(uid,gid))_exit(125);
  char go=0;if(read(gate[0],&go,1)!=1||go!='G')_exit(125);
  if(!close_extra())_exit(125);
  struct sigaction normal={0};normal.sa_handler=SIG_DFL;sigemptyset(&normal.sa_mask);
  if(sigaction(SIGTERM,&normal,NULL)||sigaction(SIGINT,&normal,NULL)||sigaction(SIGHUP,&normal,NULL)||sigaction(SIGPIPE,&normal,NULL))_exit(125);
  sigset_t empty;sigemptyset(&empty);if(sigprocmask(SIG_SETMASK,&empty,NULL))_exit(125);
  char *const env[]={"TZ=UTC","LC_ALL=C",NULL};
  execve(argv[0],argv,env);_exit(126);
 }
 close(output[1]);output[1]=-1;close(error[1]);error[1]=-1;close(gate[0]);gate[0]=-1;
 (void)setpgid(r->pid,r->pid);
 if(fcntl(output[0],F_SETFL,O_NONBLOCK)||fcntl(error[0],F_SETFL,O_NONBLOCK))goto done;
 if(!trace_begin(a,e,r)||stopping)goto done;
 command_begin=now();
 if(write(gate[1],"G",1)!=1)goto done;
 close(gate[1]);gate[1]=-1;
 for(;;){
  if(stopping)goto done;
  if(!drain(output[0],r->output,&r->out_len)||!drain(error[0],r->error,&r->err_len)||!trace_drain(r))goto done;
  pid_t result=waitpid(r->pid,&r->status,WNOHANG);
  if(result==r->pid){exited=true;break;}
  if(result<0&&errno!=EINTR)goto done;
  uint64_t elapsed=now()-begin;
  if(elapsed>(uint64_t)timeout_ms*UINT64_C(1000000)){r->timed_out=true;goto done;}
  struct timespec pause={0,1000000};nanosleep(&pause,NULL);
 }
 /* Drain records already queued before exit. Never claim a final-lifetime sample. */
 if(!trace_drain(r)||!drain(output[0],r->output,&r->out_len)||!drain(error[0],r->error,&r->err_len))goto done;
 ok=true;
 done:
 if(command_begin)r->child_window_ns=now()-command_begin;
 if(!ok&&!r->failed_step)(void)trace_result(r,"process.before_or_after_gate",-1,errno,false);
 if(r->pid>0&&!exited){
  (void)kill(-r->pid,SIGKILL);(void)kill(r->pid,SIGKILL);
  while(waitpid(r->pid,&r->status,0)<0&&errno==EINTR){}
 }
 close_pipe(output);close_pipe(error);close_pipe(gate);
 r->cleanup_ok=trace_finish(a,e,r);if(!r->cleanup_ok)ok=false;
 alarm(0);child_pid=0;r->elapsed_ns=now()-begin;
 return ok;
}
