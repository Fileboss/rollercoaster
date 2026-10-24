"""Text metrics as GDI reports them (run under Wine with a Windows Python).

usage: python.exe metrics.py FACE OUT.json
Writes, for lfHeight -8..-30 and weights 400/700, tmHeight/tmAscent and the
advance (GetTextExtentPoint32W) of every character U+0020..U+017F.
"""
import ctypes, ctypes.wintypes as W, json, sys

g, u = ctypes.windll.gdi32, ctypes.windll.user32
g.CreateFontW.restype = W.HANDLE
g.CreateFontW.argtypes = [ctypes.c_int] * 5 + [W.DWORD] * 8 + [W.LPCWSTR]
g.SelectObject.argtypes = [W.HDC, W.HANDLE]
g.GetTextExtentPoint32W.argtypes = [W.HDC, W.LPCWSTR, ctypes.c_int, ctypes.POINTER(W.SIZE)]


class TM(ctypes.Structure):
    _fields_ = [(n, W.LONG) for n in ("h", "asc", "desc", "il", "el", "ave", "max", "weight", "over", "dax", "day")] + [
        ("rest", ctypes.c_byte * 16)
    ]


def main():
    face, out = sys.argv[1], sys.argv[2]
    dc = g.CreateCompatibleDC(None)
    res = {}
    for weight in (400, 700):
        for h in range(-8, -31, -1):
            f = g.CreateFontW(h, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 0, 0, face)
            old = g.SelectObject(dc, f)
            tm = TM()
            g.GetTextMetricsW(dc, ctypes.byref(tm))
            adv = {}
            for c in range(0x20, 0x180):
                s = W.SIZE()
                g.GetTextExtentPoint32W(dc, chr(c), 1, ctypes.byref(s))
                adv[c] = s.cx
            s = W.SIZE()
            sample = "Liste ORDONNÉE de mots clés"
            g.GetTextExtentPoint32W(dc, sample, len(sample), ctypes.byref(s))
            res[f"{weight}/{h}"] = {"height": tm.h, "ascent": tm.asc, "adv": adv, "sample": s.cx}
            g.SelectObject(dc, old)
            g.DeleteObject(f)
    json.dump(res, open(out, "w"))


if __name__ == "__main__":
    main()
