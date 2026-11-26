# shellcheck shell=bash
# Checks that the Mac's du, counting a home folder from the home folder, from
# Library, from above both it and the data volume's second name for it, from
# / , even from inside a guarded folder, leaves out each guarded folder below
# the start by its name, once, says so on stderr in full and ends 1; that a
# start naming a guarded folder, by a glob's list, in another case, with
# `./`, `//` or `/.`, through `..`, passing through one or inside one, a
# file through a link or by the data volume's name, is left out of the count
# and said, judged before it's opened, du not run when none is left, and
# that a start after it that looks like an option reaches du as a path; that
# a start that is a link is judged by its folder unless -H follows it; that
# a link -H follows that reaches no folder or a folder that can't be entered
# is left out and said apart; that a start named - is that folder and CDPATH moves none;
# that only a home's own guarded folders are left out by name; that du's own
# nonzero end is kept; that -L, alone or among letters, is taken out, given
# as -H and said unless -H or -P follows it; that the account's home is
# guarded whatever HOME says, HOME's too, one whose name begins another's, an
# unset one, and a home named through a link; that a working folder gone
# still counts a whole path; that a home that can't be found, or holds no
# guarded folder, is said; that du's options and their values, joined or apart, are
# read past to the paths, from a folder below the home so a skipped path
# would show; that a count above no guarded folder runs as given, says
# nothing and ends 0; and that --help answers without running du. A
# stand-in du writes down each
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
trap 'chmod -R u+rwx "${work}"; rm -rf "${work}"' EXIT
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

# The stand-in du ends as DU_ENDS says, 0 unless it's set.
cat >"${work}/du" <<EOF
#!${BASH}
printf '%s\n' "\$@" >"${given}"
exit "\${DU_ENDS:-0}"
EOF
chmod +x "${work}/du"

# Runs the wrapper from a folder with the words given, the account's home
# being the stand-in unless `account` names another, and HOME the stand-in
# unless `moved` names another or is `unset`; with `gone` set, the folder is
# removed once the wrapper's shell is in it. It keeps what the wrapper gave
# du, what it said and how it ended.
moved=""
account=""
gone=""
count() {
  local from=$1
  shift
  rm -f "${given}"
  ended=0
  (
    cd "${from}"
    if [ -n "${gone}" ]; then
      rmdir "${from}"
    fi
    if [ "${moved}" = unset ]; then
      unset HOME
    else
      export HOME=${moved:-${home}}
    fi
    bash "${wrapper}" "${work}/du" "${ignore}" "${data}" "${account:-${home}}" "$@"
  ) 2>"${said}" || ended=$?
  # bash itself says, as it starts, that it can't find a folder removed.
  if [ -n "${gone}" ]; then
    grep -v '^shell-init: ' "${said}" >"${said}.kept" || true
    mv "${said}.kept" "${said}"
  fi
}

# What the wrapper says it left out: the starts, and the names.
starts_left() {
  local all="" one
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
  echo "note: du left out $# start(s) in or through folders macOS asks about, uncounted: ${all}"
}
names_left() {
  local all="" one
  for one in "$@"; do
    all="${all:+${all}, }${one}"
  done
  echo "note: du left out, uncounted, every folder named ${all} beneath its starts, a project's" \
    "own too, since macOS asks about those in the home folder"
}
unfollowed="note: du followed only the links it was named, as -H, since a link beneath may reach"
unfollowed+=" a folder macOS asks about under a name of its own"

# What du was given ahead of the words typed: -I and a name, for each name.
masks() {
  local name
  for name in "$@"; do
    printf -- '-I\n%s\n' "${name}"
  done
}

# Whether the wrapper said exactly what's wanted, gave du exactly the words
# wanted, and ended 1 when it said anything, 0 when not, or as `ends` says.
ends=""
gives() {
  local what=$1 telling=$2 want ending=0 ran="(du wasn't run)"
  shift 2
  want=$(printf '%s\n' "$@")
  if [ -n "${telling}" ]; then
    ending=1
  fi
  ending=${ends:-${ending}}
  if [ -e "${given}" ]; then
    ran=$(<"${given}")
  fi
  if [ "${ran}" != "${want}" ] || [ "$(<"${said}")" != "${telling}" ] ||
    [ "${ended}" -ne "${ending}" ]; then
    echo "${what}: du was given" >&2
    echo "${ran}" >&2
    echo "and the wrapper said: $(<"${said}"), ending ${ended} rather than ${ending}" >&2
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
count "${home}/Documents/inside" -sh /
leaves_all "a count of / from inside Documents" -sh /
count "${home}/Documents/inside" -sh ../..
leaves_all "a count of ../.. from inside Documents" -sh ../..

# -L is taken out, alone or among letters, given as -H and said, unless -H
# or -P follows it; an L that is -I's value is a name.
count "${work}" -L -sh "${project}"
gives "a count asked to follow links, from a project" "${unfollowed}" -sh -H "${project}"
count "${work}" -sLh "${project}"
gives "a count asked to follow links by -sLh" "${unfollowed}" -sh -H "${project}"
count "${work}" -L -P "${project}"
gives "a count asked to follow links, then not" "" -P "${project}"
count "${work}" -L -H "${project}"
gives "a count asked to follow links, then only those named" "" -H "${project}"
count "${work}" -ILx "${project}"
gives "a count whose -I names Lx" "" -ILx "${project}"

# The account's home is guarded wherever HOME points, and HOME's own too.
moved=/nonexistent
count "${work}" -sh "${home}"
leaves_all "a count of the home folder with HOME gone" -sh "${home}"
moved="unset"
count "${work}" -sh "${home}"
leaves_all "a count of the home folder with HOME unset" -sh "${home}"
moved="${work}/another home"
mkdir -p "${moved}"
count "${work}" -sh "${moved}"
gives "a count of the home HOME names, holding no guarded folder" "" -sh "${moved}"
mkdir "${moved}/Documents" "${moved}/Desktop"
count "${work}" -sh "${moved}"
two_names=$(names_left Desktop Documents)
two_masks=$(masks Desktop Documents)
gives "a count of the home HOME names, holding two" "${two_names}" "${two_masks}" \
  -sh "${moved}"
rmdir "${moved}/Documents" "${moved}/Desktop"
# A home whose name begins the account's, which a match within the names
# would take for one already found.
moved="${work}/a"
mkdir -p "${moved}/Documents"
count "${work}" -sh "${moved}/Documents"
runs_nothing "a count of Documents in a home whose name begins the account's" \
  "${moved}/Documents"
rm -r "${moved}"
account=/nonexistent
moved=/nonexistent
count "${work}" -sh "${project}"
ends=0
gives "a count with no home to be found, which leaves nothing out" \
  "note: du found no home folder to guard, so it guards nothing" -sh "${project}"
account=${project}
moved=${project}
count "${work}" -sh "${project}"
gives "a count whose home holds no guarded folder, which leaves nothing out" \
  "note: du found no folder to guard in the home folder, so it guards nothing" \
  -sh "${project}"
ends=""
account=""
moved=""

# A working folder gone still counts a start named by its whole path.
mkdir "${work}/gone"
gone=yes
count "${work}/gone" -sh "${home}"
leaves_all "a count of the home folder from a working folder gone" -sh "${home}"
gone=""

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
count "${home}/Documents/inside" -sh ..
runs_nothing "a count of .. from inside Documents" ..
# A start that is a link is counted as the link alone, as macOS's du
# counts one unless -H follows it, so it's judged by the folder holding it;
# with -H, or a / after it, by the folder it reaches.
ln -s "${home}/Documents" "${home}/src/notes"
count "${work}" -sh "${home}/src/notes"
gives "a count of a link to Documents" "" -sh "${home}/src/notes"
count "${work}" -shH "${home}/src/notes"
runs_nothing "a count of a link to Documents that -H follows" "${home}/src/notes"
count "${work}" -sL "${home}/src/notes"
runs_nothing "a count of a link to Documents that -L follows" "${home}/src/notes"
count "${work}" -sh "${home}/src/notes/"
runs_nothing "a count of a link to Documents with a / after it" "${home}/src/notes/"
: >"${home}/Documents/inside/file"
count "${work}" -sh "${home}/src/notes/inside/file"
runs_nothing "a count of a file reached through a link to Documents" \
  "${home}/src/notes/inside/file"
rm "${home}/src/notes"
count "${work}" -sh "${data}${home}/Documents"
runs_nothing "a count of Documents by the data volume's name" "${data}${home}/Documents"

# A link -H follows is judged by where its links lead, read as written
# before anything goes there: to a file in Documents, even by a second
# link or one whose file is gone, it's left out; to a file elsewhere it's
# counted. A start that can't be judged, a folder that can't be entered or
# a link that leads round to itself, is left out and said apart. Root
# enters any folder, so the test needs an account that can't.
mkdir "${home}/src/locked"
chmod 000 "${home}/src/locked"
if (cd "${home}/src/locked" 2>/dev/null); then
  echo "a folder can still be entered after chmod 000, as root can, so this tests nothing" >&2
  exit 1
fi
chmod 755 "${home}/src/locked"
ln -s "${home}/Documents/inside/file" "${home}/src/note"
unjudged="note: du left out 1 start(s) it couldn't judge, a folder it can't enter or a"
unjudged+=" link whose end it can't read, uncounted:"
count "${home}" -sh src/note src
gives "a count of a link to a file in Documents" "" -sh src/note src
count "${home}" -shH src/note
runs_nothing "a count of a link to a file in Documents that -H follows" src/note
# Locked, Documents' folder can't be gone into, so a judgement that went
# there before reading where a link leads would be told as unjudged.
ln -s note "${home}/src/again"
chmod 000 "${home}/Documents/inside"
count "${home}" -shH src/again
runs_nothing "a count of a link to a link to a file in Documents that -H follows" src/again
chmod 755 "${home}/Documents/inside"
rm "${home}/Documents/inside/file"
chmod 000 "${home}/Documents/inside"
count "${home}" -shH src/note
runs_nothing "a count of a link to a gone file in Documents that -H follows" src/note
chmod 755 "${home}/Documents/inside"
rm "${home}/src/note" "${home}/src/again"
: >"${home}/src/file"
ln -s file "${home}/src/near"
count "${home}" -shH src/near
gives "a count of a link to a file elsewhere that -H follows" "" -shH src/near
rm "${home}/src/near" "${home}/src/file"
ln -s round "${home}/src/round"
count "${home}" -shH src/round src
gives "a count of a link that leads round to itself" "${unjudged} src/round" -shH src
rm "${home}/src/round"
chmod 000 "${home}/src/locked"
count "${home}" -sh src/locked src
gives "a count of a folder that can't be entered" "${unjudged} src/locked" -sh src
chmod 755 "${home}/src/locked"
rmdir "${home}/src/locked"

# A start named - is a folder of that name, not the last one cd was in,
# and CDPATH never sends a relative start elsewhere.
mkdir "${work}/-" "${work}/inside"
count "${work}" -sh -- -
gives "a count of a folder named -" "" -sh -- -
export CDPATH="${home}/Documents"
count "${work}" -sh inside
unset CDPATH
gives "a count of inside with CDPATH naming Documents" "" -sh inside
rmdir "${work}/-" "${work}/inside"

# du's own end, when it isn't 0, is kept, whether or not anything was left
# out.
export DU_ENDS=2
ends=2
count "${home}" -sh src
gives "a count whose du ends 2" "" -sh src
count "${home}" -sh Desktop src
left=$(starts_left Desktop)
gives "a count leaving Desktop out, whose du ends 2" "${left}" -sh src
unset DU_ENDS
ends=""

# A start named plainly is judged before it's opened: with Documents and
# Downloads unable to be entered, and so having no real path, each named
# plainly, however it's spelt, or passed through, is still left out, as is
# one named through a link to the home folder, the account's home named so.
chmod 000 "${home}/Documents" "${home}/Downloads"
count "${work}" -sh "${home}/Documents"
runs_nothing "a count of a Documents that can't be entered" "${home}/Documents"
count "${home}" -sh ./Documents
runs_nothing "a count naming ./Documents" ./Documents
count "${work}" -sh "${home}//Documents"
runs_nothing "a count naming Documents after //" "${home}//Documents"
count "${home}" -sh Documents/.
runs_nothing "a count naming Documents/." Documents/.
count "${home}" -sh Downloads/../src
runs_nothing "a count passing through Downloads" Downloads/../src
ln -s "${home}" "${work}/linked"
account=${work}/linked
count "${work}" -sh "${work}/linked/Documents"
runs_nothing "a count of Documents through a home named by a link" "${work}/linked/Documents"
account=""
chmod 755 "${home}/Documents" "${home}/Downloads"

# A start after one left out that looks like an option reaches du as a path.
left=$(starts_left Documents)
count "${home}" -s Documents -L
gives "a count with -L after Documents" "${left}" -s -- -L
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
