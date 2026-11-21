# shellcheck shell=bash
# Checks that a search of a home folder holding the Mac's ignore file never
# opens other apps' data or the folders macOS guards, with ripgrep or fd,
# started from the home folder or from Library, and still finds everything
# else. Each guarded folder is made unreadable, so opening it is an error the
# program reports. Without the file, the same search reports them, so the
# check can fail.
# usage: searches-leave-other-apps-data-alone.sh <ignore file> <rg> <fd>
set -euo pipefail

ignore=$1
rg=$2
fd=$3
home=$(mktemp -d)
trap 'chmod -R u+rwx "$home"; rm -rf "$home"' EXIT
guarded=(
  "Library/Containers/com.example.app"
  "Library/Group Containers/group.example"
  "Desktop"
  "Documents"
  "Downloads"
)
for folder in "${guarded[@]}" Library/Logs src; do
  mkdir -p "$home/$folder"
  printf 'needle\n' > "$home/$folder/needle"
done
locked=("$home/Library/Containers" "$home/Library/Group Containers" "$home/Desktop" \
  "$home/Documents" "$home/Downloads")

# Every search from the home folder and from Library, with what each found
# and what each said kept apart.
searched() {
  local from
  for from in "$home" "$home/Library"; do
    (cd "$from" && "$rg" --no-config --files-with-matches needle) || true
    (cd "$from" && "$fd" --no-global-ignore-file needle) || true
  done
}

chmod 000 "${locked[@]}"
said=$(searched 2>&1 >/dev/null)
if ! grep -q 'ermission denied' <<<"$said"; then
  echo "a search with no ignore file opened no guarded folder, so this check can't fail" >&2
  exit 1
fi

cp "$ignore" "$home/.ignore"
found=$(searched 2>"$home/said")
if [ -s "$home/said" ]; then
  echo "a search opened other apps' data or a folder macOS guards: $(cat "$home/said")" >&2
  exit 1
fi
for folder in Library/Logs src; do
  if ! grep -q "$folder/needle\|^Logs/needle" <<<"$found"; then
    echo "a search no longer finds $folder: $found" >&2
    exit 1
  fi
done
if grep -q -e Containers -e Desktop -e Documents -e Downloads <<<"$found"; then
  echo "a search listed other apps' data or a folder macOS guards: $found" >&2
  exit 1
fi

# A search that names a guarded folder still searches it.
chmod 755 "$home/Documents"
if ! (cd "$home" && "$rg" --no-config --files-with-matches needle Documents) >/dev/null; then
  echo "a search naming Documents no longer searched it" >&2
  exit 1
fi
