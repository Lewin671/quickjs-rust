"""Terminal snapshot launch; never interpret its counters or duration."""
import os,subprocess,struct,hashlib

def take(active,argv,env):
 r=subprocess.run(['/usr/bin/time','-l',str(active),*argv],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
 if r.returncode!=0 or r.stdout or len(r.stderr)<56:raise ValueError(('snapshot failed',r.returncode,r.stderr[:1000]))
 keys=('ops_count','ops_base','stack_slot_value','program','bank','rbp','rsp')
 result=dict(zip(keys,struct.unpack('<7Q',r.stderr[:56])));result['stack_slot']=result['rbp']-48
 result.update(argv_bytes=sum(len(os.fsencode(x))+1 for x in [str(active),*argv]),env_bytes=sum(len(os.fsencode(k+'='+v))+1 for k,v in env.items()),env_count=len(env),env_entry_bytes={k:len(os.fsencode(k+'='+v))+1 for k,v in env.items()},env_order_sha256=hashlib.sha256(b'\0'.join(os.fsencode(k+'='+v) for k,v in env.items())).hexdigest())
 return result
