# shellcheck shell=bash
# Runs every QML case in the folder it starts in, each for at most a minute,
# and names each that fails or runs past its minute, as it ends and again
# at the close, since the build's log otherwise says only the exit code of
# the first, not which case, nor any after it, and its tail may lose an
# early case's name under a later one's output. Each case says how long it
# took, so one nearing its limit shows before a busy builder pushes it over.
# The journeys get three minutes: they step through 150 journeys a
# millisecond at a time, and ran past a minute on loaded builders three
# times (16, 17 and 21 November) with nothing wrong. Stepping them more
# cheaply, so they fit a minute again, is due Mon 23 November 18:00.
# usage: QML_IMPORT_PATH=<qml modules> runs-every-case.sh <qml>
set -uo pipefail
shopt -s nullglob

qml=$1
limit_of() {
  case $1 in
    margin-cat-journeys.qml) echo 180 ;;
    *) echo 60 ;;
  esac
}
cases=(*.qml)
if [ "${#cases[@]}" = 0 ]; then
  echo "no cases to run in $PWD" >&2
  exit 1
fi
failed=()
for case in "${cases[@]}"; do
  limit=$(limit_of "$case")
  began=$SECONDS
  timeout "$limit" "$qml" "$case" || {
    code=$?
    if [ "$code" = 124 ]; then
      said="$case ran past its $limit s"
    else
      said="$case failed, exit code $code"
    fi
    echo "$said" >&2
    failed+=("$said")
  }
  echo "$case took $((SECONDS - began)) s of its $limit" >&2
done
if [ "${#failed[@]}" != 0 ]; then
  printf '%s of %s cases failed:\n' "${#failed[@]}" "${#cases[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  exit 1
fi
