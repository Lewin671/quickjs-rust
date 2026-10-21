"""Reviewable derived evidence: one instruction/access per line, typed indexes."""
import json
from pathlib import Path

def formatted(value,depth=0):
 small=json.dumps(value,separators=(',',':'),ensure_ascii=False,allow_nan=False)
 if not isinstance(value,(dict,list)) or not value or len(small)<=180 or isinstance(value,dict) and ('pc' in value or 'normalized_hex' in value):return small
 pad='  '*depth;child='  '*(depth+1)
 if isinstance(value,list):return '[\n'+',\n'.join(child+formatted(v,depth+1) for v in value)+'\n'+pad+']'
 return '{\n'+',\n'.join(child+json.dumps(k,ensure_ascii=False)+': '+formatted(v,depth+1) for k,v in value.items())+'\n'+pad+'}'

def write(path,value):
 path=Path(path);path.parent.mkdir(parents=True,exist_ok=True);path.write_text(formatted(value)+'\n')

def load_images(root):
 root=Path(root);index=json.loads((root/'images.json').read_text());index.pop('raw_job_sha256')
 index['audits']={role:json.loads((root/path).read_text()) for role,path in index['audits'].items()}
 return index

def load_mechanism(root):
 root=Path(root);data=json.loads((root/'mechanism.json').read_text())
 data['loops']={role:{case:json.loads((root/path).read_text()) for case,path in cases.items()} for role,cases in data['loops'].items()}
 return data
