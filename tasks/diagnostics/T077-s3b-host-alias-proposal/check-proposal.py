"""Offline proposal test only: never edits/import-replaces the live guard."""
import copy,json,types,tempfile
from pathlib import Path
from unittest.mock import patch
from tools.benchmark.layout_identity import digest
from tools.benchmark.tests.test_layout_rule import policy

path=Path('tools/benchmark/layout_policy.py')
source=path.read_text()
old="""    if live and (policy['host'] != host_identity() or policy.get('host_node') != platform.node()):
        raise ValueError('layout policy host/recipe mismatch: recalibration required')"""
new="""    if live:
        actual_node = platform.node()
        same_node = policy.get('host_node') == actual_node or (
            policy.get('host_node') == 'iMac.local' and actual_node == 'iMac.lan'
        )
        if policy['host'] != host_identity() or not same_node:
            raise ValueError('layout policy host/recipe mismatch: recalibration required')"""
assert source.count(old)==1
module=types.ModuleType('tools.benchmark._s3b_alias_proposal')
module.__package__='tools.benchmark'
exec(compile(source.replace(old,new),'s3b-alias-proposal-only','exec'),module.__dict__)
identity=json.loads(Path('tasks/diagnostics/T077-s3b-host-qualification/identity.json').read_text())['actual']
data=policy();data.update(host_node='iMac.local',host=identity)
results=[]
with tempfile.TemporaryDirectory() as directory:
 p=Path(directory)/'policy.json'
 def check(name,node,actual,accept=True,expected='iMac.local'):
  value=copy.deepcopy(data);value['host_node']=expected;p.write_text(json.dumps(value))
  module.host_identity=lambda:actual
  with patch.object(module.platform,'node',return_value=node):
   try:loaded=module.load_policy(p,digest(p))
   except ValueError as e:
    assert not accept and 'recalibration' in str(e),(name,str(e))
   else:
    assert accept,name
    assert loaded==value,(name,'policy values changed')
  results.append({'case':name,'accepted':accept,'passed':True})
 check('unchanged qualified hostname','iMac.local',identity)
 check('exact approved-alias proposal','iMac.lan',identity)
 check('unrelated hostname rejected','another-iMac.local',identity,False)
 check('alias is not a wildcard','iMac.other',identity,False)
 check('reverse mapping is not implicitly accepted','iMac.local',identity,False,'iMac.lan')
 for key in identity:
  changed=copy.deepcopy(identity);changed[key]={'changed':True}
  check('mismatched '+key+' rejected','iMac.lan',changed,False)
assert path.read_text()==source
Path('target/s3b-host-alias-offline-results.json').write_text(json.dumps({'scope':'offline proposal only; live guard unchanged; no timing','tests':results},indent=2)+'\n')
print(len(results),'offline proposal checks pass; live guard unchanged')
