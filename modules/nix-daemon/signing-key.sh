# shellcheck shell=bash
# Makes the key the Nix daemon signs what it builds with, once, as the
# machine activates, and writes its public half beside it for other machines
# to trust. The secret half is root's alone and never leaves the machine; a
# key already there is kept, since other machines trust it by name.
#
# Usage: signing-key.sh NIX KEY-NAME SECRET PUBLIC
set -eu

nix=$1
name=$2
secret=$3
public=$4

if [ ! -s "$secret" ]; then
  (
    umask 077
    "$nix" --extra-experimental-features nix-command key generate-secret \
      --key-name "$name" > "$secret.new"
    mv "$secret.new" "$secret"
  )
fi
"$nix" --extra-experimental-features nix-command key convert-secret-to-public \
  < "$secret" > "$public.new"
chmod 0644 "$public.new"
mv "$public.new" "$public"
