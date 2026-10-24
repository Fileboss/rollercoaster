"""Extract Looping's own MLD and SQL output. Runs under Wine with a Windows Python.

usage: python.exe extract.py LOOPING_EXE OUT_DIR FILE.loo [FILE.loo ...]

For each file, Looping is started on it and the text of its two RichEdit panes
("MLD" and "LDD") is written to OUT_DIR/<stem>.mld.txt and OUT_DIR/<stem>.sql,
UTF-8 with LF line endings. Looping fills both panes when it opens a document,
using the DBMS and the options stored in that document.
"""
import ctypes, ctypes.wintypes as W, os, subprocess, sys, time

u = ctypes.windll.user32
EnumProc = ctypes.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
u.SendMessageW.argtypes = [W.HWND, W.UINT, W.WPARAM, W.LPARAM]
u.SendMessageW.restype = ctypes.c_ssize_t
u.GetWindowThreadProcessId.argtypes = [W.HWND, ctypes.POINTER(W.DWORD)]


def text(h):
    n = u.SendMessageW(h, 0x0E, 0, 0)  # WM_GETTEXTLENGTH
    b = ctypes.create_unicode_buffer(n + 1)
    u.SendMessageW(h, 0x0D, n + 1, ctypes.addressof(b))  # WM_GETTEXT
    return b.value


def cls(h):
    b = ctypes.create_unicode_buffer(256)
    u.GetClassNameW(h, b, 256)
    return b.value


def windows(parent=None):
    out = []
    cb = EnumProc(lambda h, _: out.append(h) or True)
    if parent is None:
        u.EnumWindows(cb, 0)
    else:
        u.EnumChildWindows(parent, cb, 0)
    return out


def main_window(pid):
    for h in windows():
        p = W.DWORD()
        u.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and text(h).startswith("Looping - "):
            return h
    return None


def panes(frame):
    """{'MLD': text, 'LDD': text} from the RichEdit inside each named control bar."""
    out = {}
    for h in windows(frame):
        if cls(h).startswith("RichEdit"):
            bar = u.GetParent(u.GetParent(h))
            out[text(bar)] = text(h)
    return out


def extract(exe, path, timeout=60):
    proc = subprocess.Popen([exe, path])
    try:
        deadline, last, stable = time.time() + timeout, None, 0
        while time.time() < deadline:
            time.sleep(0.5)
            frame = main_window(proc.pid)
            if not frame:
                continue
            got = panes(frame)
            # wait until the text stops changing (generation can be incremental);
            # an empty model gives empty panes, so allow that after a longer wait
            if got and got == last:
                stable += 1
                if stable >= (2 if got.get("LDD") else 6):
                    return got
            else:
                stable = 0
            last = got
        raise TimeoutError(path)
    finally:
        proc.kill()
        proc.wait()


def main():
    exe, out_dir, files = sys.argv[1], sys.argv[2], sys.argv[3:]
    os.makedirs(out_dir, exist_ok=True)
    for f in files:
        stem = os.path.splitext(os.path.basename(f))[0]
        got = extract(exe, f)
        for key, ext in (("MLD", ".mld.txt"), ("LDD", ".sql")):
            with open(os.path.join(out_dir, stem + ext), "w", encoding="utf-8", newline="\n") as fh:
                fh.write(got.get(key, "").replace("\r\n", "\n").replace("\r", "\n"))
        print(stem, "ok", flush=True)


if __name__ == "__main__":
    main()
