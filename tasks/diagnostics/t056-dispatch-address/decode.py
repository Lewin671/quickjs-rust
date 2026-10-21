"""Decode only captured numeric Typed bank/op fields; raw padding retained."""
import struct
def decode(row):
 regs=[]
 for i in range(8):
  raw=bytes.fromhex(row['bank_hex'])[i*16:(i+1)*16];bits=int.from_bytes(raw[8:16],'little');regs.append({'index':i,'raw_hex':raw.hex(),'tag':raw[0],'payload_bits':f'{bits:016x}'})
 ops=[]
 for i in range(row['ops_count']):
  raw=bytes.fromhex(row['ops_hex'])[16*i:16*(i+1)];kind={2:'Binary',26:'Exit',4:'Update'}[raw[0]];o={'kind':kind,'raw_hex':raw.hex()}
  if kind=='Binary':o.update(operator={0:'Add',16:'Lt'}[raw[1]],dst=struct.unpack_from('<H',raw,2)[0],left=struct.unpack_from('<H',raw,4)[0],right=struct.unpack_from('<H',raw,6)[0])
  elif kind=='Exit':o.update(cond=struct.unpack_from('<H',raw,2)[0],exit_ip=struct.unpack_from('<I',raw,4)[0])
  else:o.update(operator='Increment' if raw[1]==0 else 'Decrement',dst=struct.unpack_from('<H',raw,2)[0],src=struct.unpack_from('<H',raw,4)[0])
  ops.append(o)
 return {**row,'backedge_iterations':0,'registers':regs,'ops':ops}
