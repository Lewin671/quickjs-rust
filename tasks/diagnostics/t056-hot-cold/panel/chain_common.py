"""Frozen-image checks for lead-submitted pre-attempt verification only."""
import hashlib,json,os,re,subprocess
from pathlib import Path

CANDIDATE='5dd3a810cac6a88f6a1f86c0ef46dc68dfac47ee'
BASE='f58c696e8379261ed726f0cea1051ed6d34f431a'
BASE_HASH='b0dfc65d0c026d62aa64ef3d69a089eb73b51088c84af75b44617ea2a1737f0e'
CANDIDATE_HASH='26ed840a9cbb5bc17c2993ad7fb61380bbeab03b3a5bf39acbe220512da783fc'
PLAN=Path('tasks/performance-units/interpreter-core-v2-layout-edges.json')
UNIT_HASH='d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284'
FOCUS=('external/sunspider-1.0/bitops-bitwise-and','development/graphlib/paths','external/sunspider-1.0/access-nbody','broad/global_read','broad/property_dynamic_read','broad/local_read','broad/branch_arithmetic',
       'external/sunspider-1.0/access-binary-trees','external/sunspider-1.0/string-unpack-code',
       'development/moment/dates','development/diff/compare','development/meriyah/parse','external/sunspider-1.0/math-spectral-norm','external/jetstream3-js-subset/raytrace-public-class-fields')

# User-required static fast-family prerequisite failed on this exact image.
STATIC_FAST_FAMILY_PASS=False

def fast_family():
    require(STATIC_FAST_FAMILY_PASS, 'static relative-PC fast-family prerequisite failed; no timing/last attempt on this candidate; see T056 hot/cold record')

def require(ok,why):
    if not ok:raise ValueError(why)
def read(path):return json.loads(Path(path).read_text())
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def out():return Path(os.environ['BENCH_OUT'])
def write(path,value):
    with Path(path).open('x') as f:json.dump(value,f,indent=2);f.write('\n')
def bound():
    require(subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==CANDIDATE,'wrong frozen candidate')
    require(not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],text=True),'dirty runner')
    subprocess.run(['git','merge-base','--is-ancestor','50b4254778d11548095b27cf46b874c873790e8b',CANDIDATE],check=True)
    from tools.benchmark.performance_decision import load_unit
    from tools.benchmark.layout_policy import load_policy
    unit,h=load_unit(PLAN)
    require(h==UNIT_HASH and unit['base_sha']==BASE and unit['migration']['current_stage']==1,'frozen unit drift')
    require(unit['fast_gate']['max_attempts']==2,'attempt budget drift')
    policy=load_policy(Path(unit['layout_gate']['policy_path']),unit['layout_gate']['policy_sha256'])
    write(out()/'binding.json',{'candidate':CANDIDATE,'unit_sha256':UNIT_HASH,'helper_sha256':sha(__file__),
          'valid_failed_attempts_at_preparation':1,'remaining_attempts_across_A_B_at_preparation':1})
    return unit,policy

def job(identifier):
    require(re.fullmatch(r'[0-9]{8}T[0-9]{6}Z-[a-z0-9._-]+',identifier),'invalid job id')
    p=Path(os.environ['BENCH_QUEUE'])/identifier
    require((p/'status').read_text().strip()=='done 0','prerequisite not done 0: '+identifier)
    require('DISTURBED' not in (p/'log').read_text(),'DISTURBED prerequisite: '+identifier)
    root=p/'out';binding=read(root/'binding.json')
    require(binding['candidate']==CANDIDATE and binding['unit_sha256']==UNIT_HASH and binding['helper_sha256']==sha(__file__),'prerequisite binding drift')
    return root

def canonical_images():
    for revision,expected in [(BASE,BASE_HASH),(CANDIDATE,CANDIDATE_HASH)]:
        script='set -euo pipefail; . scripts/lib.sh; c="$(qjs_resolve_cargo)"; qjs_build_revision "$PWD" "$c" "$1"'
        path=Path(subprocess.check_output(['bash','-c',script,'--',revision],text=True).strip())
        require(sha(path)==expected,'canonical image mismatch: '+revision)

def correctness(identifier):
    root=job(identifier);r=read(root/'correctness-ready.json')
    require(r['candidate']==CANDIDATE and r['full_gate_passed'] is True and r['ng_fixtures']==255 and r['subset_cases']==5170,'correctness receipt incomplete')
    for path,expected in r['artifacts'].items():require(sha(root/path)==expected,'correctness artifact changed')
    from tools.benchmark.performance_evaluation import _test262_zero_gap
    require(_test262_zero_gap(read(root/'burndown-entry.json'),CANDIDATE),'exact correctness not zero-gap')
    require(read(root/'image.json')['binary_sha256']==CANDIDATE_HASH,'exact image drift')
    return root

def pregate(identifier,policy):
    root=job(identifier);images=(root/'images.txt').read_text().splitlines()
    require(images[:2]==['base '+BASE,'candidate '+CANDIDATE],'pre-gate source drift')
    require(images[2].split()[0]==BASE_HASH and images[3].split()[0]==CANDIDATE_HASH,'pre-gate image drift')
    data=read(root/'pregate.json');rows=data['cases']
    require(data['pairs']==1 and len(rows)==86 and {r['id'] for r in rows}==set(policy['cases']),'pre-gate inventory drift')
    require(all(len(r['ratios']['instructions'])==1 and r['ratios']['instructions'][0]<=max(1.03,policy['cases'][r['id']]['instruction_band']) for r in rows),'pre-gate blocks this image')
    require('VERDICT pass []' in (root/'verdict.txt').read_text(),'pre-gate verdict missing')
    return root
