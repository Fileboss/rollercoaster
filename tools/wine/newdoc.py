"""Have Looping create and save a new, empty document. Runs under Wine with a Windows Python.

usage: python.exe newdoc.py LOOPING_EXE TARGET.loo

Looping is started without a file (no document), « Nouveau » (57600) creates
one, then « Enregistrer sous » (57604) is sent and its file dialog given TARGET. The settings block of
the result is Looping's own default for a new document (fresh Wine prefix: no
user preferences).
"""
import os, subprocess, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import export as E  # noqa: E402
import extract as X  # noqa: E402


def main():
    exe, target = sys.argv[1], sys.argv[2]
    proc = subprocess.Popen([exe])
    try:
        deadline = time.time() + 60
        frame = None
        while time.time() < deadline and not frame:
            time.sleep(0.5)
            frame = next((h for h in E.process_windows(proc.pid) if X.text(h).startswith("Looping")), None)
        time.sleep(2)
        E.u.PostMessageW(frame, 0x111, 57600, 0)  # ID_FILE_NEW
        # « Voulez-vous récupérer les propriétés d'un modèle existant ? » : non
        while time.time() < deadline and not X.main_window(proc.pid):
            time.sleep(0.5)
            for h in E.process_windows(proc.pid):
                if X.cls(h) == "#32770" and any(X.text(c) == "&No" or X.text(c) == "&Non" for c in X.windows(h)):
                    E.u.PostMessageW(h, 0x111, 7, 0)  # IDNO
        time.sleep(2)
        E.u.PostMessageW(frame, 0x111, 57604, 0)  # ID_FILE_SAVE_AS
        dlg = None
        while time.time() < deadline and not dlg:
            time.sleep(0.3)
            dlg = E.file_dialog(proc.pid)
        if not dlg:
            raise TimeoutError("no file dialog")
        edit = next(c for c in X.windows(dlg) if X.cls(c) == "Edit")
        buf = E.ctypes.create_unicode_buffer(target)
        E.u.SendMessageW(edit, 0x0C, 0, E.ctypes.addressof(buf))  # WM_SETTEXT
        E.u.PostMessageW(dlg, 0x111, 1, 0)  # IDOK
        while time.time() < deadline:
            time.sleep(0.5)
            if os.path.exists(target) and os.path.getsize(target) > 0:
                time.sleep(1)
                print("ok", flush=True)
                return
        raise TimeoutError("nothing written")
    finally:
        proc.kill()
        proc.wait()


if __name__ == "__main__":
    main()
