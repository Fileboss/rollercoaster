"""MFC message-map entries (WM_COMMAND / ON_UPDATE_COMMAND_UI) of command ids.

usage: msgmap.py ID...   -> address of each entry, its code, handler
Needs pefile; uses $LOOPING_EXE, else ../../Looping.exe.
"""
import os, struct, sys
import pefile

EXE = os.environ.get("LOOPING_EXE") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../Looping.exe")
pe = pefile.PE(EXE, fast_load=True)
BASE = pe.OPTIONAL_HEADER.ImageBase
want = {int(a, 0) for a in sys.argv[1:]}
for sec in pe.sections:
    if sec.Name.rstrip(b"\0") not in (b".rdata", b".data"):
        continue
    data = sec.get_data()
    for off in range(0, len(data) - 32, 4):
        msg, code, idf, idl = struct.unpack_from("<IIII", data, off)
        if msg == 0x111 and idf in want and idl >= idf and idl - idf < 64:
            sig, pfn = struct.unpack_from("<QQ", data, off + 16)
            if BASE < pfn < BASE + 0x400000:
                kind = {0: "COMMAND", 0xffffffff: "UPDATE_UI"}.get(code, hex(code))
                print(f"{idf}-{idl} {kind:9} entry {BASE + sec.VirtualAddress + off:#x} handler {pfn:#x}")
