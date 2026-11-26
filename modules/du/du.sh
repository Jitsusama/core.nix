# shellcheck shell=bash
# A Mac's du, which runs macOS's own but never walks into the folders the
# home folder's ignore file guards, other programs' data and the folders
# macOS asks about. Opening one makes macOS ask whether the counter may read
# it, and an agent's du of the home folder on 24 November walked into every
# one of them, since du reads no ignore file.
# A start that is a guarded folder, lies in one or passes through one on
# its way, as `~/Downloads/../src` does, is left out of the count: judged
# first by its path as typed, empty steps and `.` dropped and `..` taking
# back a step, before anything opens it, then by its real path. Naming one
# is how a shell's glob asks too: `du -sh ~/*` names Desktop, Documents and
# Downloads. A start above one leaves out, by -I, every folder of that name
# beneath it, since macOS's du leaves a folder out by its name alone, not
# its path: a project's own Documents goes too. A count asked to follow
# links (-L) follows only those it was named, as -H does, since macOS's du
# matches -I against the name a link has, not the folder it reaches
# (ignorep in its du.c), so a link beneath named otherwise would walk into
# one. Each is said on
# stderr, and du then ends 1, so a smaller total never reads as the whole.
# Neither names a way round, since an agent reads a named way as the next
# thing to run.
# The home folder guarded is the account's, as its configuration names it,
# and the one HOME names when that's elsewhere, each as named and by its
# real path. A guarded folder reached through a link in a start's path, or
# in the path a link leads to, is resolved by bash, which looks inside it
# once; a file is judged by its folder's real path, as is a start that is a
# link, which macOS's du counts as the link alone unless -H, or a / after
# it, follows it. One followed is judged by where its links lead, each read
# as written and judged as typed before it's touched. A folder that can't
# be entered, or a link whose end can't be read, can't be judged, so it's
# left out and said apart.
# Another du ahead of this one on PATH, as a dev shell's is, isn't reached,
# and folders outside the home folder, as on /Volumes, aren't guarded.
# usage: du.sh <du> <ignore file> <data volume> <home> [du's arguments...]
set -euo pipefail

du=$1
ignore=$2
# Where macOS mounts the volume the home folder lives on, which reaches the
# home folder by a second name, so a count of / walks it twice.
data=$3
account=$4
shift 4

# macOS's du takes no --help, so answering it here hides nothing of du's.
if [ "${1-}" = --help ]; then
  cat <<'EOF'
du  macOS's du, leaving guarded folders alone

  Runs macOS's du, but never walks into a folder
  the home folder's .ignore guards: a start that
  is one, lies in one or passes through one is
  left out, a count starting above one leaves out
  every folder of that name, and -L follows only
  the links named, as -H. Each is said on stderr.

EXAMPLES
  du -sh ~
  du -sh ~/*

EXIT
  du's own, or 1 when anything was left out or
  -L taken as -H
EOF
  exit 0
fi
args=("$@")

# du's options come first, then the paths: macOS's du reads options only up
# to the first path. Only the options it takes are read: -B, -I, -d and -t
# take a value, joined to them or as the next word, and -- ends them. Of -H,
# -L and -P the last given wins, as in du; -L is taken out, wherever it
# sits among a word's letters, and when it would have won, -H is given
# last in its place and said. -H is kept as `heads`, since it decides how a
# start that is a link is judged.
follows=""
heads=""
dashed=""
options=()
at=0
while [ "${at}" -lt "${#args[@]}" ]; do
  arg=${args[at]}
  at=$((at + 1))
  if [ "${arg}" = -- ]; then
    dashed=yes
    break
  elif [[ ${arg} == --?* ]]; then
    options+=("${arg}")
    continue
  elif [[ ${arg} =~ ^-[A-Za-z] ]]; then
    letters=${arg#-}
    before=${letters%%[BIdt]*}
    value=${letters:${#before}}
    kept_letters=""
    while [ -n "${before}" ]; do
      letter=${before:0:1}
      before=${before:1}
      case ${letter} in
        L)
          follows=yes
          continue
          ;;
        H)
          follows=""
          heads=yes
          ;;
        P)
          follows=""
          heads=""
          ;;
        *) ;;
      esac
      kept_letters+=${letter}
    done
    if [ -n "${kept_letters}${value}" ]; then
      options+=("-${kept_letters}${value}")
    fi
    if [[ ${letters} =~ ^[^BIdt]*[BIdt]$ ]] && [ "${at}" -lt "${#args[@]}" ]; then
      options+=("${args[at]}")
      at=$((at + 1))
    fi
  else
    at=$((at - 1))
    break
  fi
done
starts=("${args[@]:at}")
if [ -n "${follows}" ]; then
  options+=(-H)
  heads=yes
fi
# No path is du's ., judged as one but not named to du, as it wasn't typed.
unnamed=""
if [ ${#starts[@]} -eq 0 ]; then
  starts=(.)
  unnamed=yes
fi

# The working folder, which relative starts are judged from. A folder's
# real path is found by going there and back, without a subshell, since a
# count may name thousands of starts; when the working folder can't be
# gone back to, as when it was removed, in a subshell instead, so a start
# named by its whole path is still judged and counted.
origin=${PWD-}
if [ -z "${origin}" ]; then
  origin=$(pwd -P 2>/dev/null) || origin=""
fi
here=""
if [ -n "${origin}" ] && cd -- "${origin}" 2>/dev/null; then
  here=yes
fi
# A folder's real path, as du opens it; empty when it can't be reached. A
# relative path is gone to from `./`, so cd never takes one CDPATH names
# instead, nor the last folder, which it reads `-` as.
real() {
  local to=$1
  [[ ${to} == /* ]] || to=./${to}
  real=""
  if [ -z "${here}" ]; then
    real=$(cd -P -- "${to}" 2>/dev/null && pwd -P) || real=""
    return 0
  fi
  if cd -P -- "${to}" 2>/dev/null; then
    real=${PWD}
  fi
  if ! cd -- "${origin}" 2>/dev/null; then
    echo "note: du's working folder went away while its starts were judged," \
      "so it counted nothing" >&2
    exit 1
  fi
}

# Whether `plain` is a guarded folder or lies in one, as `inside`. A Mac's
# disk takes a name in any case, and cd keeps the case typed, so paths are
# matched in bash's lowercase, which folds ASCII and most letters, not all
# APFS folds.
judge() {
  local path=${plain,,} target
  for target in ${lowered[@]+"${lowered[@]}"}; do
    if [ "${path}" = "${target}" ] || [[ ${path} == "${target}"/* ]]; then
      inside=yes
    fi
  done
}
# Sets plain to the path folder $1 and then path $2 make, read as typed:
# empty steps and `.` dropped, `..` taking back a step, and / when nothing
# is left. Sets inside when a step $2 takes, or where it ends, is or lies
# in a guarded folder; $1's own steps are where the count already is.
plain() {
  local rest step judging=""
  plain=""
  inside=""
  for rest in "$1" "$2"; do
    rest=${rest}/
    while [ -n "${rest}" ]; do
      step=${rest%%/*}
      rest=${rest#*/}
      case ${step} in
        "" | .) ;;
        ..) plain=${plain%/*} ;;
        *)
          plain=${plain}/${step}
          if [ -n "${judging}" ]; then
            judge
          fi
          ;;
      esac
    done
    judging=yes
  done
  [ -n "${plain}" ] || plain=/
  judge
}

# The guarded folders, under each home folder that can be reached, as named
# and by its real path, and the data volume's second name for it. Only a
# line naming one folder from the home folder's top, /Name/, is read.
lowered=()
homes=()
for home in "${account}" "${HOME-}"; do
  [[ ${home} == /* ]] || continue
  real "${home}"
  [ -n "${real}" ] || continue
  plain "" "${home}"
  for one in "${plain}" "${real}"; do
    if [ -z "${one}" ] || [ "${one}" = / ]; then
      continue
    fi
    # Compared whole, since a home's name may hold spaces.
    known=""
    for had in ${homes[@]+"${homes[@]}"}; do
      [ "${had}" != "${one}" ] || known=yes
    done
    [ -n "${known}" ] || homes+=("${one}")
  done
done
targets=()
# A last line with no newline is read too, as ripgrep reads it.
while IFS= read -r line || [ -n "${line}" ]; do
  [[ ${line} =~ ^/[^*?![]+/$ ]] || continue
  line=${line#/}
  for home in ${homes[@]+"${homes[@]}"}; do
    # Only a folder there, so a project's own folder of that name isn't
    # left out for one this home doesn't have.
    [ -d "${home}/${line%/}" ] || continue
    targets+=("${home}/${line%/}")
    if [ -d "${data}${home}" ]; then
      targets+=("${data}${home}/${line%/}")
    fi
  done
done <"${ignore}"
for target in ${targets[@]+"${targets[@]}"}; do
  lowered+=("${target,,}")
done

# Each start a guarded folder holds, or that passes through one, is left
# out. Each guarded folder's name is kept once, for a start above it.
declare -A leaving=()
names=()
kept=()
left=()
unjudged=()
leave() {
  local name=${1##*/}
  [ -n "${leaving[${name}]-}" ] && return
  leaving[${name}]=yes
  names+=("${name}")
}
# Sets real to start `path`'s real path, by its folder's with its name,
# clearing judged when that folder can't be entered. One that isn't there
# is cleared too: a folder that can't be searched hides whether what's
# below it is there, and reads as missing all the same, so the two can't
# be told apart without going where a guarded folder may be.
by_folder() {
  local folder=.
  [[ ${path} != */* ]] || folder=${path%/*}
  real "${folder:-/}"
  [ -z "${real}" ] || real=${real%/}/${path##*/}
  [ -n "${real}" ] || judged=""
}
# Sets real to the real path of link $1, which du follows, by the links it
# leads through: each is read as written, from the real folder holding it,
# and judged as typed, as `inside`, before anything goes to where it leads,
# so a link into a guarded folder is left out without a look inside. It
# clears judged when a folder on the way can't be entered, or after 32
# links, where macOS gives up. Its own `path`, which by_folder reads, steps
# along the links, leaving the start's as typed for what's said of it.
by_end() {
  local path=$1 to links=0
  while :; do
    by_folder
    if [ -z "${judged}" ] || [ -z "${real}" ]; then
      return 0
    fi
    plain "" "${real}"
    [ -z "${inside}" ] || return 0
    [ -L "${real}" ] || break
    links=$((links + 1))
    # The . keeps a newline that ends the target, which $() would drop.
    if [ "${links}" -gt 32 ] || ! to=$(readlink -- "${real}" && echo .); then
      judged=""
      return 0
    fi
    to=${to%.}
    to=${to%$'\n'}
    # A / after it only says to follow it, as this does; kept, it would have
    # by_folder go into the link it ends.
    to=${to%"${to##*[!/]}"}
    to=${to:-/}
    [[ ${to} == /* ]] || to=${real%/*}/${to}
    plain "" "${to}"
    if [ -n "${inside}" ]; then
      real=${plain}
      return 0
    fi
    path=${to}
  done
  if [ -d "${real}" ]; then
    real "${real}"
    [ -n "${real}" ] || judged=""
  fi
}
for path in "${starts[@]}"; do
  if [[ ${path} == /* ]]; then
    plain "" "${path}"
  elif [ -n "${origin}" ]; then
    plain "${origin}" "${path}"
  else
    inside=""
  fi
  if [ -n "${inside}" ]; then
    left+=("${path}")
    continue
  fi
  # Then by its real path: a folder's own, and a file's by its folder's
  # with its name, as is a link's that du doesn't follow, since it counts
  # the link alone, and one -H or a / after it follows by where it leads. A
  # link is told before a folder, since asking whether it's a folder goes
  # where it leads. One that can't be judged, its folder not there or not
  # to be entered, is left out too, said apart; one alone not there in a
  # folder that is, is du's to say.
  real=""
  judged=yes
  bare=${path%"${path##*[!/]}"}
  if [ -L "${bare}" ] && [ -z "${heads}" ] && [ "${bare}" = "${path}" ]; then
    by_folder
  elif [ -L "${bare}" ]; then
    by_end "${bare}"
  elif [ -d "${path}" ]; then
    real "${path}"
    [ -n "${real}" ] || judged=""
  else
    by_folder
  fi
  if [ -z "${judged}" ]; then
    unjudged+=("${path}")
    continue
  fi
  if [ -n "${real}" ]; then
    plain "" "${real}"
    if [ -n "${inside}" ]; then
      left+=("${path}")
      continue
    fi
    for target in ${targets[@]+"${targets[@]}"}; do
      if [ "${real}" = / ] || [[ ${target,,} == "${real,,}"/* ]]; then
        leave "${target}"
      fi
    done
  fi
  kept+=("${path}")
done

# The words given, as one list, in `all`.
listed() {
  local one
  all=""
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
}
short=""
if [ ${#homes[@]} -eq 0 ]; then
  echo "note: du found no home folder to guard, so it guards nothing" >&2
elif [ ${#targets[@]} -eq 0 ]; then
  echo "note: du found no folder to guard in the home folder, so it guards nothing" >&2
fi
if [ ${#left[@]} -gt 0 ]; then
  listed "${left[@]}"
  echo "note: du left out ${#left[@]} start(s) in or through folders macOS asks about," \
    "uncounted: ${all}" >&2
  short=yes
fi
if [ ${#unjudged[@]} -gt 0 ]; then
  listed "${unjudged[@]}"
  echo "note: du left out ${#unjudged[@]} start(s) it couldn't judge, a folder it" \
    "can't enter or a link whose end it can't read, uncounted: ${all}" >&2
  short=yes
fi
if [ ${#kept[@]} -eq 0 ]; then
  exit 1
fi
if [ -n "${unnamed}" ]; then
  kept=()
fi
masks=()
if [ ${#names[@]} -gt 0 ]; then
  for name in "${names[@]}"; do
    masks+=(-I "${name}")
  done
  listed "${names[@]}"
  echo "note: du left out, uncounted, every folder named ${all} beneath its starts," \
    "a project's own too, since macOS asks about those in the home folder" >&2
  short=yes
fi
if [ -n "${follows}" ]; then
  echo "note: du followed only the links it was named, as -H, since a link beneath" \
    "may reach a folder macOS asks about under a name of its own" >&2
  short=yes
fi
# The starts go past -- when one was typed, and when a kept start looks
# like an option, as one behind a start left out can, so du reads it as a
# path; the -H given for -L goes ahead of it.
past=()
for path in ${kept[@]+"${kept[@]}"}; do
  [[ ${path} != -* ]] || dashed=yes
done
[ -z "${dashed}" ] || past=(--)
ended=0
"${du}" ${masks[@]+"${masks[@]}"} ${options[@]+"${options[@]}"} ${past[@]+"${past[@]}"} \
  ${kept[@]+"${kept[@]}"} || ended=$?
if [ -n "${short}" ] && [ "${ended}" -eq 0 ]; then
  ended=1
fi
exit "${ended}"
