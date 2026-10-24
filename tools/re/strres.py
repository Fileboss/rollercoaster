import re,sys
sys.path.insert(0,'.')
from pe import d,va2off
def s_at(va):
    o=va2off(va)
    if o is None or o>=len(d): return None
    # utf16?
    w=[]
    i=o
    while i+1<len(d) and len(w)<200:
        u=d[i]|d[i+1]<<8
        if u==0: break
        if not (0x20<=u<0x2000 or u in (9,10,13)): w=None; break
        w.append(chr(u)); i+=2
    if w and len(w)>=1 and (len(w)>=2 or True):
        if len(w)>=1 and i+1<len(d) and d[i]==0 and d[i+1]==0:
            sw=''.join(w)
            if len(sw)>=1 and all(c.isprintable() or c in '\r\n\t' for c in sw):
                a=None
            else: sw=None
        else: sw=None
    else: sw=None
    # ascii
    e=d.find(b'\0',o,o+300)
    sa=None
    if e>o:
        b=d[o:e]
        try:
            t=b.decode('cp1252')
            if all(c.isprintable() or c in '\r\n\t' for c in t): sa=t
        except: pass
    if sw and (not sa or len(sw)>=len(sa)): return 'L'+repr(sw)
    if sa and len(sa)>=1: return repr(sa)
    if sw: return 'L'+repr(sw)
    return None
def rep(m):
    va=int(m.group(1),16)
    s=s_at(va)
    return m.group(0)+('/*%s*/'%s if s else '')
if __name__=="__main__":
  src=sys.stdin.read()
  print(re.sub(r'&?DAT_(14[0-9a-f]{7})',rep,src))
