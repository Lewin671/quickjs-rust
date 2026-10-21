"""Offline replay and retry policy tests; never launch a helper or a workload."""
import copy,json,unittest
from pathlib import Path
from unittest.mock import patch
import counters,protocol as p,recovery
ROOT=Path(__file__).resolve().parent
ROWS=[json.loads(line) for line in (ROOT/'run-v3-initial.jsonl').read_text().splitlines()]
MEASURED=[row for row in ROWS if row['phase']=='measured']
GOOD=[json.loads(row['result']['stdout']) for row in MEASURED[:2]]
BAD=json.loads(MEASURED[2]['result']['stdout'])
DESCRIPTION=json.loads((ROOT/'groups-v3.json').read_text())['groups'][0]['description']

class RecoveryTests(unittest.TestCase):
 def test_real_raw_captures_identify_only_work_counter_discontinuities(self):
  for value in GOOD:self.assertIs(recovery.observe(0,value,DESCRIPTION),value)
  with self.assertRaises(recovery.InvalidObservation) as caught:recovery.observe(1,BAD,DESCRIPTION)
  issues=caught.exception.issues
  self.assertEqual({(i['cpu'],i['index']) for i in issues},{(0,0),(0,1),(2,0),(2,1)})
  self.assertEqual({i['window'] for i in issues},{'before..after'})
  decreases=[i for i in issues if i['reason']=='counter_decrease']
  self.assertEqual([i['delta'] for i in decreases],[-281474648211501,-281473001498549])
 def test_valid_negative_N2N_and_noisy_idle_are_never_retry_conditions(self):
  value=counters.delta(dict(zip(('N','2N'),GOOD)))
  self.assertFalse(value['positive_cycle_instruction_increment'])
  self.assertIn('CPU_CLK_UNHALTED.THREAD',value['negative_events'])
  self.assertFalse(value['cycle_instruction_idle_at_most_5pct'])
 def test_second_capture_failure_replaces_both_sizes_in_same_order(self):
  audit=[];calls=[];retry=recovery.Recovery(audit.append)
  def measure(label,n,attempt):
   calls.append((label,n,attempt))
   if attempt==1 and label=='2N':raise recovery.InvalidObservation([{'reason':'counter_decrease'}])
   return (label,attempt)
  result=retry.pair(measure,[('N',p.COUNT),('2N',2*p.COUNT)],{'case':'real-unit'})
  self.assertEqual(result,{'N':('N',2),'2N':('2N',2)})
  self.assertEqual([(a,b) for a,b,_ in calls],[('N',p.COUNT),('2N',2*p.COUNT)]*2)
  self.assertEqual(audit[0]['abandoned_valid_sizes'],['N'])
  self.assertEqual(retry.finish()['invalid_observations'],1)
 def test_retry_wall_cap_is_fatal_and_recorded_before_abort(self):
  audit=[];retry=recovery.Recovery(audit.append)
  def fail(*args):raise recovery.InvalidObservation([{'reason':'counter_decrease'}])
  with patch.object(recovery.time,'monotonic',side_effect=[0,p.RETRY_SECONDS+1]):
   with self.assertRaisesRegex(RuntimeError,'wall budget'):retry.pair(fail,[('N',p.COUNT)],{})
  self.assertEqual(len(audit),1)
 def test_cleanup_configuration_timeout_and_unexplained_failure_are_fatal(self):
  for key,new in (('cleanup_ok',False),('readback',['0']*4),('timed_out',True)):
   value=copy.deepcopy(BAD);value[key]=new
   with self.assertRaises(AssertionError):recovery.observe(1,value,DESCRIPTION)
  value=copy.deepcopy(BAD);value['snapshots']['idle_after']['begin_ns']+=1000000000;value['snapshots']['idle_after']['end_ns']+=1000000000
  with self.assertRaises(AssertionError):recovery.observe(1,value,DESCRIPTION)
  value=copy.deepcopy(GOOD[0]);value.update(capture_complete=False,counters_valid=False,failed_step='read.delta_integrity',failed_return=100,failed_errno=0,cycles=None,instructions=None)
  for event in value['events']:event.update(count=None,idle_count=None,per_cpu=[None]*value['cpu_count'],idle_per_cpu=[None]*value['cpu_count'])
  with self.assertRaisesRegex(AssertionError,'unexplained'):recovery.observe(1,value,DESCRIPTION)
 def test_monotonic_huge_positive_jump_is_invalid_without_modulo_repair(self):
  value=copy.deepcopy(GOOD[0]);value['snapshots']['after']['per_cpu'][0][0]+=2**48
  value['events'][0]['count']+=2**48;value['events'][0]['per_cpu'][0]+=2**48;value['cycles']+=2**48
  with self.assertRaises(recovery.InvalidObservation) as caught:recovery.observe(0,value,DESCRIPTION)
  self.assertTrue(any(i['reason']=='counter_rate_bound' and i['delta']>2**48 for i in caught.exception.issues))
  self.assertEqual(value['snapshots']['after']['per_cpu'][0][0],GOOD[0]['snapshots']['after']['per_cpu'][0][0]+2**48)
  value['events'][0]['per_cpu'][0]+=1
  with self.assertRaises(AssertionError):recovery.observe(0,value,DESCRIPTION)

if __name__=='__main__':unittest.main()
