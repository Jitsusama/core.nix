# shellcheck shell=bash
# Checks the job that keeps a Mac's local name, with a stand-in scutil that
# keeps the name in a file and records each set: a Mac macOS renamed a-mac-2
# gets a-mac back and says so once, and one already named a-mac is left
# unwritten and says nothing.
# usage: local-name-comes-back.sh <keep.sh> <bash>
set -euo pipefail

keep=$1
bash=$2
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/scutil" <<EOF
#!${bash}
case "\$1" in
  --get) cat "$work/name" ;;
  --set) printf '%s\n' "\$3" > "$work/name"; echo set >> "$work/sets" ;;
  *) exit 64 ;;
esac
EOF
chmod +x "$work/bin/scutil"
export PATH="$work/bin:$PATH"

echo a-mac-2 > "$work/name"
"$bash" "$keep" a-mac 2> "$work/said"
name=$(<"$work/name")
if [[ "${name}" != a-mac ]]; then
  echo "a renamed Mac's name stayed ${name}" >&2
  exit 1
fi
said=$(<"$work/said")
if [[ "${said}" != *"the local name was a-mac-2, so it is a-mac again" ]]; then
  echo "setting the name back said: ${said}" >&2
  exit 1
fi

: > "$work/sets"
"$bash" "$keep" a-mac 2> "$work/said"
if [[ -s "$work/sets" || -s "$work/said" ]]; then
  echo "a name already right was written again, or the job said something" >&2
  exit 1
fi
