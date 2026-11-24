# shellcheck shell=bash
# Checks that the Mac's find, searching a home folder from the home folder or
# from Library, run there or given either's path from elsewhere, never opens
# the folders the ignore file guards, and still finds everything else, folders
# in a project named like a guarded one included. Each guarded folder is made
# unreadable, so opening it is an error find reports. The find it runs, run
# alone, reports them, so the check can fail. A search naming a guarded
# folder, or starting inside one, still searches it; an expression with an
# action of its own keeps it; and a search going depth first runs as given.
# The home folder's name holds find's wildcards, which must match only
# themselves.
# usage: find-leaves-guarded-folders-alone.sh <find.sh> <ignore file> <find>
set -euo pipefail

wrapper=$1
ignore=$2
real=$3
# What the searches say is kept beside the home folder, never in it, so no
# search reads it.
work=$(mktemp -d)
home="${work}/a [home]*"
said=${work}/said
trap 'chmod -R u+rwx "${work}"; rm -rf "${work}"' EXIT
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
  mkdir -p "${home}/${folder}"
  printf 'needle\n' >"${home}/${folder}/needle"
done
locked=(
  "${home}/Library/Containers" "${home}/Library/Group Containers"
  "${home}/Library/Mobile Documents" "${home}/Library/CloudStorage"
  "${home}/Desktop" "${home}/Documents" "${home}/Downloads"
)
starts=("${home}" "${home}/Library")
# How a search reaches its start: run inside it as ., given its path from
# elsewhere, or given it with a slash at its end.
hows=(inside elsewhere slashed)
how=inside
program=${wrapper}

# A search for the needle from a folder, of the paths given or of all of it,
# reached as $how says, with the expression given or -name needle: what it
# finds on stdout, relative to that folder, and what it says in $said.
search() {
  local from=$1
  shift
  local at=${from} paths=(.)
  case ${how} in
    elsewhere) at=${work} paths=("${from}") ;;
    slashed) at=${work} paths=("${from}/") ;;
    *) ;;
  esac
  local expression=(-name needle)
  if [ $# -gt 0 ]; then
    expression=("$@")
  fi
  local out
  if [ "${program}" = "${wrapper}" ]; then
    out=$( (cd "${at}" && HOME=${home} bash "${wrapper}" "${real}" "${ignore}" \
      "${paths[@]}" "${expression[@]}") 2>"${said}") || true
  else
    out=$( (cd "${at}" && "${real}" "${paths[@]}" "${expression[@]}") 2>"${said}") || true
  fi
  out=${out//"${from}//"/}
  out=${out//"${from}/"/}
  printf '%s\n' "${out//.\//}"
}

# What a search from each start finds when it finds everything it should.
declare -A expected=(
  ["${home}"]=$'Library/Logs/needle\nsrc/needle\nsrc/Desktop/needle\nsrc/Documents/needle\nsrc/Downloads/needle'
  ["${home}/Library"]=Logs/needle
)

# The name a failure gives a search, so it says which went wrong.
named() {
  who="${program##*/} from ${1#"${work}"/} (${how})"
}

chmod 000 "${locked[@]}"
program=${real}
for how in "${hows[@]}"; do
  for from in "${starts[@]}"; do
    named "${from}"
    search "${from}" >/dev/null
    if ! grep -q 'ermission denied' "${said}"; then
      echo "${who} opened no guarded folder, so this check can't fail" >&2
      exit 1
    fi
  done
done

program=${wrapper}
for how in "${hows[@]}"; do
  for from in "${starts[@]}"; do
    named "${from}"
    found=$(search "${from}")
    if [ -s "${said}" ]; then
      words=$(<"${said}")
      echo "${who} opened other programs' data or a folder macOS guards: ${words}" >&2
      exit 1
    fi
    wanted=${expected[${from}]}
    while read -r path; do
      if ! grep -qxF "${path}" <<<"${found}"; then
        echo "${who} no longer finds ${path}: ${found}" >&2
        exit 1
      fi
    done <<<"${wanted}"
  done
done

# An expression with an action of its own keeps it, and prints nothing more;
# one with none, or none at all, still prints what it matches.
how=inside
named "${home}"
found=$(search "${home}" -name needle -exec printf 'seen %s\n' '{}' ';')
if [ -s "${said}" ] || ! grep -qxF 'seen Library/Logs/needle' <<<"${found}" \
  || grep -qxF Library/Logs/needle <<<"${found}"; then
  echo "${who} with -exec lost its action or printed more: ${found} $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" -type f -o -name nothing)
if [ -s "${said}" ] || ! grep -qxF Library/Logs/needle <<<"${found}"; then
  echo "${who} with an expression joined by -o lost a match: ${found} $(<"${said}")" >&2
  exit 1
fi

# Readable again, the guarded folders are still never listed, since the
# search leaves them out rather than the locks.
chmod 755 "${locked[@]}"
for how in "${hows[@]}"; do
  for from in "${starts[@]}"; do
    named "${from}"
    found=$(search "${from}")
    if grep -qE -e '^(Library/)?(Containers|Group Containers|Mobile Documents|CloudStorage)/' \
      -e '^(Desktop|Documents|Downloads)/' <<<"${found}"; then
      echo "${who} listed other programs' data or a folder macOS guards: ${found}" >&2
      exit 1
    fi
  done
done

# A search naming a guarded folder, or starting inside one, as in a
# repository kept in Documents, still searches it.
how=inside
named "${home}"
found=$( (cd "${home}" && HOME=${home} bash "${wrapper}" "${real}" "${ignore}" Documents \
  -name needle) 2>"${said}") || true
if ! grep -qxF Documents/repo/needle <<<"${found}"; then
  echo "${who} naming Documents no longer searched it: ${found}" >&2
  exit 1
fi
named "${home}/Documents/repo"
found=$(search "${home}/Documents/repo")
if ! grep -qxF needle <<<"${found}"; then
  echo "${who} no longer searched it: ${found}" >&2
  exit 1
fi

# A search going depth first runs as given, since find can't prune it.
named "${home}"
found=$(search "${home}" -depth -name needle)
if ! grep -qxF Documents/repo/needle <<<"${found}"; then
  echo "${who} with -depth didn't run as given: ${found}" >&2
  exit 1
fi
