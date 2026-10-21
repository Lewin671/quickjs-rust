"""Retry only counter-discontinuous observations; never choose timing outcomes.
A unit is the complete N/2N pair. Retain every abandoned mate in the raw log.
"""
import time
import counters,protocol as p

class InvalidObservation(Exception):
 def __init__(self,issues):
  super().__init__('counter discontinuity');self.issues=issues

def observe(status,value,description):
 assert value['cleanup_ok'] and value.get('cleanup_failed_step') is None
 assert value['child_status']==0 and not value['timed_out']
 counters.structure(value,description)
 if status:
  assert status==1 and value['failed_step']=='read.delta_integrity'
  assert value['capture_complete'] is False and value['counters_valid'] is False
  assert value['failed_return']==100 and value['failed_errno']==0 # Monterey EPROTO
  assert value['cycles'] is None and value['instructions'] is None
  for event in value['events']:
   assert event['count'] is None and event['idle_count'] is None
   assert event['per_cpu']==event['idle_per_cpu']==[None]*value['cpu_count']
 else:
  assert value['capture_complete'] and value['counters_valid'] and value.get('failed_step') is None
  counters.totals(value)
 issues=counters.discontinuities(value)
 if issues:raise InvalidObservation(issues)
 assert status==0,'unexplained helper integrity failure; do not retry'
 counters.check(value,description)
 return value

class Recovery:
 def __init__(self,emit):
  self.emit=emit;self.calls=0;self.failures=0;self.discarded_seconds=0.;self.retried_units=0
 def report(self):
  return {'planned_captures':p.PLANNED_CAPTURES,'measured_helper_calls':self.calls,'invalid_observations':self.failures,'invalid_per_planned_capture':self.failures/p.PLANNED_CAPTURES,'observed_invalid_fraction':self.failures/self.calls if self.calls else 0.,'discarded_unit_seconds':self.discarded_seconds,'retried_units':self.retried_units,'max_unit_attempts':p.MAX_UNIT_ATTEMPTS,'max_invalid_per_planned_capture':p.MAX_INVALID_FRACTION,'max_invalid_observations':p.MAX_INVALID_OBSERVATIONS,'retry_seconds_limit':p.RETRY_SECONDS}
 def pair(self,measure,labels,unit):
  for attempt in range(1,p.MAX_UNIT_ATTEMPTS+1):
   begin=time.monotonic();values={};label=None
   try:
    for label,n in labels:
     self.calls+=1;values[label]=measure(label,n,attempt)
   except InvalidObservation as error:
    self.failures+=1;self.discarded_seconds+=time.monotonic()-begin
    if attempt==1:self.retried_units+=1
    self.emit({'unit':unit,'attempt':attempt,'outcome':'invalid_observation','failed_size':label,'abandoned_valid_sizes':list(values),'issues':error.issues,'quality':self.report()})
    if self.failures>p.MAX_INVALID_OBSERVATIONS:raise RuntimeError('counter failure burden exceeds prespecified5% of352 planned captures') from error
    if self.discarded_seconds>p.RETRY_SECONDS:raise RuntimeError('counter retry wall budget exhausted') from error
    if attempt==p.MAX_UNIT_ATTEMPTS:raise RuntimeError('counter integrity retry bound exhausted; unit missing') from error
   else:
    self.emit({'unit':unit,'attempt':attempt,'outcome':'accepted_pair','quality':self.report()})
    return values
  raise AssertionError('unreachable')
 def finish(self):
  assert self.failures/p.PLANNED_CAPTURES<=p.MAX_INVALID_FRACTION
  return self.report()
