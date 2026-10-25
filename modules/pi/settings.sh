# shellcheck shell=bash
# Puts the settings declared for pi, the JSON file $DECLARED names, in the
# settings file pi keeps (settings.nix says why it's merged, not replaced).
# usage: pi-settings <pi's settings file>
file=$1
mkdir -p "$(dirname "$file")"
current='{}'
if [ -s "$file" ]; then
  current=$(cat "$file")
fi
merged=$(mktemp "$file.XXXXXX")
trap 'rm -f "$merged"' EXIT
if ! jq --slurpfile declared "$DECLARED" '. * $declared[0]' <<< "$current" > "$merged"; then
  echo "pi-settings: $file isn't JSON, so it's left as it is" >&2
  exit 1
fi
# Written only when something changed, so pi's file keeps its time.
if [ -s "$file" ] && cmp -s <(jq -S . <<< "$current") <(jq -S . "$merged"); then
  exit 0
fi
chmod 0644 "$merged"
mv "$merged" "$file"
