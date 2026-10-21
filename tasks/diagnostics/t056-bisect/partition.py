"""Exhaustive source-hunk partition of f84..dfff; never edits the checkout."""
import hashlib,re,subprocess
from pathlib import Path
OLD='8f7d97c71b34e2bec52e2c6df87f4ba4ac51641a'
PARENT='f84fec3c'
NEW='dfff3bb6bf3db01fb52a7612ed1144900b5f08c5'
GROUPS=('a','b','c','d')
ROLES={**{'plus_'+g:[g] for g in GROUPS},**{'minus_'+g:[x for x in GROUPS if x!=g] for g in GROUPS}}
ORDER='crates/qjs-cli/hot-functions-x86_64.order'
PREFIX='crates/qjs-runtime/src/'
def git(*args):return subprocess.check_output(['git',*args])
def digest(value):return hashlib.sha256(value).hexdigest()
def blob(rev,path):
 q=subprocess.run(['git','show',rev+':'+path],capture_output=True)
 if q.returncode:
  assert path==PREFIX+'value/object/flags.rs' and rev in (OLD,PARENT)
  return b''
 return q.stdout

def classify(path,body):
 if path==ORDER:return 'd'
 if path==PREFIX+'bytecode/typed_loop/execute.rs':return 'c'
 if path.startswith(PREFIX+'property/layout') or path==PREFIX+'value/object/storage.rs':return 'b'
 if path==PREFIX+'value/object.rs':
  changed=[x[1:] for x in body if x.startswith(('+','-'))]
  return 'b' if any('lookup_for_write' in x for x in changed) else 'a'
 if path in [PREFIX+'value/object/'+x for x in ('class_fields.rs','flags.rs','tests.rs')]:return 'a'
 raise ValueError('unclassified build input '+path)

def inventory():
 assert not git('diff',OLD,PARENT,'--','crates','tools','scripts','.cargo','Cargo.toml','Cargo.lock')
 changed=git('diff','--name-only',PARENT,NEW).decode().splitlines();records=[]
 for path in changed:
  binary=path.startswith(('crates/','.cargo/')) or path in ('Cargo.toml','Cargo.lock','rust-toolchain.toml')
  if not binary:
   records.append({'path':path,'release_input':False,'reason':'outside canonical Rust recipe source inputs; docs or diagnostic scripts','hunks':[]});continue
  raw=git('diff','--no-ext-diff','--unified=3',PARENT,NEW,'--',path).decode();lines=raw.splitlines(keepends=True);hunks=[]
  for line in lines:
   match=re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@',line)
   if match:
    hunks.append({'header':line.rstrip(),'old_start':int(match[1]),'old_count':int(match[2] or 1),'new_start':int(match[3]),'new_count':int(match[4] or 1),'lines':[]})
   elif hunks and line[:1] in (' ','+','-'):hunks[-1]['lines'].append(line)
  for i,h in enumerate(hunks):
   h.update(id=path+':'+str(i+1),group=classify(path,h['lines']),patch_sha256=digest(''.join(h['lines']).encode()))
  assert hunks
  records.append({'path':path,'release_input':True,'test_only':path.endswith('/tests.rs'),'old_sha256':digest(blob(OLD,path)),'new_sha256':digest(blob(NEW,path)),'hunks':hunks})
 return records

def materialize(record,groups):
 path=record['path'];original=blob(OLD,path).decode().splitlines(keepends=True);result=[];cursor=0
 for h in record['hunks']:
  start=max(0,h['old_start']-1);old=[x[1:] for x in h['lines'] if x[0] in (' ','-')];new=[x[1:] for x in h['lines'] if x[0] in (' ','+')]
  assert len(old)==h['old_count'] and len(new)==h['new_count'] and original[start:start+len(old)]==old,(path,h['header'])
  assert start>=cursor
  result.extend(original[cursor:start]);result.extend(new if h['group'] in groups else old);cursor=start+len(old)
 result.extend(original[cursor:]);return ''.join(result).encode()

def verify(records):
 for r in records:
  if r['release_input']:
   assert materialize(r,[])==blob(OLD,r['path'])
   assert materialize(r,GROUPS)==blob(NEW,r['path'])
 return {g:[h['id'] for r in records for h in r['hunks'] if h['group']==g] for g in GROUPS}
