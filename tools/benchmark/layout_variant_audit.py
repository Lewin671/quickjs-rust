"""Static x86_64 Mach-O body/placement audit for the layout panel.

Resolve PC-relative references to symbols/sections so relocation bytes may
change while instructions, targets and symbol-relative offsets remain exact.
Only the last function's trailing section filler may differ.
"""
from __future__ import annotations

import bisect
import json
import re
import struct

from .layout_identity import command, digest

def sections(binary):
    data=binary.read_bytes(); cursor=32; result=[]
    assert struct.unpack_from('<I',data)[0]==0xfeedfacf
    for _ in range(struct.unpack_from('<I',data,16)[0]):
        cmd,size=struct.unpack_from('<II',data,cursor)
        if cmd==0x19:
            for i in range(struct.unpack_from('<I',data,cursor+64)[0]):
                sec=cursor+72+i*80
                name=data[sec:sec+16].rstrip(b'\0').decode()
                segment=data[sec+16:sec+32].rstrip(b'\0').decode()
                addr,length,offset=struct.unpack_from('<QQI',data,sec+32)
                result.append((addr,length,segment+'/'+name,offset))
        cursor+=size
    return result

def inspect(binary, out, tag):
    listing=command(['nm','-n',binary]); (out/(tag+'.nm')).write_text(listing)
    rows=[]
    for line in listing.splitlines():
        p=line.split()
        if len(p)==3 and p[1] in ('t','T') and p[2]!='__mh_execute_header': rows.append((int(p[0],16),p[2]))
    assert len(rows)==len(set(n for a,n in rows))==len(set(a for a,n in rows))
    sec=sections(binary);text=next(x for x in sec if x[2]=='__TEXT/__text')
    addresses=[a for a,n in rows]; names=[n for a,n in rows]
    def target(address):
        if text[0]<=address<text[0]+text[1]:
            i=bisect.bisect_right(addresses,address)-1
            if i>=0:return names[i]+'+'+hex(address-addresses[i])
        for a,size,name,off in sec:
            if a<=address<a+size:return name+'+'+hex(address-a)
        if 0x100000000 <= address < text[0]: return 'HEADER+'+hex(address-0x100000000)
        return None
    assembly=command(['otool','-tv',binary]);(out/(tag+'.asm')).write_text(assembly)
    instr=[]
    for line in assembly.splitlines():
        m=re.match(r'^([0-9a-f]{16})\s+([^\s]+)\s*(.*)',line)
        if m:instr.append((int(m[1],16),m[2],m[3].split('##')[0].strip()))
    bodies={n:[] for a,n in rows}
    for i,(a,op,args) in enumerate(instr):
        idx=bisect.bisect_right(addresses,a)-1
        if idx<0:continue
        following=instr[i+1][0] if i+1<len(instr) else text[0]+text[1]
        args=re.sub(r'(-?0x[0-9a-f]+)\(%rip\)',lambda m:'RIP['+(target(following+int(m[1],16)) or 'RAW:'+m[1])+']',args)
        branch = op.startswith(('j','call','loop'))
        if op in ('rep','repne','repe','bnd'):
            branch = bool(re.match(r'(?:j\w*|call\w*|loop\w*)\s+',args))
        if branch:
            args=re.sub(r'(?<![\w$])0x[0-9a-f]+$',lambda m:target(int(m[0],16)) or 'RAW_REL:'+hex(int(m[0],16)-following),args)
        bodies[names[idx]].append(op+' '+args)
    (out/(tag+'.normalized.json')).write_text(json.dumps(bodies,indent=2)+'\n')
    sizes={n:b-a for (a,n),(b,_) in zip(rows,rows[1:])}
    return {'sha256':digest(binary),'bytes':binary.stat().st_size,'addresses':dict((n,a) for a,n in rows),'sizes':sizes,'bodies':bodies,'sections':sec}


def audit(natural, variant, pad):
    original = natural['addresses']
    actual = variant['addresses']
    filtered = {n: a for n, a in actual.items() if n != '_qjs_layout_noise_pad'}
    if list(filtered) != list(original):
        raise ValueError('layout audit: function inventory/order changed')
    shift = actual[next(iter(original))] - original[next(iter(original))]
    if not pad <= shift < pad + 32 or shift % 16:
        raise ValueError('layout audit: invalid uniform shift')
    if any(actual[n] != a + shift for n, a in original.items()):
        raise ValueError('layout audit: nonuniform function displacement')
    def strip_tail(body):
        result = list(body)
        while result and result[-1].startswith(('nop', 'int3')):
            result.pop()
        return result
    last = list(original)[-1]
    for name, body in natural['bodies'].items():
        other = variant['bodies'][name]
        if name == last:
            body, other = strip_tail(body), strip_tail(other)
        if body != other:
            raise ValueError(f'layout audit: body drift in {name}')
    return {'functions': len(original), 'pad_bytes': pad,
            'actual_uniform_shift': shift, 'body_drift': 0, 'address_drift': 0}
