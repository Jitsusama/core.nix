# shellcheck shell=bash
# Fails on a folder that doesn't say what it holds: one with no README.md or
# AGENTS.md, no entry file (a module, its checks, a package or its
# default.nix, a crate's manifest or root, or a file named after the folder)
# opening with a comment, no file beside it of its name doing the same
# (backports.nix for backports/), no line of its own in its parent's README.md
# or AGENTS.md opening with its name, and, at the top, no line of the map in
# README.md or AGENTS.md, a table's row or a tree's line with a comment. A
# folder named tests holds the tests of the folder it's in, so it says what it
# holds when that folder does. A crate's src/bin is Cargo's folder of the
# crate's other programs, so it says what it holds when each of them opens
# with a comment saying what it is. A folder still waiting for its note may be
# named in the waiting list, which fails in turn once the folder has its note
# or is gone, so the list only shrinks. Core runs it on itself, and gives it to
# the repositories built on it as `core.lib.everyFolderSaysWhatItHolds`.
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

# default.nix is what Nix itself opens for a folder, as callPackage ./folder
# does, so a package that builds from its whole folder says what it is there
# without a README that would change what it builds.
entries=(home.nix nixos.nix darwin.nix system.nix checks.nix package.nix default.nix Cargo.toml
  main.rs lib.rs mod.rs)

# Whether every program in a crate's src/bin opens with a comment.
programs_say() {
  local program
  for program in "$1"/*.rs; do
    [ -f "$program" ] || return 1
    case $(head -n 1 "$program") in
      '//'*) ;;
      *) return 1 ;;
    esac
  done
}

# Whether a folder's parent describes it where a reader of the parent already
# looks: a bullet or a paragraph of its README.md or AGENTS.md that opens with
# the folder's name, as `launcher/`, linked or not, and goes on to say more.
parent_says() {
  local name=${1##*/} note
  name=${name//./\\.}
  for note in "${1%/*}/README.md" "${1%/*}/AGENTS.md"; do
    [ -f "$note" ] || continue
    grep -qE "^(- )?\[?\`$name/\`(\]\[[^]]*\]|\]\([^)]*\))?[: ]" "$note" && return 0
  done
  return 1
}

# Whether the repository's map describes a top folder: a row of a table in
# README.md or AGENTS.md opening with its name, as core's does, or a line of a
# tree drawn in one, its name and then a comment, as `checks/   # the scripts`.
map_says() {
  local name=${1//./\\.} map
  for map in README.md AGENTS.md; do
    [ -f "$map" ] || continue
    grep -qE "^\| \`$name/|^$name/ +# " "$map" && return 0
  done
  return 1
}

# Whether a folder says what it holds.
says() {
  local folder=$1 entry first
  [ -f "$folder/README.md" ] || [ -f "$folder/AGENTS.md" ] && return 0
  case $folder in
    */tests) says "${folder%/tests}" && return 0 ;;
    */src/bin) programs_say "$folder" && return 0 ;;
    */*) parent_says "$folder" && return 0 ;;
    *) map_says "$folder" && return 0 ;;
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
    "and the like) whose first line is a comment saying what goes in it, or a" \
    "line in its parent's README.md opening with its name, as - \`launcher/\`: ...," \
    "or, for a top folder, a line of the map in README.md, as checks/   # ..." \
    "${silent[@]/#/  }" >&2
fi
if [ "${#spoken[@]}" -gt 0 ]; then
  printf '%s\n' "These folders are waiting for a note they now have, or are gone; take" \
    "them out of the waiting list in checks.nix:" "${spoken[@]/#/  }" >&2
fi
[ "${#silent[@]}" -eq 0 ] && [ "${#spoken[@]}" -eq 0 ] || exit 1
