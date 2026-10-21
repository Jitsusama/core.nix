# The settings core holds pi to, and the program that puts them in the
# settings file pi keeps. pi writes that file itself, as /settings changes
# it and as it records what it has shown, so the program keeps every key pi
# wrote, gives the declared ones their values, and leaves a file it can't
# read as JSON for a person to look at rather than replacing it.
{ pkgs }:
let
  inherit (pkgs) lib;

  settings = {
    # The room kept for what a compaction writes. pi's default, 16,384
    # tokens, runs out summarising a long session, which then compacts
    # nothing and runs on toward the model's limit.
    compaction.reserveTokens = 64000;
  };
in
pkgs.writeShellApplication {
  name = "pi-settings";
  runtimeInputs = [
    pkgs.coreutils
    pkgs.jq
  ];
  text = ''
    file="$1"
    mkdir -p "$(dirname "$file")"
    current='{}'
    if [ -s "$file" ]; then
      current="$(cat "$file")"
    fi
    merged="$(mktemp "$file.XXXXXX")"
    trap 'rm -f "$merged"' EXIT
    if ! jq --argjson declared ${lib.escapeShellArg (builtins.toJSON settings)} \
      '. * $declared' <<<"$current" >"$merged"; then
      echo "pi-settings: $file isn't JSON, so it's left as it is" >&2
      exit 1
    fi
    # Written only when something changed, so pi's file keeps its time.
    if [ -s "$file" ] && cmp -s <(jq -S . <<<"$current") <(jq -S . "$merged"); then
      exit 0
    fi
    chmod 0644 "$merged"
    mv "$merged" "$file"
  '';
}
