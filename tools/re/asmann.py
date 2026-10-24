import re,sys
sys.path.insert(0,'.')
from strres import s_at
out=open(sys.argv[2],'w')
for line in open(sys.argv[1]):
    m=re.search(r'# 0x(14[0-9a-f]{7})',line)
    if m:
        a=int(m.group(1),16)
        if 0x140360000<=a<0x140462790:
            s=s_at(a)
            if s: line=line.rstrip('\n')+'  ; '+s[:80]+'\n'
    out.write(line)
