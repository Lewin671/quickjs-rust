/* Root boundary: fixed trusted paths, strict sudo identity, irreversible child drop. */
#include "tool.h"
#include <errno.h>
#include <grp.h>
#include <libproc.h>
#include <sys/acl.h>
#include <sys/stat.h>
#include <unistd.h>

bool parse_identity(const char *s,uint32_t *out) {
 uint64_t n=0;
 if(!s||!*s)return false;
 for(;*s;s++){if(*s<'0'||*s>'9')return false;n=n*10+(unsigned)(*s-'0');if(n>=UINT32_MAX)return false;}
 if(!n)return false;
 *out=(uint32_t)n;return true;
}
bool secure_path(const char *path,bool directory) {
 struct stat st;
 if(lstat(path,&st)||st.st_uid!=0||(st.st_mode&0022))return false;
 if(directory?!S_ISDIR(st.st_mode):!S_ISREG(st.st_mode))return false;
 acl_t acl=acl_get_file(path,ACL_TYPE_EXTENDED);
 if(!acl)return errno==ENOENT||errno==ENOTSUP;
 acl_entry_t entry;bool safe=true;
 int result=acl_get_entry(acl,ACL_FIRST_ENTRY,&entry);
 while(result==0){
  acl_tag_t tag;acl_permset_t set;
  if(acl_get_tag_type(entry,&tag)||acl_get_permset(entry,&set)){safe=false;break;}
  if(tag==ACL_EXTENDED_ALLOW){
   const acl_perm_t writes[]={ACL_WRITE_DATA,ACL_APPEND_DATA,ACL_DELETE,ACL_DELETE_CHILD,ACL_WRITE_ATTRIBUTES,ACL_WRITE_EXTATTRIBUTES,ACL_WRITE_SECURITY,ACL_CHANGE_OWNER};
   for(size_t i=0;i<sizeof(writes)/sizeof(writes[0]);i++)if(acl_get_perm_np(set,writes[i])!=0)safe=false;
  }
  result=acl_get_entry(acl,ACL_NEXT_ENTRY,&entry);
 }
 if(result!=0&&errno!=EINVAL) safe=false;
 acl_free(acl);return safe;
}
bool drop_identity(uid_t uid,gid_t gid) {
 if(!uid||!gid||setgroups(0,NULL)||setgid(gid)||setuid(uid))return false;
 if(getuid()!=uid||geteuid()!=uid||getgid()!=gid||getegid()!=gid)return false;
 errno=0;
 return setuid(0)==-1&&errno==EPERM;
}

/* Enumerate actual open FDs; a caller can lower RLIMIT_NOFILE below an open FD. */
bool close_extra(void) {
 struct proc_fdinfo fds[8192];
 int n=proc_pidinfo(getpid(),PROC_PIDLISTFDS,0,fds,(int)sizeof(fds));
 if(n<=0||n>=(int)sizeof(fds)||n%(int)sizeof(fds[0]))return false;
 for(size_t i=0;i<(size_t)n/sizeof(fds[0]);i++)if(fds[i].proc_fd>=3&&close(fds[i].proc_fd)&&errno!=EBADF)return false;
 return true;
}
