# The program that puts the settings declared for pi (jitsusama.pi.settings)
# in the settings file pi keeps. pi writes that file itself, as /settings
# changes it and as it records what it has shown, so the program keeps every
# key pi wrote, gives the declared ones their values, and leaves a file it
# can't read as JSON for a person to look at rather than replacing it.
{ pkgs, settings }:
let
  inherit (pkgs) lib;
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
