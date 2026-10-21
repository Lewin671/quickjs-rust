"""Render build/audit or lead-only timing payload; never submit a timing job."""
import base64,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent
def render(kind='timing'):
 files={n:(ROOT/n).read_bytes() for n in ({'build':('build.py',),'audit':('audit.py','geometry.py','walk.py','decode.py'),'timing':('collect.py','protocol.py','analyze.py','chain_common.py','window.py','config.json')}[kind])}
 if kind=='audit':
  old=ROOT.parent/'t056-dispatch-address';queue=ROOT.parents[5]/'outputs/imac-bench'
  captures=json.loads((queue/'20261021T124843Z-t056-dispatch-heap-capture/out/captures.json').read_text())
  selected=[next(r for r in captures['snapshots'] if r['role']==role and r['case']=='broad/global_read') for role in ('aligned','candidate','count_flags')]
  files['captures.json']=json.dumps({'snapshots':selected}).encode()
  for role in ('aligned','candidate','count_flags'):
   data=json.loads((old/'loops'/(role+'-global_read.json')).read_text());files[role+'-global_read.json']=json.dumps({'code':[{'op':r['op']} for r in data['code']]}).encode()
 body='set -euo pipefail\nexport PYTHONDONTWRITEBYTECODE=1\nexport PYTHONPATH="$BENCH_OUT:$PWD${PYTHONPATH:+:$PYTHONPATH}"\npython3 - <<\'FILES\'\nfrom pathlib import Path\nimport base64,os\nout=Path(os.environ["BENCH_OUT"])\n'
 for name,content in files.items():body+='with (out/'+repr(name)+').open("xb") as f: f.write(base64.b64decode('+repr(base64.b64encode(content).decode())+'))\n'
 body+='FILES\npython3 "$BENCH_OUT/'+{'build':'build.py','audit':'audit.py','timing':'collect.py'}[kind]+'"\n'
 if kind!='timing':return body
 return 'set -euo pipefail\npython3 - <<\'T056_WINDOW\'\n'+(ROOT/'window.py').read_text()+'\nbody = '+repr(body)+'\ntry:\n    status=run(body,20*60,[])\nexcept ValueError as error:\n    print("error:",error,file=sys.stderr);status=124\nsys.exit(status)\nT056_WINDOW\n'
if __name__=='__main__':print(render(sys.argv[1] if len(sys.argv)>1 else 'timing'),end='')
