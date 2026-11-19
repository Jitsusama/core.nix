# shellcheck shell=bash
# Runs every QML case in the folder it starts in, each for at most a minute,
# and names each that fails or runs past its minute, as it ends and again
# at the close, since the build's log otherwise says only the exit code of
# the first, not which case, nor any after it, and its tail may lose an
# early case's name under a later one's output.
# usage: QML_IMPORT_PATH=<qml modules> runs-every-case.sh <qml>
set -uo pipefail
shopt -s nullglob

qml=$1
limit=60
cases=(*.qml)
if [ "${#cases[@]}" = 0 ]; then
  echo "no cases to run in $PWD" >&2
  exit 1
fi
failed=()
for case in "${cases[@]}"; do
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
done
if [ "${#failed[@]}" != 0 ]; then
  printf '%s of %s cases failed:\n' "${#failed[@]}" "${#cases[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  exit 1
fi
