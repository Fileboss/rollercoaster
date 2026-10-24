import struct,re,sys
sys.path.insert(0,'.')
from pe import d,off2va,va2off
# find all 8-byte pointers in .rdata/.data pointing to ASCII class-like names, and check CRuntimeClass shape
res=[]
lo=0x140000000
for sec in ('.rdata','.data'):
    pass
def cstr(va):
    o=va2off(va)
    if o is None: return None
    e=d.find(b'\0',o,o+64)
    if e<0: return None
    s=d[o:e]
    if re.fullmatch(rb'C[A-Za-z0-9_]{2,60}',s): return s.decode()
for o in range(0x35ee00, 0x35ee00+943104+45568, 8):
    p=struct.unpack_from('<Q',d,o)[0]
    if not (0x140360000<=p<0x140462790): continue
    n=cstr(p)
    if not n: continue
    size,schema=struct.unpack_from('<iI',d,o+8)
    create,base=struct.unpack_from('<QQ',d,o+16)
    if 0<size<0x100000 and (create==0 or 0x140001000<=create<0x14035fa00):
        res.append((n,hex(off2va(o)),hex(size),hex(schema),hex(create),hex(base)))
for r in res:
    if not r[0].startswith(('CMFC','COle','CWnd','CCmd')): print(*r)
