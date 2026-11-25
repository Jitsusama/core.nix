# shellcheck shell=bash
# Checks that the Mac's du, counting a home folder from the home folder, from
# Library, from above both it and the data volume's second name for it, from
# / , or following links from anywhere, leaves out each guarded folder below
# the start by its name, once, and says so on stderr in full; that a start
# naming a guarded folder, by a glob's list, in another case, through `..`
# or inside one, is left out of the count and said, du not run when none is
# left; that the account's home is guarded whatever HOME says, and HOME's
# too; that du's options and their values, joined or apart, are read past
# to the paths, from a folder below the home so a skipped path would show;
# that a count above no guarded folder runs as given and says nothing; and
# that --help answers without running du. A stand-in du writes down each
# word it's given, since macOS's du, whose -I leaves a folder out by its
# name, isn't there to run on Linux; nor is a Mac's disk, which takes a path
# in any case, so a second folder named like the home in capitals stands in
# for the home typed so. The home folder's name holds a space and du's
# wildcards, which must reach du as one word.
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
project=${home}/src/project
mkdir -p "${project}"

cat >"${work}/du" <<EOF
#!${BASH}
printf '%s\n' "\$@" >"${given}"
EOF
chmod +x "${work}/du"

# Runs the wrapper from a folder with the words given, the account's home
# being the stand-in and HOME the stand-in unless `moved` names another,
# keeping what it gave du, what it said and how it ended.
moved=""
count() {
  local from=$1
  shift
  rm -f "${given}"
  ended=0
  (cd "${from}" && HOME=${moved:-${home}} bash "${wrapper}" "${work}/du" "${ignore}" "${data}" \
    "${home}" "$@") 2>"${said}" || ended=$?
}

# What the wrapper says it left out: the starts, and the names.
starts_left() {
  local all="" one
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
  echo "note: du left out $# start(s) macOS asks about, uncounted: ${all}; /usr/bin/du counts them"
}
names_left() {
  local all="" one
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
  echo "note: du left out, uncounted, every folder named ${all}, which macOS asks about"
}

# What du was given ahead of the words typed: -I and a name, for each name.
masks() {
  local name
  for name in "$@"; do
    printf -- '-I\n%s\n' "${name}"
  done
}

# Whether the wrapper said exactly what's wanted, and gave du exactly the
# words wanted.
gives() {
  local what=$1 telling=$2 want
  shift 2
  want=$(printf '%s\n' "$@")
  if [ "$(<"${given}")" != "${want}" ] || [ "$(<"${said}")" != "${telling}" ]; then
    echo "${what}: du was given" >&2
    cat "${given}" >&2
    echo "and the wrapper said: $(<"${said}")" >&2
    echo "rather than" >&2
    echo "${want}" >&2
    echo "and: ${telling}" >&2
    exit 1
  fi
}

every_name=$(names_left "${names[@]}")
every_mask=$(masks "${names[@]}")

# Whether du was given the masks for every name, once each, then the words
# typed, and the wrapper said so.
leaves_all() {
  local what=$1
  shift
  gives "${what}" "${every_name}" "${every_mask}" "$@"
}

# Whether du wasn't run at all, the wrapper ending 1 and saying it left the
# starts out.
runs_nothing() {
  local what=$1 telling ran=""
  shift
  telling=$(starts_left "$@")
  if [ -e "${given}" ]; then
    ran=$(<"${given}")
  fi
  if [ -n "${ran}" ] || [ "${ended}" -ne 1 ] || [ "$(<"${said}")" != "${telling}" ]; then
    echo "${what}: du ran (${ran}), it ended ${ended}, and it said $(<"${said}")" >&2
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
library_names=$(names_left "${library[@]}")
library_masks=$(masks "${library[@]}")

count "${work}" -sh "${home}"
leaves_all "a count of the home folder from elsewhere" -sh "${home}"
count "${home}"
leaves_all "a count of the home folder from in it, naming no path"
count "${home}" -d 1 .
leaves_all "a count of . from the home folder" -d 1 .
count "${home}" -sh Library
gives "a count of Library" "${library_names}" "${library_masks}" -sh Library
count "${work}" -sh /
leaves_all "a count of /" -sh /
count "${work}" -s "${data}"
leaves_all "a count above the data volume's name for home" -s "${data}"
count "${work}" -sh "${project}" "${home}"
leaves_all "a count of a project and then the home folder" -sh "${project}" "${home}"
count "${work}" -L -sh "${project}"
leaves_all "a count following links, from a project" -L -sh "${project}"

# The account's home is guarded wherever HOME points, and HOME's own too.
moved=/nonexistent
count "${work}" -sh "${home}"
leaves_all "a count of the home folder with HOME gone" -sh "${home}"
moved="${work}/another home"
mkdir -p "${moved}"
count "${work}" -sh "${moved}"
leaves_all "a count of the home HOME names" -sh "${moved}"
moved=""

# A Mac's disk takes the home folder's name in any case.
mkdir -p "${work}/A [HOME]*"
count "${work}" -sh "${work}/A [HOME]*"
leaves_all "a count of the home folder in capitals" -sh "${work}/A [HOME]*"

# Options and their values, joined or apart, are read past to the paths,
# counted from a project, where a path read as a value would leave `.`.
count "${project}" -I x -t 1M -sd 2 -h "${home}"
leaves_all "a count with -I, -t and -d apart" -I x -t 1M -sd 2 -h "${home}"
count "${project}" -Ix -d2 --si -- "${home}"
leaves_all "a count with -I and -d joined, a long option and --" -Ix -d2 --si -- "${home}"
count "${project}" -d1 "${home}"
leaves_all "a count with -d1" -d1 "${home}"
count "${project}" -hd1 "${home}"
leaves_all "a count with -hd1" -hd1 "${home}"
count "${project}" -I.git "${home}"
leaves_all "a count with -I.git" -I.git "${home}"
count "${project}" -B 4096 "${home}/Documents"
runs_nothing "a count of Documents with -B apart" "${home}/Documents"

# A start that is a guarded folder, however it's named, or lies in one, is
# left out of the count, which goes on with the rest.
count "${home}" -sh Desktop
runs_nothing "a count naming Desktop" Desktop
count "${work}" -sh "${home}/Documents/inside"
runs_nothing "a count inside Documents" "${home}/Documents/inside"
count "${work}" -sh "${home}/DOWNLOADS/"
runs_nothing "a count naming Downloads in capitals" "${home}/DOWNLOADS/"
count "${work}" -sh "${home}/src/../Documents"
runs_nothing "a count reaching Documents through .." "${home}/src/../Documents"
count "${home}" -sh Desktop Library src
left=$(starts_left Desktop)
gives "a count of a glob's list of the home folder" "${left}"$'\n'"${library_names}" \
  "${library_masks}" -sh Library src
count "${work}" -sh "${home}/Documents" "${home}"
left=$(starts_left "${home}/Documents")
gives "a count of Documents and then the home folder" "${left}"$'\n'"${every_name}" \
  "${every_mask}" -sh "${home}"

# A count above no guarded folder runs as given and says nothing.
count "${home}" -sh src
gives "a count of a project" "" -sh src
count "${work}" -d 1 -I "${home}" "${project}"
gives "a count whose -I names the home folder" "" -d 1 -I "${home}" "${project}"
count "${work}" --si -I "${home##*/}" "${project}"
gives "a count whose -I, after a long option, names the home folder" "" \
  --si -I "${home##*/}" "${project}"

rm -f "${given}"
help=$(cd "${work}" && HOME=${home} bash "${wrapper}" "${work}/du" "${ignore}" "${data}" "${home}" \
  --help)
if [ -e "${given}" ] || [[ ${help} != "du  "* ]]; then
  echo "--help ran du or didn't answer: ${help}" >&2
  exit 1
fi
