# shellcheck shell=bash
# A Mac's du, which runs macOS's own but never walks into the folders the
# home folder's ignore file guards, other programs' data and the folders
# macOS asks about. Opening one makes macOS ask whether the counter may read
# it, and an agent's du of the home folder on 24 November walked into every
# one of them, since du reads no ignore file.
# A start that is a guarded folder, or lies in one, is left out of the
# count, since naming it is how a shell's glob asks too: `du -sh ~/*` names
# Desktop, Documents and Downloads. A start above one leaves out, by -I,
# every folder of that name beneath it, since macOS's du leaves a folder out
# by its name alone, not its path: a project's own Documents goes too. A
# count following links (-L) leaves out every guarded name wherever it
# starts, since a link may reach the home folder from anywhere. Either is
# said on stderr, so a smaller total never reads as the whole, and
# /usr/bin/du run by its path counts what this leaves out.
# The home folder guarded is the account's, as its configuration names it,
# and the one HOME names when that's elsewhere. A guarded folder reached
# through `..` or a link is resolved by bash, which looks inside it once;
# one named plainly is left out without being touched. Another du ahead of
# this one on PATH, as a dev shell's is, isn't reached.
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
  is one, or lies in one, is left out, and a count
  starting above one leaves out every folder of
  that name. Either is said on stderr; /usr/bin/du
  counts them as given.

EXAMPLES
  du -sh ~
  du -sh ~/*

EXIT
  du's own, or 1 when every start was left out
EOF
  exit 0
fi
args=("$@")

# du's options come first, then the paths. Only the options macOS's du takes
# are read: -B, -I, -d and -t take a value, joined to them or as the next
# word, and -- ends them. -L, before any value, follows links.
follows=""
at=0
while [ "${at}" -lt "${#args[@]}" ]; do
  arg=${args[at]}
  at=$((at + 1))
  if [ "${arg}" = -- ]; then
    break
  elif [[ ${arg} == --?* ]]; then
    continue
  elif [[ ${arg} =~ ^-[A-Za-z] ]]; then
    letters=${arg#-}
    before=${letters%%[BIdt]*}
    [[ ${before} == *L* ]] && follows=yes
    if [[ ${letters} =~ ^[^BIdt]*[BIdt]$ ]]; then
      at=$((at + 1))
    fi
  else
    at=$((at - 1))
    break
  fi
done
options=("${args[@]:0:at}")
starts=("${args[@]:at}")
# No path is du's ., judged as one but not named to du, as it wasn't typed.
unnamed=""
if [ ${#starts[@]} -eq 0 ]; then
  starts=(.)
  unnamed=yes
fi

# A folder's real path, as du opens it, never one CDPATH names instead, and
# without a subshell, since a count may name thousands of starts; empty
# when it can't be reached.
origin=${PWD}
real() {
  real=""
  if CDPATH='' cd -P -- "$1" 2>/dev/null; then
    real=${PWD}
  fi
  cd -- "${origin}"
}

# The guarded folders, under each home folder and the data volume's second
# name for it. Only a line naming one folder from the home folder's top,
# /Name/, is read.
homes=()
for home in "${account}" "${HOME-}"; do
  [ -n "${home}" ] || continue
  real "${home}"
  [ -n "${real}" ] || continue
  [[ " ${homes[*]-} " == *" ${real} "* ]] || homes+=("${real}")
done
targets=()
# A last line with no newline is read too, as ripgrep reads it.
while IFS= read -r line || [ -n "${line}" ]; do
  [[ ${line} =~ ^/[^*?![]+/$ ]] || continue
  line=${line#/}
  for home in ${homes[@]+"${homes[@]}"}; do
    targets+=("${home}/${line%/}")
    if [ -d "${data}${home}" ]; then
      targets+=("${data}${home}/${line%/}")
    fi
  done
done <"${ignore}"

# Whether a path is a guarded folder or lies in one, as `inside`. A Mac's
# disk takes a name in any case, and cd keeps the case typed, so paths are
# matched whatever their case.
guarded() {
  local path=${1,,} target
  inside=""
  for target in ${targets[@]+"${targets[@]}"}; do
    target=${target,,}
    if [ "${path}" = "${target}" ] || [[ ${path} == "${target}"/* ]]; then
      inside=yes
    fi
  done
}

# Each start a guarded folder holds is left out, by its path as typed
# before anything opens it, then by its real path. Each guarded folder's
# name is kept once, for a start above it.
declare -A leaving=()
names=()
kept=()
left=()
leave() {
  local name=${1##*/}
  [ -n "${leaving[${name}]-}" ] && return
  leaving[${name}]=yes
  names+=("${name}")
}
for path in "${starts[@]}"; do
  typed=${path%"${path##*[!/]}"}
  [[ ${typed} == /* ]] || typed=${origin}/${typed}
  guarded "${typed}"
  if [ -n "${inside}" ]; then
    left+=("${path}")
    continue
  fi
  real=""
  [ -d "${path}" ] && real "${path}"
  if [ -n "${real}" ]; then
    guarded "${real}"
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
if [ -n "${follows}" ]; then
  for target in ${targets[@]+"${targets[@]}"}; do
    leave "${target}"
  done
fi

# The words given, as one list, in `all`.
listed() {
  local one
  all=""
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
}
if [ ${#left[@]} -gt 0 ]; then
  listed "${left[@]}"
  echo "note: du left out ${#left[@]} start(s) macOS asks about, uncounted:" \
    "${all}; /usr/bin/du counts them" >&2
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
  echo "note: du left out, uncounted, every folder named ${all}, which macOS asks about" >&2
fi
exec "${du}" ${masks[@]+"${masks[@]}"} ${options[@]+"${options[@]}"} ${kept[@]+"${kept[@]}"}
