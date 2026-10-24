#!/bin/sh
# Rebuilds samples/ref/ from the real Looping run under Wine (see extract.sh):
#   <stem>.mld.txt / <stem>.sql            each document as stored
#   <stem>.<dbms>.mld.txt / .sql           the same with the DBMS (+0x52c) forced
#   <stem>[.<dbms>].set-<off>-<val>.*      the same with one option forced
#                                          (settings i32 at CLoopingDoc+0x<off>)
#   <stem>[.set-<off>-<val>].fk-<n>.*      the same with a "Nom clé étrangère"
#                                          mask (FK_MASKS in tests/reference.rs)
# Documents: samples/*.loo, plus the synthetic models of
# `cargo run -p looping-core --example cases`, written to samples/cases/.
# Variants are written by our own resave, so this also checks that
# Looping opens the files we write.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
cargo build -q -p looping-cli
cargo run -q -p looping-core --example cases -- samples/cases
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

all_dbms="access mysql oracle postgresql sqlserver sybase db2 mariadb sqlite"
# types in the textual MLD (7f0), constraint names (7f4), comments (7f8), cascade (888), naming convention (a78),
# accents (ab8), placement of constraints (b40)
options="7f0-0 7f4-1 7f4-2 7f8-1 7f8-2 7f8-3 888-1 888-2 888-3 888-4 a78-1 a78-2 a78-3 a78-4 a78-5 ab8-1 b40-1 b40-2"

# options whose output depends on the DBMS (none of them shows with Access)
dbms_options="7f8-1 7f8-2 7f8-3 888-1 888-2 888-3 888-4"

# variants FILE "DBMS..." "OPTIONS..." ["DBMS-DEPENDENT OPTIONS"]
variants() {
    stem=$(basename "${1%.*}")
    i=0
    for d in $all_dbms; do
        case " $2 " in *" $d "*)
            target/debug/looping-cli resave "$1" "$tmp/$stem.$d.loo" --set 0x52c=$i ;;
        esac
        i=$((i + 1))
    done
    for opt in $3; do
        target/debug/looping-cli resave "$1" "$tmp/$stem.set-$opt.loo" --set "0x${opt%-*}=${opt#*-}"
    done
    i=0
    for d in $all_dbms; do
        [ $i -gt 0 ] && for opt in $4; do
            target/debug/looping-cli resave "$1" "$tmp/$stem.$d.set-$opt.loo" --set 0x52c=$i --set "0x${opt%-*}=${opt#*-}"
        done
        i=$((i + 1))
    done
}

# masks "[Classe]_[FK]" and "[FK]_[Classe]": Looping keeps the first 10
# characters at +0x7cc and the rest at +0xa52
masks() {
    stem=$(basename "${1%.*}")$2
    target/debug/looping-cli resave "$1" "$tmp/$stem.fk-1.loo" $3 --set-str '0x7cc=[Classe]_' --set-str '0xa52=[FK]'
    target/debug/looping-cli resave "$1" "$tmp/$stem.fk-2.loo" $3 --set-str '0x7cc=[FK]_[Clas' --set-str '0xa52=se]'
}

for f in samples/*.loo samples/*.Loo samples/cases/*.loo; do
    [ -e "$f" ] && masks "$f"
done
[ -e samples/beeven.loo ] && masks samples/beeven.loo .set-a78-1 "--set 0xa78=1"

for f in samples/*.loo samples/*.Loo; do
    [ -e "$f" ] || continue
    case "$f" in
        samples/beeven.loo) variants "$f" "$all_dbms" "$options" ;;
        *) variants "$f" "$all_dbms" "" ;;
    esac
done
for f in samples/cases/*.loo; do
    case "$f" in
        */c14_*) variants "$f" "mysql postgresql oracle" "$options" "$dbms_options" ;;
        */c1[5-8]_*) variants "$f" "mysql postgresql" "" "888-1 888-2 888-3 888-4" ;;
        */c27_*) variants "$f" "mysql postgresql" "7f0-0" ;;
        */c31_*) variants "$f" "mysql postgresql" "" "7f8-1 7f8-2 b40-1" ;;
        */c19_* | */c20_*) variants "$f" "mysql postgresql oracle sqlserver" "7f8-1" "7f8-1 7f8-2 7f8-3 888-1" ;;
        *) variants "$f" "mysql postgresql" "" ;;
    esac
done
tools/wine/extract.sh samples/ref samples/*.loo samples/*.Loo samples/cases/*.loo "$tmp"/*.loo
tools/wine/make-export-refs.sh
