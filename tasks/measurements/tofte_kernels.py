"""Nonempty operation kernels for T038; generated JS is diagnostic input only."""
from pathlib import Path
import hashlib
import json
import re

BUNDLE_SHA = "fabd80e6d4c685f5723de063ba77f7d8e0bca4af6bdecd30dd87611b7f758032"

def generate(p: Path, bundle: Path) -> None:
    if hashlib.sha256(bundle.read_bytes()).hexdigest() != BUNDLE_SHA:
        raise ValueError("bundle is not the hash-verified T038 workload")
    p.mkdir(parents=True, exist_ok=True)
    (p / "tofte.js").write_bytes(bundle.read_bytes())
    src=(p/'tofte.js').read_text()
    # Supplementary source event counters; never time this altered script.
    src=re.sub(r'((?:self|newDate|date)\s*\.(get\w+)\(\))',lambda m:f'hit("{m[2]}",{m[1]})',src)
    src=re.sub(r'function (\w+)\(([^)]*)\)\s*\{',lambda m:m[0]+f' hits["fn:{m[1]}"]=(hits["fn:{m[1]}"]||0)+1;',src)
    src=src.replace('function (input,time) {','function (input,time) {hits.format=(hits.format||0)+1;')
    src=src.replace('if (array[i] == x)', 'hits.membership=(hits.membership||0)+1; if (array[i] == x)')
    src=src.replace('while (ia[ij]) {','while (ia[ij]) {hits.char=(hits.char||0)+1;')
    src=src.replace('print(\'__QJS_EXTERNAL_OK__\')','print(\'__QJS_EXTERNAL_OK__\')')
    src='var hits={};function hit(k,v){hits[k]=(hits[k]||0)+1;return v;}\n'+src+'\nprint(JSON.stringify(hits));\n'
    (p/'source-counts.js').write_text(src)
    # All C/K micro shapes have non-empty callees; prices are B=(D8-D4)/4, A=2D4-D8.
    kernels={
    'scalar':('s+=v;', '', 'var s=0;'),
    'date-hours':('s+=v.getHours();','','var s=0;'),
    'date-date':('s+=v.getDate();','','var s=0;'),
    'date-month':('s+=v.getMonth();','','var s=0;'),
    'date-year':('s+=v.getFullYear();','','var s=0;'),
    'date-time':('s+=v.getTime();','','var s=0;'),
    'date-set':('v.setTime(1167613871000);s+=1;','','var s=0;'),
    'date-new':('s+=new Date("January 1 2001 00:00:00 +0000").getFullYear();','','var s=0;'),
    'string-concat':('s=(s+v).substring(1);','','var s="ab";'),
    'number-concat':('s+=("0"+v).length;','','var s=0;'),
    'string-new':('s+=new String(v).length;','','var s=0;'),
    'string-length':('s+=v.length;','','var s=0;'),
    'array-read':('s+=v[1];','','var s=0;'),
    'array-write':('v[1]=3;s+=1;','','var s=0;'),
    'string-eq':('s+=v=="m";','','var s=0;'),
    'array-exists':('s+=arrayExists(v,"m");','function arrayExists(a,x){for(var j=0;j<a.length;j++)if(a[j]==x)return true;return false;}','var s=0;'),
    'eval-small':('s+=eval("v");','','var s=0;'),
    'eval-large':('s+=eval("v");','','var '+','.join('a'+str(i)+'='+str(i) for i in range(44))+';var s=0;'),
    'eval-call':('s+=eval("g()");','', 'var s=0;function g(){return v;}'),
    'eval-call-large':('s+=eval("g()");','', 'var '+','.join('a'+str(i)+'='+str(i) for i in range(44))+';var s=0;function g(){return v;}'),
    'closure':('function gK(){return v;}s+=1;','','var s=0;'),
    'closure-large':('function gK(){return v;}s+=1;','','var '+','.join('a'+str(i)+'='+str(i) for i in range(44))+';var s=0;'),
    'closure-eval-large':('function gK(){return v;}s+=1;','','var '+','.join('a'+str(i)+'='+str(i) for i in range(44))+';var s=0;'),
    }
    inputs={k:'2' for k in kernels}
    for k in kernels:
     if k.startswith('date-'):inputs[k]='new Date(1167613871000)'
    inputs.update({'string-concat':'"z"','string-length':'new String(11)','array-read':'[1,3]','array-write':'[1,3]','string-eq':'"m"','array-exists':'["a","A","B","d","D","F","g","G","h","H","i","j","l","L","m","M","n","O","r","s","S","t","U","w","W","y","Y","z"]'})
    for key,(op,extra,seed) in kernels.items():
     for C in (4,8):
      for K in (4,8):
       N=2000 if key.startswith(('closure','eval')) else 30000
       ops=''.join(op.replace('gK','g'+str(i)) for i in range(K))
       end='s+=eval("v");' if key=='closure-eval-large' else ''
       code=extra+f'function f(v){{{seed}{ops}{end}return s;}}function run(n,v){{var out;for(var i=0;i<n;i++){{'+ 'out=f(v);'*C +f'}}return out;}}var result=run({N},{inputs[key]});if(result===undefined)throw 1;print("OK");'
       (p/f'{key}-{C}-{K}.js').write_text(code)
    (p/'component-kernels.json').write_text(__import__('json').dumps({'N':{k:2000 if k.startswith(('closure','eval')) else 30000 for k in kernels},'kernels':kernels,'inputs':inputs},indent=2))
    src=(p/'tofte.js').read_text().split('var date = new Date')[0]
    cfg={'N':{}}
    for char,name in [('Y','year'),('m','month'),('d','day'),('l','weekday'),('F','monthname'),('g','hour'),('i','minute'),('s','second'),('A','ampm'),('-','literal')]:
     key='format-'+name;cfg['N'][key]=300
     for C in (4,8):
      for K in (4,8):
       code=src+'\nvar date=new Date("1/1/2007 1:11:11");function run(n){var out;for(var j=0;j<n;j++){'+('out=date.formatDate('+json.dumps(char*K)+');')*C+'}return out;}var result=run(300);if(result.length===0)throw 1;print("OK");'
       (p/f'{key}-{C}-{K}.js').write_text(code)
    (p/'format-kernels.json').write_text(json.dumps(cfg,indent=2))
    cfg={'N':{}}
    for key in ['scan-step','eval-name-count','concat-only','array-literal']:
     cfg['N'][key]=30000 if key in ['scan-step','concat-only'] else 2000
     for C in (4,8):
      for K in (4,8):
       N=cfg['N'][key]
       if key=='scan-step':
        callee='function f(v){var s=0;for(var j=0;j<v.length;j++){if(v[j]=="x")s++;}return s;}'
        value=json.dumps(['m']*K)
       elif key=='eval-name-count':
        callee='function f(v){var '+','.join(f'a{i}={i}' for i in range(44))+';var s=0;'+'s+=eval("v");'*K+'return s;}'
        value='2'
       elif key=='concat-only':
        callee='function f(v){var s=0;'+'s+=("a"+v).length;'*K+'return s;}'
        value='"bc"'
       else:
        callee='function f(v){var s=0;'+'s+=["a","b","c","d","e","f","g"][v].length;'*K+'return s;}'
        value='2'
       code=callee+'function run(n,v){var out;for(var j=0;j<n;j++){'+('out=f(v);'*C)+'}return out;}var result=run('+str(N)+','+value+');if(result===undefined)throw 1;print("OK");'
       (p/f'{key}-{C}-{K}.js').write_text(code)
    (p/'extra-kernels.json').write_text(json.dumps(cfg))
    cfg={'N':{}}
    for key in ['matched-eval','matched-direct','array-escaped','date-day','date-minute','date-second','date-legacy-year','call-captured']:
     N=1000 if key.startswith(('matched','array')) else 30000;cfg['N'][key]=N
     for C in (4,8):
      for K in (4,8):
       if key.startswith('matched'):
        seed='var '+','.join(f'a{i}={i}' for i in range(20))+';var s=0;'+''.join(f'function g{i}(){{return v.getTime();}}' for i in range(28))
        op='s+=eval("g0()");' if key=='matched-eval' else 's+=g0();'
        callee='function f(v){'+seed+op*K+'if(v===null)eval("0");return s;}'
        value='new Date(1167613871000)'
       elif key=='array-escaped':
        callee='function f(v){var s=0;'+'v.a=["a","b","c","d","e","f","g"];s+=v.a.length;'*K+'return s;}'
        value='{a:null}'
       elif key=='call-captured':
        callee='function make(v){function g(){return v.getTime();}return g;}var g=make(new Date(1167613871000));function f(v){var s=0;'+'s+=v();'*K+'return s;}'
        value='g'
       else:
        method={'date-day':'getDay','date-minute':'getMinutes','date-second':'getSeconds','date-legacy-year':'getYear'}[key]
        callee='function f(v){var s=0;'+f's+=v.{method}();'*K+'return s;}'
        value='new Date(1167613871000)'
       code=callee+'function run(n,v){var out;for(var j=0;j<n;j++){'+('out=f(v);'*C)+'}return out;}var result=run('+str(N)+','+value+');if(result===undefined)throw 1;print("OK");'
       (p/f'{key}-{C}-{K}.js').write_text(code)
    (p/'matched-kernels.json').write_text(json.dumps(cfg))

    cfg = {'N': {'strict-string-scan': 30000, 'native-string-search': 30000}}
    for C in (4, 8):
        for K in (4, 8):
            source = (p / f'scan-step-{C}-{K}.js').read_text().replace('=="x"', '==="x"')
            (p / f'strict-string-scan-{C}-{K}.js').write_text(source)
            source = 'function f(v){return v.indexOf("x")>=0;}function run(n,v){var out;for(var j=0;j<n;j++){' + ('out=f(v);'*C) + '}return out;}var result=run(30000,' + json.dumps(['m']*K) + ');if(result!==false)throw 1;print("OK");'
            (p / f'native-string-search-{C}-{K}.js').write_text(source)
    (p / 'search-kernels.json').write_text(json.dumps(cfg))
