import re,sys
src=open('d3.c').read()
parts=re.split(r'^//==== FUNC ',src,flags=re.M)[1:]
calls={}
for p in parts:
    a=p.split(' ',1)[0]
    calls[a]=set(re.findall(r'FUN_(1[0-9a-f]{8})\(',p))-{a}
def callers(t): return [a for a,c in calls.items() if t in c]
cmd=sys.argv[1]
for t in sys.argv[2:]:
    if cmd=='up': print(t,'<-',callers(t))
    else: print(t,'->',sorted(calls.get(t,[])))
