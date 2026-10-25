# shellcheck shell=bash
# Fails on a folder that doesn't say what it holds: one with no README.md or
# AGENTS.md, no entry file (a module, its checks, a package, a crate's root or
# a file named after the folder) opening with a comment, no file beside it of
# its name doing the same (backports.nix for backports/), and, at the top, no
# row in AGENTS.md's map. A folder named tests holds the tests of the folder
# it's in, so it says what it holds when that folder does. A folder still waiting for its note may be
# named in the waiting list, which fails in turn once the folder has its note
# or is gone, so the list only shrinks.
# usage: every-folder-says-what-it-holds.sh <tree> <waiting list, a path a line>
set -euo pipefail

fail() {
  echo "$*" >&2
  exit 1
}

tree=$1
waiting=$2
[ -d "$tree" ] || fail "no tree at $tree"
[ -f "$waiting" ] || fail "no waiting list at $waiting"

entries=(home.nix nixos.nix darwin.nix system.nix checks.nix package.nix main.rs lib.rs mod.rs)

# Whether a folder says what it holds.
says() {
  local folder=$1 entry first
  [ -f "$folder/README.md" ] || [ -f "$folder/AGENTS.md" ] && return 0
  case $folder in
    */tests) says "${folder%/tests}" && return 0 ;;
    */*) ;;
    *) grep -qF "| \`$folder/" AGENTS.md && return 0 ;;
  esac
  for entry in "${entries[@]/#/$folder/}" "$folder/${folder##*/}.nix" "$folder.nix"; do
    [ -f "$entry" ] || continue
    first=$(head -n 1 "$entry")
    case $first in
      '#'* | '//'*) return 0 ;;
    esac
  done
  return 1
}

cd "$tree"
silent=()
while IFS= read -r folder; do
  folder=${folder#./}
  says "$folder" && continue
  grep -qxF "$folder" "$waiting" || silent+=("$folder")
done < <(find . -mindepth 1 -type d -not -path '*/.*' | sort)

spoken=()
while IFS= read -r folder; do
  [ -n "$folder" ] || continue
  if [ ! -d "$folder" ] || says "$folder"; then
    spoken+=("$folder")
  fi
done < "$waiting"

if [ "${#silent[@]}" -gt 0 ]; then
  printf '%s\n' "These folders don't say what they hold: give each a README.md, or an" \
    "entry file (home.nix, nixos.nix, checks.nix, package.nix, main.rs, lib.rs" \
    "and the like) whose first line is a comment saying what goes in it:" "${silent[@]/#/  }" >&2
fi
if [ "${#spoken[@]}" -gt 0 ]; then
  printf '%s\n' "These folders are waiting for a note they now have, or are gone; take" \
    "them out of the waiting list in checks.nix:" "${spoken[@]/#/  }" >&2
fi
[ "${#silent[@]}" -eq 0 ] && [ "${#spoken[@]}" -eq 0 ] || exit 1
