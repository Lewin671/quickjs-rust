"""Three orthogonal header transformations on 8f archives; no checkout edits."""
import re,itertools,functools
import bisect_partition as bp
OLD=bp.OLD;NEW=bp.NEW;PREFIX=bp.PREFIX
PARTS=('count','flags','order')
ROLES={'_'.join(c):c for n in (1,2) for c in itertools.combinations(PARTS,n)}
FLAGS={'extensible':'Extensible','raw_json':'RawJson','array_prototype_exotic':'ArrayPrototype','immutable_prototype_exotic':'ImmutablePrototype','string_indices_pending':'StringIndicesPending'}
FILES=[PREFIX+x for x in ('value/object.rs','value/object/class_fields.rs','value/object/flags.rs','value/object/tests.rs')]
@functools.cache
def records():return bp.inventory()
@functools.cache
def full(path):
 r=next(r for r in records() if r['path']==path)
 return bp.materialize(r,['a']).decode()
def region(source,kind):
 start=source.index('struct ObjectData {\n')+len('struct ObjectData {\n') if kind=='struct' else source.index('        Self(Rc::new(ObjectData {\n')+len('        Self(Rc::new(ObjectData {\n')
 end=source.index('\n}',start) if kind=='struct' else source.index('        }))',start)
 return start,end,source[start:end]
def fields(block,kind):
 lines=block.splitlines(keepends=True);result={};pending=[]
 if kind=='struct':
  for line in lines:
   pending.append(line);m=re.match(r'    (\w+):',line)
   if m:result[m[1]]=''.join(pending);pending=[]
  assert not ''.join(pending).strip();return result
 # Initializer values span lines only for the new checked u32 conversion.
 key=None
 for line in lines:
  m=re.match(r'            (\w+):',line)
  if m:key=m[1];result[key]=line
  else:assert key is not None;result[key]+=line
 return result

def source(path,selected):
 selected=set(selected);old=bp.blob(OLD,path).decode()
 if not selected:return old.encode()
 if selected==set(PARTS):return full(path).encode()
 if path.endswith('/flags.rs'):return full(path).encode() if 'flags' in selected else b''
 if path.endswith('/tests.rs'):return old.encode() # test-only full-a guard is112; partials retain <=120
 s=old
 if path.endswith('/class_fields.rs'):
  if 'flags' in selected:s=s.replace('use super::{ObjectRef, is_array_index_key};','use super::{Flag, ObjectRef, is_array_index_key};').replace('self.0.string_indices_pending.get()','self.0.flags.get(Flag::StringIndicesPending)')
  if 'count' in selected:s=s.replace('usize::from(is_array_index_key(key))','u32::from(is_array_index_key(key))')
  return s.encode()
 assert path==PREFIX+'value/object.rs'
 modern=full(path)
 if 'flags' in selected:
  s=s.replace('mod class_fields;','mod class_fields;\nmod flags;').replace('use storage::{Payload, PropertyStorage};','use flags::{Flag, ObjectFlags};\nuse storage::{Payload, PropertyStorage};')
  for field,flag in FLAGS.items():
   s=s.replace('self.0.'+field+'.get()','self.0.flags.get(Flag::'+flag+')').replace('self.0.'+field+'.set(','self.0.flags.set(Flag::'+flag+', ')
 for kind in ('struct','initializer'):
  start,end,block=region(s,kind);oldfields=fields(region(old,kind)[2],kind);newfields=fields(region(modern,kind)[2],kind);values=dict(oldfields)
  if 'count' in selected:values['index_property_count']=newfields['index_property_count']
  if 'flags' in selected:
   for field in FLAGS:del values[field]
   values['flags']=newfields['flags']
  if 'order' in selected:
   order=list(newfields)
   if 'flags' not in selected:where=order.index('flags');order[where:where+1]=list(FLAGS)
  else:
   order=list(oldfields)
   if 'flags' in selected:
    order=['flags' if x=='extensible' else x for x in order if x not in FLAGS or x=='extensible']
  assert set(order)==set(values)
  replacement=''.join(values[x] for x in order)
  if kind=='struct':replacement=replacement.rstrip('\n')
  s=s[:start]+replacement+s[end:]
 return s.encode()
