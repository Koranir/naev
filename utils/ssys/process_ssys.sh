#!/usr/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
BAS=$(realpath --relative-to="$PWD" "${SCRIPT_DIR}/../../dat")
DST="$BAS/ssys"

COL=-c
#COL=

git checkout "$BAS/spob" "$DST"
#git checkout "$DST"

echo -n "gen colored sys map... " >&2
cmd=$(./utils/ssys/ssys2pov.py -C dat/ssys/*.xml) && $cmd 2>/dev/null && mv -v out.png map_bef.png
echo -n "freeze non-empty: " >&2
"$SCRIPT_DIR"/ssys_empty.py -r "$DST"/*.xml | "$SCRIPT_DIR"/ssys_freeze.py -f | wc -l
echo "gen before graph" >&2
"$SCRIPT_DIR"/ssys2dot.py $COL "$DST"/*.xml -k | neato -n2 -Tpng 2>/dev/null > before.png
echo "gen after graph " >&2
"$SCRIPT_DIR"/ssys2dot.py "$DST"/*.xml | tee before.dot | neato 2>/dev/null |
tee after.dot | neato -n2 -Tpng 2>/dev/null > after.png
echo -n "apply after graph " >&2
"$SCRIPT_DIR"/dot2ssys.py < after.dot
echo "gen final graph" >&2
"$SCRIPT_DIR"/ssys2dot.py $COL "$DST"/*.xml -k | neato -n2 -Tpng 2>/dev/null > final.png
echo -n "gen colored sys map... " >&2
cmd=$(./utils/ssys/ssys2pov.py -C dat/ssys/*.xml) && $cmd 2>/dev/null && mv -v out.png map_fin.png
echo "relax" >&2
"$SCRIPT_DIR"/ssys_relax.py "$DST"/*.xml | wc -l
