from pathlib import Path
import json,re,shutil
import argparse
a=argparse.ArgumentParser();a.add_argument("--output-dir",type=Path,required=True);a.add_argument("--rust",type=Path,required=True);a.add_argument("--ng",type=Path,required=True);a.add_argument("--bundle",type=Path,required=True);args=a.parse_args();p=args.output_dir.resolve();p.mkdir(parents=True,exist_ok=True);(p/"tofte.js").write_bytes(args.bundle.read_bytes())
for role,src in [('r',args.rust),('n',args.ng)]:
 (p/role).mkdir(exist_ok=True);shutil.copy2(src,p/role/'qjs')
spec={}
def emit(key,body,decls='',seed='',inputs='3',N=1000):
 for C in [4,8]:
  for K in [4,8]:
   ops=''.join(body.replace('gK','g'+str(i)) for i in range(K))
   src=decls+'function f(v){'+seed+ops+'return 1;}function run(n,v){var out;for(var i=0;i<n;i++){'+('out=f(v);'*C)+'}return out;}if(run('+str(N)+','+inputs+')!==1)throw 1;print("OK");'
   (p/f'{key}-{C}-{K}.js').write_text(src)
 spec[key]={'N':N,'kind':'matrix'}
# Hold parent binding count fixed while changing sibling declarations.
for E in [0,1]:
 for S in [16,50,100]:
  key=f'decl-s{S}-e{E}'
  for C in [4,8]:
   for H in [4,8]:
    vars=','.join('a'+str(i) for i in range(S-H-2))
    fns=''.join(f'function g{i}(){{return v;}}' for i in range(H))
    flag='if(v<0)eval("v");' if E else ''
    src=f'function f(v){{var {vars};{fns}{flag}return g0;}}function run(n,v){{var out;for(var i=0;i<n;i++){{'+('out=f(v);'*C)+'}return out;}if(typeof run(1500,3)!=="function")throw 1;print("OK");'
    (p/f'{key}-{C}-{H}.js').write_text(src)
  spec[key]={'N':1500,'kind':'matrix'}
# Near real declaration count, fixed source binding count (50).
for E in [0,1]:
 key=f'decl-near-e{E}'
 for C in [4,8]:
  for H in [24,28]:
   vars=','.join('a'+str(i) for i in range(50-H-2));fns=''.join(f'function g{i}(){{return v;}}' for i in range(H));flag='if(v<0)eval("v");' if E else ''
   src=f'function f(v){{var {vars};{fns}{flag}return g0;}}function run(n,v){{var out;for(var i=0;i<n;i++){{'+('out=f(v);'*C)+'}return out;}if(typeof run(1500,3)!=="function")throw 1;print("OK");'
   (p/f'{key}-{C}-{H}.js').write_text(src)
 spec[key]={'N':1500,'kind':'matrix','K':[24,28]}
# Generic own-local accesses, same closure-free body with one unreachable eval.
for E in [0,1]:
 flag='if(v<0)eval("v");' if E else ''
 # Catch forces both onto interpreter/direct frame instead of folded/wide bodies.
 emit(f'local-e{E}', 's+=v;',seed='var s=0;'+flag+'try{',N=30000)
 for f in p.glob(f'local-e{E}-*.js'):
  t=f.read_text().replace('return 1;}', '}catch(e){throw e;}return 1;}');f.write_text(t)
# Warm evals, small/large scope; one getter keeps inner call wide.
for S in [0,40]:
 seed='var '+','.join('a'+str(i) for i in range(S))+';' if S else ''
 seed+='var s=0;function g(){return v.getTime();}'
 emit(f'eval-call-s{S}','s+=eval("g()");',seed=seed,inputs='new Date(1167613871000)',N=2000)
 emit(f'direct-call-s{S}','s+=g();',seed=seed+'if(v===null)eval("g()");',inputs='new Date(1167613871000)',N=2000)
# Actual declaration bodies: telescope fixed initialization without empty bodies.
text=(p/'tofte.js').read_text();prefix=text.split('    var self = this;')[0];prefix=prefix[prefix.index('Date.prototype.formatDate = function (input,time) {')+len('Date.prototype.formatDate = function (input,time) {'):]
arrays=re.findall(r'    var (\w+) =\s*(\[[\s\S]*?\]);',prefix)
# only top-level six literals; nested daysinmonths belongs to a function.
arrays=arrays[:6]
stripped=prefix
for name,val in arrays:stripped=stripped.replace('var '+name+' =', 'var '+name+' =',1);stripped=re.sub(r'    var '+name+r' =\s*\[[\s\S]*?\];','    var '+name+';',stripped,count=1)
fnstart=stripped.index('    function a()')
functions=stripped[fnstart:]
allvars=['switches','daysLong','daysShort','monthsShort','monthsLong','daysSuffix']+['a','A','B','d','D','F','g','G','h','H','i','j','l','L','m','M','n','O','r','s','S','t','U','W','w','Y','y','z']+['self','prevTime','ia','ij']
seedvars='var '+','.join(allvars)+';'
fullarrays=''.join('var '+name+'='+value+';' for name,value in arrays)
flag='if(input===null)eval(time);'
parts={'base':flag,'scope':seedvars+flag,'decls':seedvars+functions+flag,'arrays':seedvars+functions+fullarrays+flag,'splitjoin':seedvars+functions+fullarrays+'self=this;var result=input.split("").join("");'+flag}
for name,body in parts.items():
 for C in [4,8]:
  src='function f(input,time){'+body+'return 1;}var date=new Date(1167613871000);function run(n){var out;for(var i=0;i<n;i++){'+('out=f.call(date,"----","");'*C)+'}return out;}if(run(1000)!==1)throw 1;print("OK");'
  (p/f'fixed-{name}-{C}.js').write_text(src)
 spec['fixed-'+name]={'N':1000,'kind':'call'}
# Distinct source cache pressure: identifier spelling is same length, same bound values.
for mode in ['warm','cold']:
 for C in [4,8]:
  for K in [4,8]:
   step='s+=eval(mode==='+'"warm"?"v+0":"v+0/*"+i+"*/");'
   # All source production same except decimal suffix; do not interpret as exclusive compile.
   src='function f(v,mode,i){var s=0;'+step*K+'return s;}function run(n){var out;for(var i=0;i<n;i++){'+(f'out=f(3,"{mode}",i);'*C)+'}return out;}if(run(2000)!=='+str(3*K)+')throw 1;print("OK");'
   (p/f'eval-{mode}-{C}-{K}.js').write_text(src)
  spec['eval-'+mode]={'N':2000,'kind':'matrix'}
(p/'spec.json').write_text(json.dumps(spec,indent=2))
print('keys',len(spec),'inputs',len(list(p.glob('*.js'))))
