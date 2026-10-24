#!/bin/sh
# Reference MLD/SQL from the real Looping, run headless under Wine.
#   tools/wine/extract.sh OUT_DIR FILE.loo...
# With EXPORT="COMMAND_ID EXT", runs that export command of Looping instead
# (export.py), e.g. EXPORT="33187 .xml" (MLD as XML), "33154 .vba" (script
# file, a VBA module for Access), "33136 .png" (image of the diagram).
# Needs wine and a Windows "embeddable" Python in $WINPY (default
# ~/.cache/looping-winpy, downloaded on first use). Uses its own prefix
# ($WINEPREFIX, default ~/.cache/looping-wine) with the null graphics driver,
# so no window shows up.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
WINPY="${WINPY:-$HOME/.cache/looping-winpy}"
if [ ! -x "$WINPY/python.exe" ]; then
    zip=$(mktemp)
    curl -sSfL -o "$zip" https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip
    mkdir -p "$WINPY"
    python3 -m zipfile -e "$zip" "$WINPY"
    rm -f "$zip"
fi
export WINEPREFIX="${WINEPREFIX:-$HOME/.cache/looping-wine}" WINEDEBUG=-all
if [ ! -d "$WINEPREFIX" ]; then
    wineboot -i >/dev/null 2>&1
    wine reg add 'HKCU\Software\Wine\Drivers' /v Graphics /d null /f >/dev/null 2>&1
    wineserver -w
fi
# Calibri (the documents' font) is not free: use Carlito, its metric clone,
# so that the image exports have the text layout of Windows
if [ ! -e "$WINEPREFIX/.carlito" ]; then
    wine reg add 'HKCU\Software\Wine\Fonts\Replacements' /v Calibri /d Carlito /f >/dev/null 2>&1
    touch "$WINEPREFIX/.carlito"
fi
out=$1; shift
win() { winepath -w "$(realpath "$1")"; }
args=""
for f in "$@"; do args="$args|$(win "$f")"; done
mkdir -p "$out"
IFS='|'
if [ -n "$EXPORT" ]; then
    # shellcheck disable=SC2086
    wine "$WINPY/python.exe" "$(win "$here/export.py")" "$(win "${LOOPING_EXE:-$root/Looping.exe}")" "$(win "$out")" ${EXPORT% *} ${EXPORT#* } ${args#|}
else
    # shellcheck disable=SC2086
    wine "$WINPY/python.exe" "$(win "$here/extract.py")" "$(win "${LOOPING_EXE:-$root/Looping.exe}")" "$(win "$out")" ${args#|}
fi
