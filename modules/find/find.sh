# shellcheck shell=bash
# A Mac's find, which runs macOS's own but leaves out the folders the home
# folder's ignore file guards, other programs' data and the folders macOS asked
# about, when a search starts above one. Opening one makes macOS ask whether
# the searcher may read it, and an agent's find of the home folder asked over
# and over, since ripgrep and fd read the ignore file and find never does.
# A folder left out is still listed, as find lists it from its parent without
# opening it, and find says on stderr that it left folders out, so an empty
# answer never reads as nothing there.
# A search naming a guarded folder, or starting inside one, still searches it,
# as with ripgrep and fd. A search find can't prune runs as given: one going
# depth first (-d or -depth alone, or -delete), one whose -mindepth lies past a
# guarded folder, one with options spelled other than apart (-xf, -f/path), and
# /usr/bin/find run by its path. One following links (-L) leaves out only the
# folders it reaches by their own names.
# usage: find.sh <find> <ignore file> <data volume> [find's arguments...]
set -euo pipefail

find=$1
ignore=$2
# Where macOS mounts the volume the home folder lives on, which reaches the
# home folder by a second name, so a search of / walks it twice.
data=$3
shift 3

# macOS's find takes no --help, so answering it here hides nothing of find's.
if [ "${1-}" = --help ]; then
  cat <<'EOF'
find  macOS's find, leaving guarded folders alone

  Runs macOS's find, but a search starting above a
  folder the home folder's .ignore guards lists it
  without searching it, and says so on stderr. A
  search naming one, or starting in one, searches
  it; one find can't prune runs as given.

EXAMPLES
  find ~ -name '*.nix'
  find ~/Documents -name report.pdf

EXIT
  find's own
EOF
  exit 0
fi
args=("$@")

# What find says when it left a folder out, so the searcher knows to name it.
note='find: left out the folders macOS asks about; name one to search it'

# find's options come first, then the paths, then the expression. Only the
# options macOS's find takes are read, each apart: -f takes a path of its own,
# a start like any other, and -- ends them.
options=()
starts=()
depth=false
at=0
while [ "${at}" -lt "${#args[@]}" ]; do
  arg=${args[at]}
  if [ "${arg}" = -f ] && [ $((at + 1)) -lt "${#args[@]}" ]; then
    options+=("${arg}" "${args[at + 1]}")
    starts+=("${args[at + 1]}")
    at=$((at + 2))
  elif [ "${arg}" = -- ]; then
    options+=("${arg}")
    at=$((at + 1))
    break
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
starts+=(${paths[@]+"${paths[@]}"})

# macOS's -depth followed by a number tests a file's depth and walks as ever;
# alone, as -d, it goes depth first. find tests nothing above -mindepth, so a
# guarded folder there is never pruned.
acts=false
mindepth=0
for ((at = 0; at < ${#expression[@]}; at++)); do
  arg=${expression[at]}
  next=${expression[at + 1]-}
  case ${arg} in
    -depth) [[ ${next} =~ ^[-+]?[0-9]+$ ]] || depth=true ;;
    -d | -delete) depth=true ;;
    -mindepth)
      if [[ ${next} =~ ^[0-9]+$ ]]; then
        mindepth=${next}
      fi
      ;;
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

# The guarded folders, by both the home folder's names. Only a line naming one
# folder from the home folder's top is read: ripgrep reads a pattern or a
# negation as a pattern, which find can't, and checks.nix holds every line to it.
# Each folder is taken by its real path, as find opens it, never a folder
# CDPATH names instead.
home=$(CDPATH='' cd -P -- "${HOME}" 2>/dev/null && pwd -P) || home=""
targets=()
while IFS= read -r line; do
  [[ ${line} =~ ^/[^*?![]+/$ ]] || continue
  [ -n "${home}" ] || continue
  line=${line#/}
  targets+=("${home}/${line%/}")
  if [ -d "${data}${home}" ]; then
    targets+=("${data}${home}/${line%/}")
  fi
done <"${ignore}"
reals=()
declare -A starting=()
for path in ${starts[@]+"${starts[@]}"}; do
  real=$(CDPATH='' cd -P -- "${path}" 2>/dev/null && pwd -P) || real=""
  reals+=("${real}")
  starting[${real:-.}]=yes
done

# Each guarded folder under a start path, named as find names it from there:
# the start path as given, then the rest of the way. A start path ending in a
# slash is named both ways, since macOS's find adds another and Linux's
# doesn't. A folder that is itself a start isn't pruned, since find would
# prune that start too.
prunes=()
pruned=true
if [ "${depth}" = true ]; then
  pruned=false
fi
for ((at = 0; at < ${#starts[@]}; at++)); do
  path=${starts[at]}
  real=${reals[at]}
  [ -n "${real}" ] || continue
  for target in ${targets[@]+"${targets[@]}"}; do
    [ "${real}" = / ] && rest=${target#/} || rest=${target#"${real}"/}
    [ "${rest}" = "${target}" ] && continue
    [ -n "${starting[${target}]-}" ] && continue
    slashes=${rest//[^\/]/}
    if [ $((${#slashes} + 1)) -lt "${mindepth}" ]; then
      pruned=false
    fi
    [ ${#prunes[@]} -gt 0 ] && prunes+=(-o)
    prunes+=(-path "$(literal "${path}/${rest}")")
    if [ "${path}" != "${path%/}" ]; then
      prunes+=(-o -path "$(literal "${path}${rest}")")
    fi
  done
done

if [ ${#prunes[@]} -eq 0 ] || [ "${pruned}" = false ]; then
  exec "${find}" "$@"
fi
echo "${note}" >&2
# What the search printed by itself, it still prints: an expression with no
# action of its own is given -print, as find gives it. A pruned folder is
# still tested and listed, once, without find opening it.
shown=(-print)
if [ "${acts}" = true ]; then
  shown=()
fi
if [ ${#expression[@]} -eq 0 ]; then
  expression=(-true)
fi
exec "${find}" ${options[@]+"${options[@]}"} ${paths[@]+"${paths[@]}"} \
  '(' "${prunes[@]}" ')' -prune '(' '(' "${expression[@]}" ')' ${shown[@]+"${shown[@]}"} \
  -o -true ')' -o '(' "${expression[@]}" ')' ${shown[@]+"${shown[@]}"}
