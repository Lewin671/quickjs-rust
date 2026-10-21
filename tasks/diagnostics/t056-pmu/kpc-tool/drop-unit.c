/* Mock credential syscalls ONLY in this separately linked, never-installed test. */
#include "tool.h"
#include <assert.h>
#include <errno.h>
#include <grp.h>
#include <stdio.h>
#include <unistd.h>
static unsigned calls,fail_at;static uid_t current_uid;static gid_t current_gid;
int test_setgroups(int n,const gid_t *groups) {assert(++calls==1&&n==0&&groups==NULL);return fail_at==1?-1:0;}
int test_setgid(gid_t gid) {assert(++calls==2&&gid==20);if(fail_at==2)return -1;current_gid=gid;return 0;}
int test_setuid(uid_t uid) {
 if(uid==0){assert(++calls==4);errno=EPERM;return fail_at==4?0:-1;}
 assert(++calls==3&&uid==501);if(fail_at==3)return -1;current_uid=uid;return 0;
}
uid_t test_getuid(void){return current_uid;}uid_t test_geteuid(void){return current_uid;}
gid_t test_getgid(void){return current_gid;}gid_t test_getegid(void){return current_gid;}
int main(void) {
 for(fail_at=0;fail_at<=4;fail_at++){
  calls=0;current_uid=0;current_gid=0;
  bool ok=drop_identity(501,20);assert(ok==(fail_at==0));
  assert(calls==(fail_at?fail_at:4));
 }
 calls=0;assert(!drop_identity(0,20)&&!drop_identity(501,0)&&calls==0);
 puts("drop-unit: group clear -> gid -> uid -> no-root-regain, every failure rejects: pass");return 0;
}
