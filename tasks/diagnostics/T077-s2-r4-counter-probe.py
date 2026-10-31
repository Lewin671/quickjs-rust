"""Diagnostic-only archived source instrumentation, identical policy at both endpoints.

Counts dispatch operations by tier/kind, hot-IC attempts/results and element/store
routes. Never time these binaries. Manifest lists unsupported endpoint mechanisms.
"""
from pathlib import Path
import hashlib,json,re,subprocess,sys
revision,destination=sys.argv[1:];out=Path(destination).resolve();out.mkdir(parents=True,exist_ok=False)
proc=subprocess.Popen(['git','archive',revision],stdout=subprocess.PIPE)
subprocess.run(['tar','-x','-C',str(out)],stdin=proc.stdout,check=True);proc.stdout.close();assert proc.wait()==0
src=out/'crates/qjs-runtime/src';fields=set();patches=[];unsupported=[]
def field(name): fields.add(name);return 'crate::diagnostics::count!('+name+');'
def edit(path,transform):
 p=src/path;s=p.read_text();new=transform(s)
 if s!=new: p.write_text(new);patches.append({'path':path,'before_sha256':hashlib.sha256(s.encode()).hexdigest(),'after_sha256':hashlib.sha256(new.encode()).hexdigest()})
def function(path,name,label,outcome=None):
 def patch(s):
  pattern=r'^( *)(?:pub(?:\([^\n]*\))? )?fn '+re.escape(name)+r'\b'
  m=re.search(pattern,s,re.M)
  if not m: unsupported.append(path+'::'+name);return s
  start=s.index('{',m.end());end=re.search(r'^'+m[1]+r'}',s[start+1:],re.M)
  assert end,(path,name)
  stop=start+1+end.start();body=s[start+1:stop]
  head='\n'+field(label+'_attempts')+'\n'
  if outcome:
   expression='r4_result.is_some()' if outcome=='option' else 'r4_result.is_ok()' if outcome=='result' else 'r4_result'
   head+='let r4_result=(|| {'+body+'})();\nif '+expression+' {'+field(label+'_hits')+'} else {'+field(label+'_misses')+'}\nr4_result\n'
   return s[:start+1]+head+s[stop:]
  return s[:start+1]+head+s[start+1:]
 edit(path,patch)
def dispatch(path,prefix,table):
 def patch(s):
  # Helpers are after the dispatch body; the first matching arm is its site.
  for variant,label in table.items():
   pattern=r'(\b'+prefix+r'::'+variant+r'\b(?:\s*\{[^{}]*\})?\s*=>\s*\{)'
   s,count=re.subn(pattern,lambda m:m[1]+'\n'+field(label),s,count=1)
   if not count: unsupported.append(path+'::'+variant)
  return s
 edit(path,patch)
for prefix,path in [('WideOp','bytecode/compact_fn/wide/activation.rs'),('CompactOp','bytecode/compact_fn/activation.rs')]:
 tier='wide' if prefix=='WideOp' else 'compact'
 dispatch(path,prefix,{v:'r4_'+tier+'_'+k for v,k in [('GetPropThis','named_read'),('GetPropNamed','named_read'),('SetPropThis','named_write'),('SetPropNamed','named_write'),('GetProp','computed_read'),('GetPropIndex','index_read')]})
dispatch('bytecode/typed_loop/execute.rs','TypedOp',{v:'r4_typed_'+k for v,k in [('GetNamed','named_read'),('GetNamedTyped','numeric_named_read'),('SetNamed','named_write'),('SetNamedTyped','numeric_named_write'),('ComputedRead','computed_read'),('ElementRead','element_read'),('DenseRead','dense_read'),('DenseWrite','dense_write'),('ComputedWrite','computed_write'),('DenseWriteBoxed','dense_write_boxed')]})
for name in ['dense_write','dense_write_value']:
 function('bytecode/typed_loop/execute.rs',name,'r4_typed_'+name,'bool')
for name,outcome in [('get_prop_named_hot','option'),('get_prop_element','result'),('get_prop_index',None),('get_prop_computed',None),('try_plain_set_index','bool'),('try_plain_set_prop','bool')]:
 function('bytecode/compact_fn/property.rs',name,'r4_compact_'+name,outcome)
for name in ['integer_indexed_value','indexed_element_value','set_indexed_element','set_integer_indexed_element','try_set_integer_indexed_primitive_element','read_element','write_element']:
 function('typed_array/element.rs',name,'r4_typed_array_'+name)
def hook_counts(s):
 m=re.search(r'pub\(super\) fn get_prop_named\(',s);assert m
 start=s.index('{',m.end())
 return s[:start+1]+'\nif key.as_ref()=="length" && matches!(object, Value::Array(_)) {'+field('r4_array_length_miss_handler')+'}\n'+s[start+1:]
edit('bytecode/compact_fn/property.rs',hook_counts)
base='struct NamedPropertyCacheState' in (src/'bytecode/named_property_cache.rs').read_text()
function('bytecode/named_property_cache.rs','probe_hot' if base else 'get_value','r4_hot_ic','option')
if base:
 function('value/object/slot_reads.rs','own_data_slot_write','r4_value_slot','option')
 function('value/object/slot_reads.rs','shared_data_slot_write_number','r4_numeric_slot','bool')
 unsupported+=['S1 NamedStore numeric slots, append recipes and permanent megamorphic state do not exist at base']
else:
 function('value/layout/slots.rs','overwrite_guarded_number','r4_numeric_slot','bool')
 function('value/layout/append_recipe.rs','apply','r4_store_add','bool')
 def mega(s):
  needle='if matches!(self.state.borrow().entries, Entries::Mega) {'
  assert needle in s
  return s.replace(needle,needle+field('r4_megamorphic_reads'),1)
 path='bytecode/named_property_cache/misses.rs' if (src/'bytecode/named_property_cache/misses.rs').exists() else 'bytecode/named_property_cache.rs'
 edit(path,mega)
diag=src/'diagnostics.rs';s=diag.read_text();s=s.replace('declare_counters! {','declare_counters! {\n'+''.join('    '+f+',\n' for f in sorted(fields)),1);diag.write_text(s)
(out/'r4-probe-manifest.json').write_text(json.dumps({'revision':revision,'patches':patches,'fields':sorted(fields),'unsupported':unsupported,'note':'diagnostic-only; API attempt/hit counts may overlap dispatch events; generic handler counters include wide fallback dispatch; compact arithmetic-only tier has no property opcodes'},indent=2)+'\n')
