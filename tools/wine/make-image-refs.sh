#!/bin/sh
# Reference images of the real Looping (see extract.sh, EXPORT=...):
#   samples/ref/<name>.png   "Exporter le modèle au format image" (PNG)
# for every example and synthetic case, as Looping draws them under Wine
# (Calibri replaced by Carlito). Compared by looping-cli/tests/images.rs.
#   tools/wine/make-image-refs.sh [FILE.loo...]   (default: all)
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
if [ $# -eq 0 ]; then
    for f in samples/*.loo samples/*.Loo samples/cases/*.loo; do
        [ -e "$f" ] && set -- "$@" "$f"
    done
fi
EXPORT="33136 .png" tools/wine/extract.sh samples/ref "$@"
