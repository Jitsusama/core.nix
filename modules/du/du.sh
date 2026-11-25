# shellcheck shell=bash
# A Mac's du, which runs macOS's own but leaves out the folders the home
# folder's ignore file guards, other programs' data and the folders macOS asked
# about, when a count starts above one. Opening one makes macOS ask whether the
# counter may read it, and an agent's du of the home folder on 24 November
# walked into every one of them, since du reads no ignore file.
# macOS's du leaves a folder out by its name alone (-I), not by its path, so
# a count above a guarded folder leaves out every folder named like one
# beneath it too, a project's own Documents among them, and says so on
# stderr, so a smaller total never reads as the whole.
# A count naming a guarded folder, or starting inside one, still counts it, as
# ripgrep, fd and the Mac's find do, unless another start in the same count is
# above one, when its name is left out for all of them. Nor does it reach
# /usr/bin/du run by its path, or another du ahead of it on PATH, as a dev
# shell's is. One following links (-L) leaves out only the folders it meets by
# their own names, and a session with HOME moved guards the folders of the
# home it names.
# usage: du.sh <du> <ignore file> <data volume> [du's arguments...]
set -euo pipefail

du=$1
ignore=$2
# Where macOS mounts the volume the home folder lives on, which reaches the
# home folder by a second name, so a count of / walks it twice.
data=$3
shift 3

# macOS's du takes no --help, so answering it here hides nothing of du's.
if [ "${1-}" = --help ]; then
  cat <<'EOF'
du  macOS's du, leaving guarded folders alone

  Runs macOS's du, but a count starting above a
  folder the home folder's .ignore guards leaves
  out every folder of that name, uncounted, and
  says so on stderr. A count naming one, or
  starting in one, counts it.

EXAMPLES
  du -sh ~
  du -sh ~/Documents

EXIT
  du's own
EOF
  exit 0
fi
args=("$@")

# du's options come first, then the paths. Only the options macOS's du takes
# are read: -I, -d and -t take a value, joined to them or as the next word,
# and -- ends them.
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
    if [[ ${letters} =~ ^[^Idt]*[Idt]$ ]]; then
      at=$((at + 1))
    fi
  else
    at=$((at - 1))
    break
  fi
done
starts=("${args[@]:at}")
if [ ${#starts[@]} -eq 0 ]; then
  starts=(.)
fi

# The guarded folders, by both the home folder's names. Only a line naming one
# folder from the home folder's top is read, as the Mac's find reads them.
# Each folder is taken by its real path, as du opens it, never a folder CDPATH
# names instead.
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

# Each guarded folder's name, once, for a start it lies under. A Mac's disk
# takes a name in any case, and cd keeps the case typed, so paths are matched
# whatever their case. A start that is a guarded folder, or lies inside one,
# leaves out nothing of its own.
declare -A leaving=()
names=()
for path in "${starts[@]}"; do
  real=$(CDPATH='' cd -P -- "${path}" 2>/dev/null && pwd -P) || continue
  for target in ${targets[@]+"${targets[@]}"}; do
    if [ "${real}" != / ] && [[ ${target,,} != "${real,,}"/* ]]; then
      continue
    fi
    name=${target##*/}
    [ -n "${leaving[${name}]-}" ] && continue
    leaving[${name}]=yes
    names+=("${name}")
  done
done

if [ ${#names[@]} -eq 0 ]; then
  exec "${du}" "$@"
fi
masks=()
said=""
for name in "${names[@]}"; do
  masks+=(-I "${name}")
  said="${said:+${said}, }${name}"
done
echo "du: left out, uncounted, every folder named ${said}, which macOS asks about" >&2
exec "${du}" "${masks[@]}" "$@"
