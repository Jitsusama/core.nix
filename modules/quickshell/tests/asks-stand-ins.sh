# shellcheck shell=bash
# Readies a folder for tests/asks-served.qml: a git tree to run in, op and
# loginctl stood in for, and two asks put up as an agent puts one, through
# the real secret-asks. op leaves a line in op-ran each time it runs and
# gives a value only for the one item; loginctl says the session is locked
# as the file `locked` says, "no" to begin with. It prints the two asks'
# ids, the one to release then the one to refuse.
# usage: asks-stand-ins.sh <folder> <item> <secret-asks> <git> <a shell>
set -euo pipefail

folder=$1
item=$2
asks=$3
git=$4
shell=$5

mkdir -p "$folder/runtime/daemons" "$folder/tree"
chmod 700 "$folder/runtime"
"$git" -C "$folder/tree" init -q
echo "a tree" > "$folder/tree/README"
"$git" -C "$folder/tree" add README
"$git" -C "$folder/tree" -c user.name=t -c user.email=t@example.com commit -q -m "a commit"

cat > "$folder/op" << EOF
#!$shell
echo "\$@" >> '$folder/op-ran'
[ "\$3" = '$item' ] || exit 1
printf '%s' 'this-is-not-a-secret'
EOF
cat > "$folder/loginctl" << EOF
#!$shell
case "\$1" in show-user) echo 2 ;; show-session) cat '$folder/locked' ;; esac
EOF
chmod +x "$folder/op" "$folder/loginctl"
echo no > "$folder/locked"

ask() {
  "$asks" ask --item "$item" --tree "$folder/tree" --why "To try the line." \
    --asks "$folder/runtime/secret-asks"
}
yes=$(ask)
no=$(ask)
echo "$yes $no"
