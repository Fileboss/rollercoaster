"""MSVC x64 RTTI: vtable address of a class, and owner class of a vtable entry.

usage: vtable.py CLASS...        -> vtable address and first slots
       vtable.py --owner ADDR... -> class whose vtable contains ADDR
Needs pefile; uses $LOOPING_EXE, else ../../Looping.exe.
"""
import os, struct, sys
import pefile

EXE = os.environ.get("LOOPING_EXE") or os.path.join(os.path.dirname(__file__), "../../Looping.exe")
pe = pefile.PE(EXE, fast_load=True)
BASE = pe.OPTIONAL_HEADER.ImageBase
DATA = pe.__data__
rdata = next(s for s in pe.sections if s.Name.rstrip(b"\0") == b".rdata")
RVA0, RAW0, SIZE = rdata.VirtualAddress, rdata.PointerToRawData, rdata.SizeOfRawData


def q(va):
    return struct.unpack_from("<Q", pe.get_data(va - BASE, 8))[0]


def d(va):
    return struct.unpack_from("<I", pe.get_data(va - BASE, 4))[0]


def col_name(col):
    """Class name of a Complete Object Locator, or None."""
    try:
        if d(col) != 1:
            return None
        td = BASE + d(col + 12)
        return pe.get_data(td + 16 - BASE, 256).split(b"\0")[0].decode()
    except Exception:
        return None


def vtables():
    """{class name: vtable VA} for every COL referenced from .rdata."""
    out = {}
    for off in range(0, SIZE - 8, 8):
        ptr = struct.unpack_from("<Q", DATA, RAW0 + off)[0]
        if not (BASE + RVA0 <= ptr < BASE + RVA0 + SIZE):
            continue
        name = col_name(ptr)
        if name and name.startswith(".?AV") and d(ptr + 4) == 0:
            out.setdefault(name[4:-2], BASE + RVA0 + off + 8)
    return out


if __name__ == "__main__":
    vt = vtables()
    if sys.argv[1] == "--owner":
        for a in sys.argv[2:]:
            a = int(a, 16)
            best = max(((v, n) for n, v in vt.items() if v <= a), default=None)
            print(hex(a), best and f"{best[1]}+{a - best[0]:#x}")
    else:
        for c in sys.argv[1:]:
            v = vt.get(c)
            print(c, v and hex(v), v and [hex(q(v + 8 * i)) for i in range(4)])
