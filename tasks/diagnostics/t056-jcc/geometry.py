"""Exact end-exclusive geometry; candidate fusion is physical CMP/TEST+Jcc."""
import re
def branches(code,shift=0):
 result=[]
 for index,row in enumerate(code):
  if not row['op'].startswith(('j','call','ret')):continue
  pc=int(row['pc'],16) if isinstance(row['pc'],str) else row['pc']
  prior=code[index-1] if index else None
  before=int(prior['pc'],16) if prior and isinstance(prior['pc'],str) else prior['pc'] if prior else None
  paired=bool(prior and row['op'].startswith('j') and not row['op'].startswith('jmp') and prior['op'].startswith(('cmp','test')) and before+prior['length']==pc)
  start=(before if paired else pc)+shift;pc+=shift;end=pc+row['length']
  args=hex(int(row['args'],16)+shift) if shift and re.fullmatch('0x[0-9a-f]+',row['args']) else row['args']
  result.append({'sequence':index,'pc':hex(pc),'relative_PC':row['offset'],'op':row['op'],'args':args,'length':row['length'],'pair_start':hex(start) if paired else None,'pair_candidate':paired,'start_mod32':start%32,'branch_mod32':pc%32,'end_mod32':end%32,'crosses32':pc//32!=(end-1)//32,'ends32':end%32==0,'branch_affected':pc//32!=end//32,'pair_or_branch_affected':start//32!=end//32})
 return result

def summary(rows):
 hit=[r for r in rows if r['pair_or_branch_affected']]
 return {'dynamic_branches':len(rows),'affected_dynamic':len(hit),'affected_unique':len({r['pc'] for r in hit}),'individual_dynamic':sum(r['branch_affected'] for r in rows),'pair_only_dynamic':sum(r['pair_or_branch_affected'] and not r['branch_affected'] for r in rows)}
