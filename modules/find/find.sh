# shellcheck shell=bash
# A Mac's find, which runs macOS's own but leaves out the folders the home
# folder's ignore file guards, other programs' data and the folders macOS asked
# about, when a search starts above one. Opening one makes macOS ask whether
# the searcher may read it, and an agent's find of the home folder asked over
# and over, since ripgrep and fd read the ignore file and find never does.
# A search naming a guarded folder, or starting inside one, still searches it,
# as with ripgrep and fd. A search going depth first (-d, -depth or -delete)
# runs as given, since find can't prune one, and so does /usr/bin/find run by
# its path.
# usage: find.sh <find> <ignore file> [find's arguments...]
set -euo pipefail

find=$1
ignore=$2
shift 2

# macOS's find takes no --help, so answering it here hides nothing of find's.
if [ "${1-}" = --help ]; then
  cat <<'EOF'
find  macOS's find, leaving guarded folders alone

  Runs macOS's find, but a search starting above a
  folder the home folder's .ignore guards leaves it
  out. A search naming one, or starting in one,
  searches it; a depth-first one runs as given.

EXAMPLES
  find ~ -name '*.nix'
  find ~/Documents -name report.pdf

EXIT
  find's own
EOF
  exit 0
fi
args=("$@")

# find's options come first, then the paths, then the expression. Only the
# options macOS's find takes are read: -f takes a path of its own.
options=()
depth=false
at=0
while [ "${at}" -lt "${#args[@]}" ]; do
  arg=${args[at]}
  if [ "${arg}" = -f ] && [ $((at + 1)) -lt "${#args[@]}" ]; then
    options+=("${arg}" "${args[at + 1]}")
    at=$((at + 2))
  elif [[ ${arg} =~ ^-[EHLPXdsx]+$ ]]; then
    [[ ${arg} == *d* ]] && depth=true
    options+=("${arg}")
    at=$((at + 1))
  else
    break
  fi
done
paths=()
while [ "${at}" -lt "${#args[@]}" ]; do
  arg=${args[at]}
  case ${arg} in
    -?* | '(' | ')' | '!') break ;;
    *) ;;
  esac
  paths+=("${arg}")
  at=$((at + 1))
done
expression=("${args[@]:at}")

acts=false
for arg in ${expression[@]+"${expression[@]}"}; do
  case ${arg} in
    -depth | -delete) depth=true ;;
    -print | -print0 | -printf | -fprint | -fprint0 | -fprintf | -ls | -fls | -exec | -execdir \
      | -ok | -okdir | -quit) acts=true ;;
    *) ;;
  esac
done

# A path as find matches it, with -path's own wildcards taken literally.
literal() {
  local path=$1
  path=${path//\\/\\\\}
  path=${path//\*/\\*}
  path=${path//\?/\\?}
  path=${path//\[/\\[}
  printf '%s' "${path}"
}

# Each guarded folder under a start path, named as find names it from there:
# the start path as given, then the rest of the way. A start path ending in a
# slash is named both ways, since macOS's find adds another and Linux's
# doesn't.
home=$(cd "${HOME}" 2>/dev/null && pwd -P) || home=""
guarded=()
while IFS= read -r line; do
  case ${line} in
    '' | '#'*) continue ;;
    *) ;;
  esac
  line=${line#/}
  guarded+=("${line%/}")
done <"${ignore}"
prunes=()
if [ -n "${home}" ] && [ "${depth}" = false ]; then
  for path in ${paths[@]+"${paths[@]}"}; do
    real=$(cd "${path}" 2>/dev/null && pwd -P) || continue
    for folder in "${guarded[@]}"; do
      target=${home}/${folder}
      [ "${real}" = / ] && rest=${target#/} || rest=${target#"${real}"/}
      [ "${rest}" = "${target}" ] && continue
      [ ${#prunes[@]} -gt 0 ] && prunes+=(-o)
      prunes+=(-path "$(literal "${path}/${rest}")")
      if [ "${path}" != "${path%/}" ]; then
        prunes+=(-o -path "$(literal "${path}${rest}")")
      fi
    done
  done
fi

if [ ${#prunes[@]} -eq 0 ]; then
  exec "${find}" "$@"
fi
# What the search printed by itself, it still prints: an expression with no
# action of its own is given -print, as find gives it.
shown=(-print)
[ "${acts}" = true ] && shown=()
if [ ${#expression[@]} -eq 0 ]; then
  exec "${find}" ${options[@]+"${options[@]}"} ${paths[@]+"${paths[@]}"} \
    '(' "${prunes[@]}" ')' -prune -o -print
fi
exec "${find}" ${options[@]+"${options[@]}"} ${paths[@]+"${paths[@]}"} \
  '(' "${prunes[@]}" ')' -prune -o '(' "${expression[@]}" ')' ${shown[@]+"${shown[@]}"}
