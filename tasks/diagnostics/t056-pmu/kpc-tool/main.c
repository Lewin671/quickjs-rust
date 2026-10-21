/* Privileged parent only loads fixed system APIs and counts system-wide around its gated child. */
#include "tool.h"
#include <errno.h>
#include <limits.h>
#include <pwd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>
extern char **environ;
int main(int argc,char **argv) {
 uint32_t uid=0,gid=0;
 bool identity=parse_identity(getenv("SUDO_UID"),&uid)&&parse_identity(getenv("SUDO_GID"),&gid);
 char *empty_env[]={NULL};environ=empty_env;
 if(!close_extra())return fail("cannot close inherited descriptors");umask(077);
 struct rlimit core={0,0};if(setrlimit(RLIMIT_CORE,&core))return fail("cannot disable core files");
 if(chdir("/"))return fail("cannot set fixed working directory");
 if(argc==2&&!strcmp(argv[1],"--help")){
  puts("qjs-kpc: Intel Comet Lake system-wide user counter diagnostic, with matched idle control\nUsage: qjs-kpc --events cycles,instructions,EVENT,... [--env TZ=UTC] [--env LC_ALL=C] [--timeout-ms 1..10000] -- /absolute/program args...\n       qjs-kpc --describe --events cycles,instructions,EVENT,...\n       qjs-kpc --list-events\nCycles/instructions use two of four KPEP user-only programmable counters. Counts include background work.\nChild runs as nonroot SUDO_UID/GID, no supplementary groups, cwd=/, fixed TZ=UTC LC_ALL=C.");return 0;
 }
 bool list=argc==2&&!strcmp(argv[1],"--list-events"),describe=false;
 const char *event_list=NULL;unsigned timeout=10000;int command=-1;
 if(!list)for(int i=1;i<argc;i++){
  if(!strcmp(argv[i],"--describe")&&!describe){describe=true;continue;}
  if(!strcmp(argv[i],"--events")&&!event_list&&i+1<argc){event_list=argv[++i];continue;}
  if(!strcmp(argv[i],"--env")&&i+1<argc){const char *value=argv[++i];if(strcmp(value,"TZ=UTC")&&strcmp(value,"LC_ALL=C"))return fail("only TZ=UTC and LC_ALL=C are permitted");continue;}
  if(!strcmp(argv[i],"--timeout-ms")&&i+1<argc){uint32_t n=0;if(!parse_identity(argv[++i],&n)||n>10000)return fail("timeout outside1..10000ms");timeout=n;continue;}
  if(!strcmp(argv[i],"--")){command=i+1;break;}
  return fail("invalid or duplicate option");
 }
 if(!list&&(!event_list||(!describe&&(command<0||command>=argc))||(describe&&command>=0)))return fail("invalid events/command mode");
 if(!list&&!describe){
  if(getuid()!=0||geteuid()!=0||!identity)return fail("root via sudo with nonzero SUDO_UID/GID required");
  struct passwd *user=getpwuid((uid_t)uid);
  if(!user||user->pw_gid!=(gid_t)gid)return fail("SUDO identity must match invoking account primary group");
  const char *dirs[]={"/","/usr","/usr/local","/usr/local/libexec"};
  for(size_t i=0;i<sizeof(dirs)/sizeof(dirs[0]);i++)if(!secure_path(dirs[i],true))return fail("install directory must be root-owned and not writable by others");
  if(!secure_path("/usr/local/libexec/qjs-kpc",false))return fail("installed helper must be a root-owned regular file");
  if(argv[command][0]!='/')return fail("child program must use an absolute path");
 }
 Api api={0};Events events={0};
 if(!api_load(&api))return fail("required system private-framework symbols unavailable");
 if(list)return events_list(&api)?0:fail("event database unavailable or unsupported CPU");
 if(!events_prepare(&api,&events,event_list))return fail("unsupported CPU/event/group; requires cycles,instructions and at most2 extra events");
 if(describe){events_describe(&events);return 0;}
 Run *run=calloc(1,sizeof(*run));if(!run)return fail("allocation failed");
 bool complete=run_child(&api,&events,run,argv+command,(uid_t)uid,(gid_t)gid,timeout);
 run_json(&api,&events,run,complete);
 int result=complete&&WIFEXITED(run->status)&&WEXITSTATUS(run->status)==0?0:1;
 free(run);api.kpep_config_free(events.cfg);api.kpep_db_free(events.db);return result;
}
