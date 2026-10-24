import sys,re
src=open(sys.argv[1]).read()
parts=re.split(r'^//==== FUNC ',src,flags=re.M)
idx={p.split(' ',1)[0]:p for p in parts[1:]}
for a in sys.argv[2:]:
    a=a.lower().replace('0x','')
    a=a if len(a)==9 else a
    p=idx.get(a) or idx.get(a.zfill(9))
    print('//==== FUNC '+(p if p else a+' NOT FOUND'))
