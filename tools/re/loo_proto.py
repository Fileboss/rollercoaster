import struct,sys
class R:
    def __init__(s,d): s.d=d; s.p=0; s.v=0; s.objs=[None]; s.trace=[]
    def u16(s): x=struct.unpack_from('<H',s.d,s.p)[0]; s.p+=2; return x
    def i32(s): x=struct.unpack_from('<i',s.d,s.p)[0]; s.p+=4; return x
    def u32(s): x=struct.unpack_from('<I',s.d,s.p)[0]; s.p+=4; return x
    def raw(s,n): x=s.d[s.p:s.p+n]; assert len(x)==n; s.p+=n; return x
    def count(s):
        n=s.u16()
        if n!=0xffff: return n
        n=s.u32()
        if n!=0xffffffff: return n
        return struct.unpack_from('<Q',s.raw(8))[0]
    def strlen(s):
        # AfxReadStringLength
        b=s.d[s.p]; s.p+=1
        if b<0xff: return b,1
        w=s.u16()
        if w==0xfffe:
            l,_=s.strlen(); return l,2
        if w<0xffff: return w,1
        dw=s.u32()
        if dw<0xffffffff: return dw,1
        return struct.unpack_from('<Q',s.raw(8))[0],1
    def cstr(s):
        n,cs=s.strlen()
        b=s.raw(n*cs)
        return b.decode('utf-16le') if cs==2 else b.decode('cp1252')
    def strlist(s):
        return [s.cstr() for _ in range(s.count())]
    def obj(s):
        start=s.p
        tag=s.u16()
        if tag==0x7fff: tag=s.u32(); big=True
        if tag==0: return None
        if tag==0xffff:
            schema=s.u16(); nl=s.u16(); name=s.raw(nl).decode()
            s.objs.append(('class',name)); cls=name
        elif tag & 0x8000:
            cls=s.objs[tag&0x7fff][1]
        else:
            return ('ref',tag, s.objs[tag])
        o={'_cls':cls,'_at':hex(start)}
        s.objs.append(o)
        f=getattr(s,'r_'+cls,None)
        if not f: raise Exception('no reader for %s at %x'%(cls,start))
        f(o)
        s.trace.append(o)
        return o
    # ---- classes
    def r_CDrawObj(s,o):
        v=s.v
        o['f140']= s.u16() if v<7 else s.u32()
        o['fd8']=s.u16(); o['fe4']=s.u16()
        if v>=0x17: o['fdc']=s.u16(); o['fec']=s.u16(); o['fe0']=s.u16(); o['ff4']=s.u16()
        o['r28']=struct.unpack('<4i',s.raw(16)); o['b78']=struct.unpack('<3I',s.raw(12))
        if v>=0x17:
            o['r38']=struct.unpack('<4i',s.raw(16)); o['b98']=struct.unpack('<3I',s.raw(12))
            o['r48']=struct.unpack('<4i',s.raw(16)); o['bb8']=struct.unpack('<3I',s.raw(12))
        o['id']=s.i32()
        o['f308']=[s.i32() for _ in range(4)]
        o['f318']= s.i32() if v>=2 else 6
        if v>=0x22:
            o['fe8']=s.u16(); o['ff0']=s.u16(); o['ff8']=s.u16()
            o['b88']=struct.unpack('<3I',s.raw(12)); o['ba8']=struct.unpack('<3I',s.raw(12)); o['bc8']=struct.unpack('<3I',s.raw(12))
        if v>=0x2d: o['f338']=s.i32()
    def r_CDrawArc(s,o):
        s.r_CDrawObj(o)
        if s.v>0x26: o['a']=[s.i32(),s.i32(),s.i32()]; o['s']=s.cstr()
    r_CDrawArcCif=r_CDrawArcContrainte=r_CDrawArcHeritage=r_CDrawArcHeritageUML=r_CDrawArcObjetLibre=r_CDrawArcRegle=r_CDrawArc
    def r_CDrawEntite(s,o):
        s.r_CDrawObj(o)
        fl=s.u16(); o['flags']=fl; o['rect4d0']=struct.unpack('<4i',s.raw(16))
    r_CDrawEntiteUML=r_CDrawEntite
    def r_CDrawRelation(s,o):
        s.r_CDrawObj(o)
        o['r390']=struct.unpack('<2i',s.raw(8)); o['f398']=s.i32(); o['f3a0']=s.i32()
        if s.v>=3: o['r3a4']=struct.unpack('<2i',s.raw(8))
        if s.v>0x18: o['f3ac']=s.i32()
        if s.v>0x1d: o['f39c']=s.i32(); s.i32(); s.i32(); o['s']=s.cstr()
    def param(s):
        v=s.v; p={}
        p['f8']=s.i32(); p['name']=s.cstr(); p['f20']=s.i32(); p['f24']=s.i32()
        if v>0xf:
            p['f28']=s.i32(); p['s48']=s.cstr(); p['s50']=s.cstr(); p['s58']=s.cstr(); p['f60']=s.i32(); p['s18']=s.cstr(); p['s118']=s.cstr()
        if v>0x13: p['s138']=s.cstr(); p['f140']=s.i32(); p['f144']=s.i32()
        if v>0x27: p['f14c']=s.i32()
        if v>0x19: p['f148']=s.i32()
        p['f2c..']=[s.i32() for _ in range(6)]
        if v>0xd: p['f8c']=s.i32()
        p['f90']=s.i32(); p['s98']=s.cstr(); p['fa0']=s.raw(8).hex(); p['fa8']=s.i32()
        p['cur_b0']=s.raw(12).hex(); p['cur_c0']=s.raw(12).hex()
        p['fd0']=s.i32(); p['f64']=s.i32()
        if v>=0x24: p['f68']=[s.i32() for _ in range(5)]
        p['f7c']=s.i32(); p['f80']=s.i32(); p['fd4']=s.i32(); p['fd8']=s.i32()
        p['se0']=s.cstr(); p['se8']=s.cstr(); p['sf0']=s.cstr()
        if v>=0x11: p['n12c']=s.i32()
        if v>0x11: p['n130']=s.i32()
        if v>=0x1f: p['f134']=s.i32()
        p['sf8']=s.cstr()
        if v>=4: p['s100']=s.cstr(); p['f108']=s.i32()
        if v>=5: p['f10c']=s.i32()
        if v>=6: p['f110']=s.i32(); p['f114']=s.i32(); p['s120']=s.cstr()
        if v>=7: p['s40']=s.cstr()
        if v>=0xb: p['f128']=s.i32()
        return p
    def r_CDrawTexte(s,o):
        s.r_CDrawObj(o); v=s.v
        o['f458']=s.i32(); o['f45c']=s.i32(); o['sl468']=s.strlist()
        if v>0xd: o['sl4a0']=s.strlist(); o['f4d8']=s.i32()
        if v>0xf: o['s5f8']=s.cstr()
        if v>0x29: o['f460']=s.i32(); s.i32(); s.i32()
        o['fonts']=s.raw(100+0x5c+100+0x5c)
        o['f4dc']=s.u16()
        n=s.i32(); o['params']=[s.param() for _ in range(n)]
    def r_CDrawImage(s,o):
        s.r_CDrawObj(o); v=s.v
        s.u16(); o['rect']=struct.unpack('<4i',s.raw(16)); o['path']=s.cstr()
        if v>0x2d: o['f3a8']=[s.i32() for _ in range(4)]
        if v>0x2e: o['s3b8']=s.cstr(); o['f3c0']=s.i32()

def load(path):
    d=open(path,'rb').read(); r=R(d)
    h={}
    h['size1910']=struct.unpack('<2i',r.raw(8)); h['f191c']=r.i32(); h['f334']=r.i32(); h['f338']=r.u16(); h['f33c']=r.u16(); h['f350']=r.raw(4).hex()
    blk=r.raw(0x88c); r.v=v=struct.unpack_from('<i',blk)[0]; h['version']=v
    h['q1']=r.raw(8).hex(); h['q2']=r.raw(8).hex()
    if v>0x1c: h['f340']=r.u16()
    if v>0x30: h['s348']=r.cstr()
    if v>0x2b: h['obj228']=r.obj()  # unknown member
    if v>0xf: h['strs_bf8']=[r.cstr() for _ in range(33)]
    if v>0x12: h['s_d00']=r.cstr(); h['strs_d10']=[r.cstr() for _ in range(50)]
    if v>0x12: h['ea0']=[r.cstr(),r.cstr(),r.cstr()]
    if v>0x23: h['ea0b']=[r.cstr(),r.cstr()]
    print(h)
    n=r.count(); print('count',n,hex(r.p))
    out=[]
    try:
        for i in range(n): out.append(r.obj())
    except Exception as e:
        print('ERR',e, hex(r.p)); 
    for o in out:
        if o is None or isinstance(o,tuple): continue
        o['links']=[r.i32() for _ in range(6)]
        if o['_cls'].startswith('CDrawArc'):
            o['rattach']=(r.u16(), r.raw(8).hex(), r.i32())
        elif o['_cls'] in ('CDrawFleche','CDrawTrait'):
            o['rattach']=(r.u16(), r.raw(8).hex(), r.i32())
    return r,out
if __name__=='__main__':
    r,out=load(sys.argv[1])
    for o in r.trace:
        print(o['_at'],o['_cls'],o.get('id'),o.get('links'),o.get('rattach'), {k:v for k,v in o.items() if k in('sl468','sl4a0','s5f8','flags','rect4d0','s','path')}, [p['name'] for p in o.get('params',[])])
    print('end',hex(r.p),len(r.d))
