#!/bin/sh
# Reference exports of the real Looping (see extract.sh, EXPORT=...):
#   samples/ref/<name>.xml   "MLD textuel au format XML..."
#   samples/ref/<name>.vba   "Script LDD au format TXT..." with Access (VBA module)
# <name> segments force settings as in make-refs.sh (.<dbms>, .set-<off>-<val>).
# VBA_ONLY=1 skips the XML exports.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build -q -p looping-cli
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/xml" "$tmp/vba"

# resave DIR FILE SUFFIX [--set OFF=VAL]...
resave() {
    dir=$1 f=$2 suffix=$3
    shift 3
    target/debug/looping-cli resave "$f" "$tmp/$dir/$(basename "${f%.*}")$suffix.loo" "$@"
}

for f in samples/*.loo samples/*.Loo samples/cases/*.loo; do
    [ -e "$f" ] || continue
    if [ -z "$VBA_ONLY" ]; then
        resave xml "$f" ""
        resave xml "$f" .mysql --set 0x52c=1
    fi
    resave vba "$f" .access --set 0x52c=0
done
for opt in 7f4-1 7f4-2 7f8-1 7f8-2 7f8-3 888-1 888-4 a78-1 a78-3 ab8-1 b40-1 b40-2 7f0-0; do
    set -- --set "0x${opt%-*}=${opt#*-}"
    resave vba samples/beeven.loo ".access.set-$opt" --set 0x52c=0 "$@"
    [ -n "$VBA_ONLY" ] || resave xml samples/beeven.loo ".mysql.set-$opt" --set 0x52c=1 "$@"
done
for f in samples/cases/c1[5-8]_*.loo; do
    resave vba "$f" .access.set-888-4 --set 0x52c=0 --set 0x888=4
    [ -n "$VBA_ONLY" ] || resave xml "$f" .mysql.set-888-4 --set 0x52c=1 --set 0x888=4
done
for f in samples/cases/c14_*.loo samples/cases/c19_*.loo samples/cases/c20_*.loo samples/cases/c31_*.loo samples/cases/c21_*.loo; do
    for opt in 7f8-1 7f8-3 b40-1 888-4 7f4-1; do
        set -- --set "0x${opt%-*}=${opt#*-}"
        resave vba "$f" ".access.set-$opt" --set 0x52c=0 "$@"
        [ -n "$VBA_ONLY" ] || resave xml "$f" ".mysql.set-$opt" --set 0x52c=1 "$@"
    done
done
[ -n "$VBA_ONLY" ] || EXPORT="33187 .xml" tools/wine/extract.sh samples/ref "$tmp"/xml/*.loo
EXPORT="33154 .vba" tools/wine/extract.sh samples/ref "$tmp"/vba/*.loo
