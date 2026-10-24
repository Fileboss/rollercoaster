"""Run one of Looping's export commands on documents. Runs under Wine with a Windows Python.

usage: python.exe export.py LOOPING_EXE OUT_DIR COMMAND_ID EXT FILE.loo [FILE.loo ...]

For each file, Looping is started on it, the menu command COMMAND_ID is sent
to its main window (33187 "MLD textuel au format XML...", 33154 "Script LDD
au format TXT..."), and the file dialog it opens is given OUT_DIR/<stem><EXT>.
"""
import ctypes, ctypes.wintypes as W, os, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import extract as X  # noqa: E402  (the module's main() only runs as a script)

u = X.u
u.PostMessageW.argtypes = [W.HWND, W.UINT, W.WPARAM, W.LPARAM]
u.GetDlgItem.argtypes = [W.HWND, ctypes.c_int]
u.GetDlgItem.restype = W.HWND


def process_windows(pid):
    out = []
    for h in X.windows():
        p = W.DWORD()
        u.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and u.IsWindowVisible(h):
            out.append(h)
    return out


def file_dialog(pid):
    for h in process_windows(pid):
        if X.cls(h) == "#32770" and any(X.cls(c) == "Edit" for c in X.windows(h)):
            return h
    return None


def select_type(dlg, ext):
    """Pick the entry of the "file type" combo box (cmb1, 0x470) that lists
    `*<ext>`: the image export writes the format of the selected filter."""
    combo = u.GetDlgItem(dlg, 0x470)
    if not combo:
        return
    for i in range(u.SendMessageW(combo, 0x146, 0, 0)):  # CB_GETCOUNT
        n = u.SendMessageW(combo, 0x149, i, 0)  # CB_GETLBTEXTLEN
        b = ctypes.create_unicode_buffer(n + 1)
        u.SendMessageW(combo, 0x148, i, ctypes.addressof(b))  # CB_GETLBTEXT
        if f"*{ext})" in b.value or f"*{ext}," in b.value:
            u.SendMessageW(combo, 0x14E, i, 0)  # CB_SETCURSEL
            # CBN_SELCHANGE, as if chosen by the user
            u.SendMessageW(dlg, 0x111, (1 << 16) | 0x470, combo)
            return


def export(exe, path, command, target, timeout=60):
    import subprocess

    proc = subprocess.Popen([exe, path])
    try:
        deadline = time.time() + timeout
        frame = None
        while time.time() < deadline and not (frame and X.panes(frame).get("LDD") is not None):
            time.sleep(0.5)
            frame = X.main_window(proc.pid)
        time.sleep(1)
        u.PostMessageW(frame, 0x111, command, 0)  # WM_COMMAND
        dlg = None
        while time.time() < deadline and not dlg:
            time.sleep(0.3)
            dlg = file_dialog(proc.pid)
            # a message box instead of the file dialog: export refused
            for h in process_windows(proc.pid):
                if not dlg and X.cls(h) == "#32770":
                    msg = " / ".join(X.text(c) for c in X.windows(h) if X.cls(c) == "Static" and X.text(c))
                    return msg or "?"
        if not dlg:
            raise TimeoutError(f"{path}: no file dialog")
        select_type(dlg, os.path.splitext(target)[1])
        edit = next(c for c in X.windows(dlg) if X.cls(c) == "Edit")
        buf = ctypes.create_unicode_buffer(target)
        u.SendMessageW(edit, 0x0C, 0, ctypes.addressof(buf))  # WM_SETTEXT
        u.PostMessageW(dlg, 0x111, 1, 0)  # IDOK
        while time.time() < deadline:
            time.sleep(0.3)
            # dismiss message boxes (e.g. "export done")
            for h in process_windows(proc.pid):
                if X.cls(h) == "#32770" and h != dlg and not file_dialog(proc.pid):
                    u.PostMessageW(h, 0x111, 1, 0)
            # Looping adds its default extension (.txt) to unknown ones
            for written in (target, target + ".txt"):
                if os.path.exists(written) and os.path.getsize(written) > 0:
                    time.sleep(1)
                    if written != target:
                        os.replace(written, target)
                    return None
        raise TimeoutError(f"{path}: nothing written")
    finally:
        proc.kill()
        proc.wait()


def main():
    exe, out_dir, command, ext, files = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4], sys.argv[5:]
    os.makedirs(out_dir, exist_ok=True)
    for f in files:
        stem = os.path.splitext(os.path.basename(f))[0]
        refused = export(exe, f, command, os.path.join(out_dir, stem + ext))
        print(stem, f"refused: {refused}" if refused else "ok", flush=True)


if __name__ == "__main__":
    main()
