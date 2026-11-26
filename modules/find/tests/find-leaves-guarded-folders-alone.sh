# shellcheck shell=bash
# Checks that the Mac's find, searching a home folder from the home folder,
# from Library or from above both it and the data volume's second name for it,
# run there or given the path from elsewhere, never opens the folders the
# ignore file guards, still lists them, says it left them unsearched, and
# finds everything else, folders in a project named like a guarded one
# included; an expression that would open one, -empty or an -exec, isn't
# given it.
# Each guarded folder is made unreadable, so opening it is an error find
# reports. The find it runs, run alone, reports them, so the check can fail. A
# search naming a guarded folder, or starting inside one, still searches it,
# beside its parent too; an expression with an action of its own keeps it; and
# a search find can't prune runs as given. A stand-in find that prints what
# it's given shows the spellings only macOS's find takes. The home folder's
# name holds find's wildcards, which must match only themselves.
# usage: find-leaves-guarded-folders-alone.sh <find.sh> <ignore file> <find>
set -euo pipefail

wrapper=$1
ignore=$2
real=$3
# What the searches say is kept beside the home folder, never in it, so no
# search from the home folder reads it. Its real path, as the wrapper reads the
# home folder's.
work=$(mktemp -d)
work=$(cd "${work}" && pwd -P)
home="${work}/a [home]*"
data=${work}/data
said=${work}/said
stand=$(mktemp -d)
trap 'chmod -R u+rwx "${work}"; rm -rf "${work}" "${stand}"' EXIT
note='find: left the folders macOS asks about unsearched'
unlisted='find: left out the folders macOS asks about, unsearched and unlisted'

# The folders the ignore file guards, each with a needle in it, in the home
# folder and under the data volume's name for it.
guarded=()
while IFS= read -r line; do
  case ${line} in
    /*/)
      line=${line#/}
      guarded+=("${line%/}")
      ;;
    *) ;;
  esac
done <"${ignore}"
locked=()
for folder in "${guarded[@]}"; do
  locked+=("${home}/${folder}" "${data}${home}/${folder}")
done
for folder in "${locked[@]}" "${home}/Documents/repo" "${home}/Library/Logs" "${home}/src" \
  "${home}/src/Desktop" "${home}/src/Documents" "${home}/src/Downloads" "${data}${home}/src"; do
  mkdir -p "${folder}"
  printf 'needle\n' >"${folder}/needle"
done
starts=("${home}" "${home}/Library" "${work}")
# How a search reaches its start: run inside it as ., given its path from
# elsewhere, or given it with a slash at its end.
hows=(inside elsewhere slashed)
how=inside
program=${wrapper}

# The wrapper, run in a folder with the home folder and data volume given;
# what it ends on is find's, which the checks read from what it says.
wrapped() {
  local at=$1
  shift
  (cd "${at}" && HOME=${home} bash "${wrapper}" "${real}" "${ignore}" "${data}" "$@") || true
}

# A search for the needle from a folder, reached as $how says, with the
# expression given, none (--bare) or -name needle: what it finds on stdout,
# each by its whole path, and what it says in $said.
search() {
  local from=$1
  shift
  local at=${from} paths=(.)
  case ${how} in
    elsewhere) at=${stand} paths=("${from}") ;;
    slashed) at=${stand} paths=("${from}/") ;;
    *) ;;
  esac
  local expression=(-name needle)
  if [ "${1-}" = --bare ]; then
    expression=()
  elif [ $# -gt 0 ]; then
    expression=("$@")
  fi
  local out line
  if [ "${program}" = "${wrapper}" ]; then
    out=$(wrapped "${at}" "${paths[@]}" ${expression[@]+"${expression[@]}"} 2>"${said}")
  else
    out=$( (cd "${at}" && "${real}" "${paths[@]}" ${expression[@]+"${expression[@]}"}) \
      2>"${said}") || true
  fi
  # GNU find warns of a -maxdepth or -mindepth given after -path, as the
  # wrapper's prunes put it; macOS's find doesn't, so the warning is dropped.
  local kept
  kept=$(grep -v 'warning: you have specified the global option' "${said}") || true
  printf '%s' "${kept}" >"${said}"
  while IFS= read -r line; do
    case ${line} in
      .) line=${from} ;;
      ./*) line=${from}/${line#./} ;;
      "${from}//"*) line=${from}/${line#"${from}//"} ;;
      *) ;;
    esac
    printf '%s\n' "${line}"
  done <<<"${out}"
}

# What a search from each start finds when it finds everything it should.
declare -A expected=(
  ["${home}"]="${home}/Library/Logs/needle
${home}/src/needle
${home}/src/Desktop/needle
${home}/src/Documents/needle
${home}/src/Downloads/needle"
  ["${home}/Library"]="${home}/Library/Logs/needle"
  ["${work}"]="${home}/src/needle
${data}${home}/src/needle"
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
    if [[ $(<"${said}") != "${note}" ]]; then
      echo "${who} opened a guarded folder or didn't say it left one out: $(<"${said}")" >&2
      exit 1
    fi
    while read -r path; do
      if ! grep -qxF "${path}" <<<"${found}"; then
        echo "${who} no longer finds ${path}: ${found}" >&2
        exit 1
      fi
    done <<<"${expected[${from}]}"
    # Each guarded folder under the start is still listed, as find lists it
    # from its parent.
    found=$(search "${from}" -type d)
    for folder in "${locked[@]}"; do
      if [[ ${folder} == "${from}/"* ]] && ! grep -qxF "${folder}" <<<"${found}"; then
        echo "${who} no longer lists ${folder}: ${found}" >&2
        exit 1
      fi
    done
  done
done

# An expression with an action of its own keeps it, and prints nothing more;
# one with none, or none at all, still prints what it matches.
how=inside
named "${home}"
found=$(search "${home}" -name needle -exec printf 'seen %s\n' '{}' ';')
if [[ $(<"${said}") != "${unlisted}" ]] || ! grep -qxF 'seen ./Library/Logs/needle' <<<"${found}" \
  || grep -qxF "${home}/Library/Logs/needle" <<<"${found}"; then
  echo "${who} with -exec lost its action or printed more: ${found} $(<"${said}")" >&2
  exit 1
fi
# An -exec's own -d is its command's, never find's, and neither it nor
# -empty is run on a guarded folder, which either would open.
found=$(search "${home}" -type d -exec ls -d '{}' +)
if [[ $(<"${said}") != "${unlisted}" ]] || grep -qxF "${home}/Desktop" <<<"${found}" \
  || ! grep -qxF "${home}/Library/Logs" <<<"${found}"; then
  echo "${who} gave a guarded folder to -exec ls -d: ${found} $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" -type d -empty)
if [[ $(<"${said}") != "${unlisted}" ]] || grep -qxF "${home}/Desktop" <<<"${found}"; then
  echo "${who} tested a guarded folder with -empty: ${found} $(<"${said}")" >&2
  exit 1
fi
# -execdir runs its command in each folder's parent on ./ and its name, and
# Containers is a name only a guarded folder has.
found=$(wrapped "${home}" . -type d -execdir ls -d '{}' + 2>"${said}")
if [[ $(<"${said}") != "${unlisted}" ]] || grep -qxF ./Containers <<<"${found}" \
  || ! grep -qxF ./Logs <<<"${found}"; then
  echo "${who} gave a guarded folder to -execdir ls -d: ${found} $(<"${said}")" >&2
  exit 1
fi
# A primary's operand is never read as a primary: -name's -empty is a
# pattern, so the search says the usual note, not the one -empty gives.
found=$(search "${home}" -name -empty)
if [[ $(<"${said}") != "${note}" ]]; then
  echo "${who} read -name's -empty as a primary: $(<"${said}")" >&2
  exit 1
fi
# -print and -ls print each match once, the search adding no -print of its
# own.
for action in -print -ls; do
  found=$(search "${home}" -name needle "${action}")
  logs=$(grep -cF Library/Logs/needle <<<"${found}") || logs=0
  if [[ $(<"${said}") != "${note}" ]] || [ "${logs}" != 1 ]; then
    echo "${who} with ${action} didn't print each match once: ${found} $(<"${said}")" >&2
    exit 1
  fi
done
# find opens no folder at its -maxdepth, so one there is left as find leaves
# it, listed and with no note, unless -empty would open it; one above it is
# still pruned.
found=$(search "${home}" -maxdepth 1 -type d)
if [ -s "${said}" ] || ! grep -qxF "${home}/Desktop" <<<"${found}"; then
  echo "${who} with -maxdepth 1 didn't run as given: ${found} $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" -maxdepth 1 -type d -empty)
if [[ $(<"${said}") != "${unlisted}" ]]; then
  echo "${who} with -maxdepth 1 tested a guarded folder with -empty: $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" -maxdepth 2 -name needle)
if [[ $(<"${said}") != "${note}" ]] || grep -qxF "${home}/Library/Logs/needle" <<<"${found}"; then
  echo "${who} with -maxdepth 2 opened a guarded folder or went past it: ${found}" \
    "$(<"${said}")" >&2
  exit 1
fi
# A -maxdepth counts from the start, so from above the home folder it
# reaches the guarded folders deeper down, and still prunes them.
rest=${home#"${work}"/}
slashes=${rest//[^\/]/}
named "${work}"
found=$(search "${work}" -maxdepth $((${#slashes} + 3)) -name needle)
if [[ $(<"${said}") != "${note}" ]] || ! grep -qxF "${home}/src/needle" <<<"${found}"; then
  echo "${who} with -maxdepth $((${#slashes} + 3)) opened a guarded folder or lost a match:" \
    "${found} $(<"${said}")" >&2
  exit 1
fi
named "${home}"
# A depth is read as macOS's find reads it, the last given, a + or blanks
# before its digits as well, which GNU's find refuses, so what this one
# says is left out of what's read. ' -1' and '+-1' are -1, find's own no
# limit, and zeros before a digit count for nothing however many there are.
# A number past a long long, 2^64 + 1, is held at its most, and kept in
# an int, as -1, where wrapping round would make it 1. Kept in an int,
# 2^32 + 1 is 1, and zeros past a long long's digits are 0, each of which
# opens no guarded folder, so runs with nothing said.
for given in '+3' ' 3' '+ +3' ' -1' '+-1' '0000000000000000000003' '18446744073709551617'; do
  search "${home}" -maxdepth 1 -maxdepth "${given}" -name needle >/dev/null
  if ! grep -qxF "${note}" "${said}"; then
    echo "${who} with -maxdepth 1 then '${given}' ran as -maxdepth 1: $(<"${said}")" >&2
    exit 1
  fi
done
for given in '4294967297' '0000000000000000000000'; do
  search "${home}" -maxdepth 3 -maxdepth "${given}" -name needle >/dev/null
  if grep -qxF "${note}" "${said}"; then
    echo "${who} with -maxdepth 3 then '${given}' was pruned: $(<"${said}")" >&2
    exit 1
  fi
done
# -d is -depth, so a number after either makes a test of depth, which
# walks as ever and is pruned; and -exec's command ends at a word that
# starts with ;, so a -mindepth after it is read, as find reads it.
for given in '-d 2' '-depth 2'; do
  # shellcheck disable=SC2086 # Each is split into its words, as typed.
  search "${home}" ${given} -name needle >/dev/null
  if ! grep -qxF "${note}" "${said}"; then
    echo "${who} with ${given} wasn't pruned: $(<"${said}")" >&2
    exit 1
  fi
done
search "${home}" -exec true ';x' -mindepth 3 >/dev/null
if grep -qxF -e "${note}" -e "${unlisted}" "${said}"; then
  echo "${who} with -exec ending at ';x' lost the -mindepth after it: $(<"${said}")" >&2
  exit 1
fi
# An action of its own that opens nothing lists each guarded folder it
# matches once, as it prints, with nothing added: NUL-ended, as -print0 asks.
found=$(wrapped "${home}" . -type d -print0 2>"${said}" | tr '\n\0' '!\n')
desktops=$(grep -cxF ./Desktop <<<"${found}") || desktops=0
if [[ $(<"${said}") != "${note}" ]] || [[ ${found} == *'!'* ]] || [ "${desktops}" != 1 ]; then
  echo "${who} with -print0 printed more, or a guarded folder twice: ${found} $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" -type f -o -name nothing)
if [[ $(<"${said}") != "${note}" ]] || ! grep -qxF "${home}/Library/Logs/needle" <<<"${found}"; then
  echo "${who} with an expression joined by -o lost a match: ${found} $(<"${said}")" >&2
  exit 1
fi
found=$(search "${home}" --bare)
if [[ $(<"${said}") != "${note}" ]] || ! grep -qxF "${home}/Library/Logs/needle" <<<"${found}" \
  || ! grep -qxF "${home}/Desktop" <<<"${found}"; then
  echo "${who} with no expression lost a match: ${found} $(<"${said}")" >&2
  exit 1
fi

# A search find can't prune, going depth first or testing nothing above its
# -mindepth, runs as given, so it opens the guarded folders; one whose
# -mindepth lies above them is still pruned.
for given in -depth '-depth -maxdepth 2' '-mindepth 3'; do
  # shellcheck disable=SC2086 # Each is split into its words, as typed.
  search "${home}" ${given} -name needle >/dev/null
  if ! grep -q 'ermission denied' "${said}" || grep -qF "${note}" "${said}"; then
    echo "${who} with ${given} didn't run as given: $(<"${said}")" >&2
    exit 1
  fi
done
found=$(search "${home}" -mindepth 1 -name needle)
if [[ $(<"${said}") != "${note}" ]] || ! grep -qxF "${home}/Library/Logs/needle" <<<"${found}"; then
  echo "${who} with -mindepth 1 opened a guarded folder or lost a match: $(<"${said}")" >&2
  exit 1
fi

# Readable again, nothing in the guarded folders is ever listed, since the
# search leaves them out rather than the locks.
chmod 755 "${locked[@]}"
for how in "${hows[@]}"; do
  for from in "${starts[@]}"; do
    named "${from}"
    found=$(search "${from}")
    for folder in "${locked[@]}"; do
      if grep -qF "${folder}/" <<<"${found}"; then
        echo "${who} listed what's in ${folder}: ${found}" >&2
        exit 1
      fi
      # Nor is a guarded folder printed when the expression doesn't match it.
      if grep -qxF "${folder}" <<<"${found}"; then
        echo "${who} printed ${folder}, which -name needle doesn't match: ${found}" >&2
        exit 1
      fi
    done
  done
done

# A search naming a guarded folder, or starting inside one, as in a
# repository kept in Documents, still searches it, beside its parent too.
how=inside
named "${home}"
found=$(wrapped "${home}" Documents -name needle 2>"${said}")
if ! grep -qxF Documents/repo/needle <<<"${found}" || grep -qF "${note}" "${said}"; then
  echo "${who} naming Documents no longer searched it: ${found}" >&2
  exit 1
fi
found=$(wrapped "${stand}" "${home}" "${home}/Documents" -name needle 2>"${said}")
if ! grep -qxF "${home}/Documents/repo/needle" <<<"${found}" \
  || grep -qxF "${home}/Desktop/needle" <<<"${found}"; then
  echo "${who} naming Documents beside the home folder lost it: ${found}" >&2
  exit 1
fi
named "${home}/Documents/repo"
found=$(search "${home}/Documents/repo")
if ! grep -qxF "${home}/Documents/repo/needle" <<<"${found}"; then
  echo "${who} no longer searched it: ${found}" >&2
  exit 1
fi

# What only macOS's find takes, given to a stand-in that prints its words:
# a slashed start named as macOS doubles the slash, and as Linux doesn't;
# / pruned by both the home folder's names; a depth test, -f, -- and
# -mindepth 1 pruned; and depth first, as an option or a primary, or a
# -mindepth past a guarded folder, given as typed.
printf '#!%s\nprintf "%%s\\n" "$@"\n' "${BASH}" >"${stand}/find"
chmod +x "${stand}/find"
real=${stand}/find
given() {
  said=$(wrapped "${stand}" "$@" 2>/dev/null)
  typed=$(printf '%s\n' "$@")
}
given "${home}/" -name x
if ! grep -qE '[^/]//Desktop$' <<<"${said}" || ! grep -qE '[^/]/Desktop$' <<<"${said}"; then
  echo "a slashed start isn't pruned by both spellings: ${said}" >&2
  exit 1
fi
given / -name x
if ! grep -qxF "/${home#/}/Desktop" <<<"${said//\\/}" \
  || ! grep -qxF "/${data#/}${home}/Desktop" <<<"${said//\\/}"; then
  echo "/ isn't pruned by both the home folder's names: ${said}" >&2
  exit 1
fi
# Each pruned search keeps its options and start first, as typed.
for args in depth-test f-path dashes shallow option exec-d; do
  case ${args} in
    depth-test) lead=("${home}") && given "${home}" -depth 2 -name x ;;
    f-path) lead=(-f "${home}") && given -f "${home}" -name x ;;
    dashes) lead=(-- "${home}") && given -- "${home}" -name x ;;
    shallow) lead=("${home}") && given "${home}" -mindepth 1 -name x ;;
    option) lead=(-x "${home}") && given -x "${home}" -name x ;;
    exec-d) lead=("${home}") && given "${home}" -type d -exec ls -d '{}' + ;;
    *) lead=() ;;
  esac
  leading=$(printf '%s\n' "${lead[@]}" '(')
  if ! grep -qxF -- -prune <<<"${said}" || [[ ${said} != "${leading}"$'\n'* ]]; then
    echo "a search with ${args} wasn't pruned as typed: ${said}" >&2
    exit 1
  fi
done
# -ok and -okdir ask before each run, so they're seen only here: each is kept
# off the guarded folders, which -prune ends with no listing after it.
for action in -ok -okdir; do
  given "${home}" -type d "${action}" ls '{}' ';'
  after=$(grep -xF -A1 -- -prune <<<"${said}" | tail -n 1)
  if [ "${after}" != -o ]; then
    echo "a search with ${action} lists the guarded folders: ${said}" >&2
    exit 1
  fi
done
for args in depth-primary depth-option deep delete; do
  case ${args} in
    depth-primary) given "${home}" -name x -d ;;
    depth-option) given -d "${home}" -name x ;;
    deep) given "${home}" -mindepth 2 -name x ;;
    delete) given "${home}" -name x -delete ;;
    *) ;;
  esac
  if [ "${said}" != "${typed}" ]; then
    echo "a search with ${args} wasn't given as typed: ${said}" >&2
    exit 1
  fi
done
# The ignore file's last line is guarded with no newline after it, as
# ripgrep reads it.
printf '%s' "$(<"${ignore}")" >"${work}/unended"
said=$( (cd "${stand}" && HOME=${home} bash "${wrapper}" "${real}" "${work}/unended" "${data}" \
  "${home}" -name x) 2>/dev/null)
final=${guarded[${#guarded[@]} - 1]}
if ! grep -qE "/${final}\$" <<<"${said}"; then
  echo "the ignore file's last line, ${final}, isn't guarded without a newline: ${said}" >&2
  exit 1
fi
given "${home}" "${home}/Documents" -name x
# A prune names the home folder with its wildcards escaped; a start doesn't.
if grep -qF '\[home]\*/Documents' <<<"${said}"; then
  echo "a start naming Documents beside the home folder is pruned: ${said}" >&2
  exit 1
fi
