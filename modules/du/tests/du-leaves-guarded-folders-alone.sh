# shellcheck shell=bash
# Checks that the Mac's du, counting a home folder from the home folder, from
# Library, from above both it and the data volume's second name for it, or
# from / , leaves out each guarded folder below the start by its name, once,
# and says so on stderr; that a count naming a guarded folder, starting inside
# one, or starting where none lies beneath, runs as given and says nothing;
# that du's options and their values, joined or apart, are read past to the
# paths; and that --help answers without running du. A stand-in du writes
# down each word it's given, since macOS's du, whose -I leaves a folder out by
# its name, isn't there to run on Linux; nor is a Mac's disk, which takes a
# path in any case. The home folder's name holds a space and du's wildcards,
# which must reach du as one word.
# usage: du-leaves-guarded-folders-alone.sh <du.sh> <ignore file>
set -euo pipefail

wrapper=$1
ignore=$2
work=$(mktemp -d)
work=$(cd "${work}" && pwd -P)
trap 'rm -rf "${work}"' EXIT
home="${work}/a [home]*"
data=${work}/data
given=${work}/given
said=${work}/said

# The guarded folders, in the home folder and under the data volume's name
# for it, and a project beside them.
names=()
while IFS= read -r line || [ -n "${line}" ]; do
  case ${line} in
    /*/)
      folder=${line#/}
      folder=${folder%/}
      mkdir -p "${home}/${folder}/inside" "${data}${home}/${folder}"
      names+=("${folder##*/}")
      ;;
    *) ;;
  esac
done <"${ignore}"
if [ ${#names[@]} -eq 0 ]; then
  echo "the ignore file guards no folder, so nothing here is tested" >&2
  exit 1
fi
mkdir -p "${home}/src/project"

cat >"${work}/du" <<EOF
#!${BASH}
printf '%s\n' "\$@" >"${given}"
EOF
chmod +x "${work}/du"

# Runs the wrapper from a folder with the words given, keeping what it gave
# du and what it said.
count() {
  local from=$1
  shift
  rm -f "${given}"
  (cd "${from}" && HOME=${home} bash "${wrapper}" "${work}/du" "${ignore}" "${data}" "$@") \
    2>"${said}"
}

# What du was given ahead of the words typed: -I and a name, for each name.
masks() {
  local name
  for name in "$@"; do
    printf -- '-I\n%s\n' "${name}"
  done
}

# Whether du was given exactly the masks for the names, once each, then the
# words typed, and the wrapper said it left them out.
leaves() {
  local what=$1 want
  shift
  local count=$1
  shift
  want=$(masks "${@:1:count}"; printf '%s\n' "${@:count+1}")
  if [ "$(<"${given}")" != "${want}" ]; then
    echo "${what}: du was given" >&2
    cat "${given}" >&2
    echo "rather than" >&2
    echo "${want}" >&2
    exit 1
  fi
  if ! grep -qF 'du: left out, uncounted, every folder named' "${said}"; then
    echo "${what}: didn't say it left folders out: $(<"${said}")" >&2
    exit 1
  fi
}

# Whether du was given just the words typed, and nothing was said.
runs_as_given() {
  local what=$1 want
  shift
  want=$(printf '%s\n' "$@")
  if [ "$(<"${given}")" != "${want}" ] || [ -s "${said}" ]; then
    echo "${what}: du was given $(<"${given}"), and it said $(<"${said}")" >&2
    exit 1
  fi
}

library=()
for name in "${names[@]}"; do
  case ${name} in
    Containers | "Group Containers" | "Mobile Documents" | CloudStorage) library+=("${name}") ;;
    *) ;;
  esac
done
[ ${#library[@]} -gt 0 ] || {
  echo "the ignore file guards nothing in Library, so a count of it tests nothing" >&2
  exit 1
}

count "${work}" -sh "${home}"
leaves "a count of the home folder from elsewhere" ${#names[@]} "${names[@]}" -sh "${home}"
count "${home}"
leaves "a count of the home folder from in it, naming no path" ${#names[@]} "${names[@]}"
count "${home}" -d 1 .
leaves "a count of . from the home folder" ${#names[@]} "${names[@]}" -d 1 .
count "${home}" -sh Library
leaves "a count of Library" ${#library[@]} "${library[@]}" -sh Library
count "${work}" -sh /
leaves "a count of /" ${#names[@]} "${names[@]}" -sh /
count "${work}" -s "${data}"
leaves "a count above the data volume's name for home" ${#names[@]} "${names[@]}" \
  -s "${data}"

# Options and their values, joined or apart, are read past to the paths.
count "${work}" -I x -t 1M -sd 2 -h "${home}"
leaves "a count with -I, -t and -d apart" ${#names[@]} "${names[@]}" \
  -I x -t 1M -sd 2 -h "${home}"
count "${work}" -Ix -d2 --si -- "${home}"
leaves "a count with -I and -d joined, a long option and --" ${#names[@]} "${names[@]}" \
  -Ix -d2 --si -- "${home}"

# A count naming a guarded folder, starting inside one, or where none lies
# beneath, runs as given.
count "${home}" -sh Desktop
runs_as_given "a count naming Desktop" -sh Desktop
count "${work}" -sh "${home}/Documents/inside"
runs_as_given "a count inside Documents" -sh "${home}/Documents/inside"
count "${home}" -sh src
runs_as_given "a count of a project" -sh src
count "${work}" -d 1 -I "${home}" "${home}/src/project"
runs_as_given "a count whose -I names the home folder" -d 1 -I "${home}" "${home}/src/project"
count "${work}" --si -I "${home##*/}" "${home}/src/project"
runs_as_given "a count whose -I, after a long option, names the home folder" \
  --si -I "${home##*/}" "${home}/src/project"

rm -f "${given}"
help=$(cd "${work}" && HOME=${home} bash "${wrapper}" "${work}/du" "${ignore}" "${data}" --help)
if [ -e "${given}" ] || [[ ${help} != "du  "* ]]; then
  echo "--help ran du or didn't answer: ${help}" >&2
  exit 1
fi
