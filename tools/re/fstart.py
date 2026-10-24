import os,sys,re,bisect
lines=open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../re-cache/looping.asm')).read().splitlines()
addrs=[];starts=set()
prev_pad=True
for l in lines:
    m=re.match(r'\s+([0-9a-f]+):\s+(\S+)',l)
    if not m: continue
    a=int(m.group(1),16); op=m.group(2)
    addrs.append(a)
    if prev_pad and op not in('int3','nop'): starts.add(a)
    prev_pad = op in ('int3','ret','jmp') 
S=sorted(starts)
for x in sys.argv[1:]:
    a=int(x,16); i=bisect.bisect_right(S,a)-1
    print(x, hex(S[i]))
