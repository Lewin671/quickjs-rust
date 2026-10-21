# Arguments: CORRECTNESS_JOB PREGATE_JOB. Lead submits this one with --timing.
# Native T061 fixed controls are retained; --target here is diagnostic focus,
# not an edit of the frozen migration's three payoff targets.
python3 - "$@" <<'PY'
import subprocess,sys
from common import bound,correctness,pregate,canonical_images,require,out,BASE,CANDIDATE,FOCUS,read
require(len(sys.argv)==3,'usage: triage CORRECTNESS_JOB PREGATE_JOB')
unit,policy=bound();correctness(sys.argv[1]);pregate(sys.argv[2],policy);canonical_images()
args=['./scripts/quick-triage.sh','--base',BASE,'--candidate',CANDIDATE,'--output',str(out()/'triage')]
for name in FOCUS:args.extend(['--target',name])
subprocess.run(args,check=True)
r=read(out()/'triage/triage.json')
print('DIAGNOSTIC ONLY:',r.get('verdict'),'consult T056 attempt accounting; inspect all cases manually',flush=True)
PY
