import struct,sys,os
# path of Looping.exe: $LOOPING_EXE, else the repository root
d=open(os.environ.get('LOOPING_EXE', os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../Looping.exe')),'rb').read()
pe=struct.unpack_from('<I',d,0x3c)[0]
n=struct.unpack_from('<H',d,pe+6)[0]; opt=struct.unpack_from('<H',d,pe+20)[0]
base=0x140000000
secs=[]
for i in range(n):
    o=pe+24+opt+40*i
    name=d[o:o+8].rstrip(b'\0').decode(); vs,va,rs,ro=struct.unpack_from('<IIII',d,o+8)
    secs.append((name,va,vs,ro,rs))
def off2va(x):
    for s in secs:
        if s[3]<=x<s[3]+s[4]: return base+s[1]+x-s[3]
def va2off(x):
    x-=base
    for s in secs:
        if s[1]<=x<s[1]+max(s[2],s[4]): return s[3]+x-s[1]
if __name__=='__main__':
    for s in secs: print(s)
    for a in sys.argv[1:]: print(a, hex(off2va(int(a,16))))
