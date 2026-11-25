# shellcheck shell=bash
# A Mac's find, which runs macOS's own but leaves out the folders the home
# folder's ignore file guards, other programs' data and the folders macOS asked
# about, when a search starts above one. Opening one makes macOS ask whether
# the searcher may read it, and an agent's find of the home folder asked over
# and over, since ripgrep and fd read the ignore file and find never does.
# A folder left out is still tested and listed, as find lists it from its
# parent without opening it, unless the expression would open it (-empty, or
# an -exec, -execdir, -ok or -okdir given it), when it isn't listed at all.
# Either way find says on stderr that it left folders unsearched, or left them
# out, so an empty answer needn't read as nothing there, though a search
# sending stderr to /dev/null drops that too. A folder a -maxdepth keeps find
# from opening is left as find leaves it, with no note. A folder listed can
# still be opened by what its answer is piped to.
# A search naming a guarded folder, or starting inside one, still searches it,
# as with ripgrep and fd. A search find can't prune runs as given: one going
# depth first (-d or -depth alone, or -delete), one whose -mindepth lies past a
# guarded folder, and one with -f joined to another option or to its path
# (-xf, -f/path). Nor does it reach /usr/bin/find run by its path, or another
# find ahead of it on PATH, as a dev shell's is. One following links (-L)
# leaves out only the folders it reaches by their own names, a search of /
# guards only the home folder's, not what /Volumes mounts, and a session with
# HOME moved guards the folders of the home it names.
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

# What find says when it left a folder unsearched, so an answer missing what's
# in one isn't read as whole.
note='find: left the folders macOS asks about unsearched'
unlisted='find: left out the folders macOS asks about, unsearched and unlisted'

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
# guarded folder there is never pruned. A command an -exec runs is its own
# words, read no further than its ; or {} +, so its -d is never find's. -empty
# reads a folder to test it, and a command given a folder may open it, so
# either opens a guarded folder it's run on. A primary's operand, such as
# -name's pattern, is its own word, never a primary, so -name -empty opens
# nothing. find opens no folder at its -maxdepth, and reaches none past it.
acts=false
opens=false
mindepth=0
maxdepth=''
# A depth as macOS's find reads it, by atoi (c_mXXdepth in its
# function.c): past blanks and a +, its leading digits, or 0 with none, so
# ` 2`, `+2` and `2x` are each 2, as find takes them, not left unread.
depth_of() {
  local word=$1 digits
  word=${word#"${word%%[![:space:]]*}"}
  word=${word#+}
  digits=${word%%[!0-9]*}
  depth=$((10#${digits:-0}))
}
for ((at = 0; at < ${#expression[@]}; at++)); do
  arg=${expression[at]}
  next=${expression[at + 1]-}
  case ${arg} in
    -depth) [[ ${next} =~ ^[-+]?[0-9]+$ ]] || depth=true ;;
    -d | -delete) depth=true ;;
    -mindepth)
      depth_of "${next}"
      mindepth=${depth}
      at=$((at + 1))
      ;;
    -maxdepth)
      depth_of "${next}"
      maxdepth=${depth}
      at=$((at + 1))
      ;;
    -name | -iname | -path | -ipath | -wholename | -iwholename | -regex | -iregex | -lname \
      | -ilname | -type | -size | -perm | -user | -group | -uid | -gid | -links | -inum \
      | -samefile | -fstype | -flags | -xattrname | -newer | -anewer | -cnewer | -mnewer \
      | -Bnewer | -newer[aBcmt][aBcmt] | -[aBcm]time | -[aBcm]min)
      at=$((at + 1))
      ;;
    -empty) opens=true ;;
    -exec | -execdir | -ok | -okdir)
      acts=true
      opens=true
      at=$((at + 1))
      while [ "${at}" -lt "${#expression[@]}" ]; do
        word=${expression[at]}
        [ "${word}" = ';' ] && break
        [ "${word}" = + ] && [ "${expression[at - 1]}" = '{}' ] && break
        at=$((at + 1))
      done
      ;;
    -print | -print0 | -ls) acts=true ;;
    -printf | -fprint | -fprint0 | -fls)
      acts=true
      at=$((at + 1))
      ;;
    -fprintf)
      acts=true
      at=$((at + 2))
      ;;
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
# A last line with no newline is read too, as ripgrep reads it.
while IFS= read -r line || [ -n "${line}" ]; do
  [[ ${line} =~ ^/[^*?![]+/$ ]] || continue
  [ -n "${home}" ] || continue
  line=${line#/}
  targets+=("${home}/${line%/}")
  if [ -d "${data}${home}" ]; then
    targets+=("${data}${home}/${line%/}")
  fi
done <"${ignore}"
# A Mac's disk takes a name in any case, and cd keeps the case typed, so
# paths are matched whatever their case.
reals=()
declare -A starting=()
for path in ${starts[@]+"${starts[@]}"}; do
  real=$(CDPATH='' cd -P -- "${path}" 2>/dev/null && pwd -P) || real=""
  reals+=("${real}")
  lowered=${real,,}
  starting[${lowered:-.}]=yes
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
    if [ "${real}" = / ]; then
      rest=${target#/}
    elif [[ ${target,,} == "${real,,}"/* ]]; then
      rest=${target:${#real}+1}
    else
      continue
    fi
    [ -n "${starting[${target,,}]-}" ] && continue
    slashes=${rest//[^\/]/}
    deep=$((${#slashes} + 1))
    if [ -n "${maxdepth}" ] && { [ "${deep}" -gt "${maxdepth}" ] \
      || { [ "${deep}" -eq "${maxdepth}" ] && [ "${opens}" = false ]; }; }; then
      continue
    fi
    if [ "${deep}" -lt "${mindepth}" ]; then
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
if [ "${opens}" = true ]; then
  echo "${unlisted}" >&2
else
  echo "${note}" >&2
fi
# What the search printed by itself, it still prints: an expression with no
# action of its own is given -print, as find gives it. A pruned folder is
# still tested and listed, once, without find opening it, unless testing it
# would open it. -prune stands for true, as POSIX makes it, since macOS's
# find may lack -true.
shown=(-print)
if [ "${acts}" = true ]; then
  shown=()
fi
tested=()
if [ ${#expression[@]} -gt 0 ]; then
  tested=('(' "${expression[@]}" ')')
fi
listed=()
if [ "${opens}" = false ] && [ ${#tested[@]} -gt 0 ]; then
  listed=('(' "${tested[@]}" ${shown[@]+"${shown[@]}"} -o -prune ')')
elif [ "${opens}" = false ]; then
  listed=(${shown[@]+"${shown[@]}"})
fi
exec "${find}" ${options[@]+"${options[@]}"} ${paths[@]+"${paths[@]}"} \
  '(' "${prunes[@]}" ')' -prune ${listed[@]+"${listed[@]}"} \
  -o ${tested[@]+"${tested[@]}"} ${shown[@]+"${shown[@]}"}
