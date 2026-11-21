# shellcheck shell=bash
# Checks that a search of a home folder holding the Mac's ignore file never
# opens other programs' data or the folders macOS guards, with ripgrep or fd,
# started from the home folder or from Library, or given either's path from
# elsewhere as an agent's tool does, and still finds everything else, folders
# in a project named like a guarded one included. Each guarded folder is made
# unreadable, so opening it is an error the program reports. Without the
# file, each search reports them, so the check can fail. A search naming a
# guarded folder, or starting inside one, still searches it.
# usage: searches-leave-other-programs-data-alone.sh <ignore file> <rg> <fd>
set -euo pipefail

ignore=$1
rg=$2
fd=$3
# What the searches say is kept beside the home folder, never in it, so no
# search reads it.
work=$(mktemp -d)
home=$work/home
said=$work/said
trap 'chmod -R u+rwx "$work"; rm -rf "$work"' EXIT
guarded=(
  "Library/Containers/com.example.app"
  "Library/Group Containers/group.example"
  "Library/Mobile Documents/com~apple~CloudDocs"
  "Library/CloudStorage/a-cloud"
  "Desktop"
  "Documents/repo"
  "Downloads"
)
for folder in "${guarded[@]}" Library/Logs src src/Desktop src/Documents src/Downloads; do
  mkdir -p "$home/$folder"
  printf 'needle\n' > "$home/$folder/needle"
done
locked=(
  "$home/Library/Containers" "$home/Library/Group Containers"
  "$home/Library/Mobile Documents" "$home/Library/CloudStorage"
  "$home/Desktop" "$home/Documents" "$home/Downloads"
)
programs=("$rg" "$fd")
starts=("$home" "$home/Library")
# How a search reaches its start: run inside it, or given its path from
# elsewhere.
hows=(inside elsewhere)
how=inside

# One program's search for the needle from a folder, of the paths given or
# of everything under it, reached as $how says: what it finds on stdout,
# relative to that folder, and what it says in $said. fd keeps quiet about
# a folder it can't open unless told to show errors.
search() {
  local program=$1 from=$2
  shift 2
  local at=${from} out=""
  local paths=("$@")
  if [ "${how}" = elsewhere ] && [ $# -eq 0 ]; then
    at=${work}
    paths=("${from}")
  fi
  if [ "${program}" = "${rg}" ]; then
    out=$( (cd "${at}" && "${rg}" --no-config --files-with-matches needle "${paths[@]}") \
      2>"${said}") || true
  else
    out=$( (cd "${at}" && "${fd}" --no-global-ignore-file --show-errors needle "${paths[@]}") \
      2>"${said}") || true
  fi
  printf '%s\n' "${out//"${from}/"/}"
}

# What a search from a start finds when it finds everything it should.
expected() {
  if [ "$1" = "${home}" ]; then
    printf '%s\n' Library/Logs/needle src/needle src/Desktop/needle src/Documents/needle \
      src/Downloads/needle
  else
    printf '%s\n' Logs/needle
  fi
}

# The name a failure gives a search, so it says which went wrong.
named() {
  who="${1##*/} from ${2#"${work}"/} (${how})"
}

chmod 000 "${locked[@]}"
for how in "${hows[@]}"; do
  for program in "${programs[@]}"; do
    for from in "${starts[@]}"; do
      named "${program}" "${from}"
      search "${program}" "${from}" >/dev/null
      if ! grep -q 'ermission denied' "${said}"; then
        echo "${who} with no ignore file opened no guarded folder, so this check can't fail" >&2
        exit 1
      fi
    done
  done
done

cp "${ignore}" "${home}/.ignore"
for how in "${hows[@]}"; do
  for program in "${programs[@]}"; do
    for from in "${starts[@]}"; do
      named "${program}" "${from}"
      found=$(search "${program}" "${from}")
      if [ -s "${said}" ]; then
        words=$(<"${said}")
        echo "${who} opened other programs' data or a folder macOS guards: ${words}" >&2
        exit 1
      fi
      wanted=$(expected "${from}")
      while read -r path; do
        if ! grep -qxF "${path}" <<<"${found}"; then
          echo "${who} no longer finds ${path}: ${found}" >&2
          exit 1
        fi
      done <<<"${wanted}"
    done
  done
done

# Readable again, the guarded folders are still never listed, since the file
# keeps the searches out rather than the locks.
chmod 755 "${locked[@]}"
for how in "${hows[@]}"; do
  for program in "${programs[@]}"; do
    for from in "${starts[@]}"; do
      named "${program}" "${from}"
      found=$(search "${program}" "${from}")
      if grep -qE -e '^(Library/)?(Containers|Group Containers|Mobile Documents|CloudStorage)/' \
        -e '^(Desktop|Documents|Downloads)/' <<<"${found}"; then
        echo "${who} listed other programs' data or a folder macOS guards: ${found}" >&2
        exit 1
      fi
    done
  done
done

# A search naming a guarded folder, or starting inside one, as in a
# repository kept in Documents, still searches it.
how=inside
for program in "${programs[@]}"; do
  named "${program}" "${home}"
  found=$(search "${program}" "${home}" Documents)
  if ! grep -qxF Documents/repo/needle <<<"${found}"; then
    echo "${who} naming Documents no longer searched it: ${found}" >&2
    exit 1
  fi
  named "${program}" "${home}/Documents/repo"
  found=$(search "${program}" "${home}/Documents/repo")
  if ! grep -qxF needle <<<"${found}"; then
    echo "${who} no longer searched it: ${found}" >&2
    exit 1
  fi
done
