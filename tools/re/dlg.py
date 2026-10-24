import os,pefile,struct,sys
pe=pefile.PE(os.environ.get('LOOPING_EXE') or os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../Looping.exe'))
def sz(b,o):
    w=struct.unpack_from('<H',b,o)[0]
    if w==0xffff: return struct.unpack_from('<H',b,o+2)[0],o+4
    e=o
    while struct.unpack_from('<H',b,e)[0]: e+=2
    return b[o:e].decode('utf-16le'),e+2
CLS={0x80:'Button',0x81:'Edit',0x82:'Static',0x83:'ListBox',0x84:'ScrollBar',0x85:'ComboBox'}
def dlg(b):
    o=0
    ver,sig=struct.unpack_from('<HH',b,0)
    assert sig==0xffff
    helpid,exs,style,n,x,y,cx,cy=struct.unpack_from('<IIIHhhhh',b,4); o=26
    menu,o=sz(b,o); cls,o=sz(b,o); title,o=sz(b,o)
    if style&0x40:
        o+=6; face,o=sz(b,o)
    print(' title:',title)
    for i in range(n):
        o=(o+3)&~3
        hid,ex,st,x,y,cx,cy,id_=struct.unpack_from('<IIIhhhhI',b,o); o+=24
        c,o=sz(b,o); t,o=sz(b,o)
        el=struct.unpack_from('<H',b,o)[0]; o+=2+el
        c=CLS.get(c,c)
        print(f'  {id_:5d} {c:<18} {t!r} style={st:#x}')
want=set(int(x) for x in sys.argv[1:])
for e in pe.DIRECTORY_ENTRY_RESOURCE.entries:
    if e.id!=5: continue
    for d in e.directory.entries:
        if d.id in want:
            for l in d.directory.entries:
                r=l.data.struct
                b=pe.get_data(r.OffsetToData,r.Size)
                print('DIALOG',d.id)
                try: dlg(b)
                except AssertionError: print(' (DLGTEMPLATE non étendu, ignoré)')
