# shellcheck shell=bash
# Puts the settings declared for pi, the JSON file $DECLARED names, in the
# settings file pi keeps (settings.nix says why it's merged, not replaced).
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
pi-settings  put the settings declared for pi in pi's file

  Keeps every key pi wrote and gives the declared ones
  their values. A file that isn't JSON is left for a
  person to look at. Every switch runs it.

EXAMPLES
  pi-settings ~/.pi/agent/settings.json

EXIT
  0   done, or nothing to change
  1   the file isn't JSON, so it's left as it is
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

if [ "$#" -ne 1 ]; then
  printf '%s\n' "pi-settings takes pi's settings file" \
    "  try  pi-settings --help" >&2
  exit 64
fi

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
