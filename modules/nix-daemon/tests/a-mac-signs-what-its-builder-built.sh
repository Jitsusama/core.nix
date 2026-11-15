#!/usr/bin/env bash
# Runs the hook a Mac's Nix runs after each build, as Nix runs it, against a
# stand-in nix that keeps what it was asked: it signs every path Nix names
# with the Mac's key, asks nothing when Nix names none, and ends 0 when the
# signing fails, saying which paths stay unsigned.
#
# Usage: a-mac-signs-what-its-builder-built.sh BASH HOOK
set -euo pipefail

bash=$1
hook=$2
here=$(mktemp -d)
trap 'rm -rf "$here"' EXIT

# The sandbox has no /usr/bin/env, so the stand-in runs the bash given.
echo "#!$bash" >"$here/nix"
cat >>"$here/nix" <<'EOF'
echo "$*" >>"$(dirname "$0")/asked"
exit "$(cat "$(dirname "$0")/ends" 2>/dev/null || echo 0)"
EOF
chmod +x "$here/nix"

# Each failure names the hook, since it runs beside the daemon's own checks.
wrong() {
  echo "sign-what-came-back: $1" >&2
  exit 1
}

OUT_PATHS="/nix/store/a-one /nix/store/b-two" "$bash" "$hook" "$here/nix" ||
  wrong "the hook failed though the signing passed"
asked=$(cat "$here/asked" 2>/dev/null || true)
[ "$asked" = "store sign --key-file /etc/nix/signing-key /nix/store/a-one /nix/store/b-two" ] ||
  wrong "the hook asked nix for '$asked', not to sign both paths with the Mac's key"

rm -f "$here/asked"
OUT_PATHS="" "$bash" "$hook" "$here/nix" || wrong "the hook failed with nothing built"
[ ! -e "$here/asked" ] || wrong "the hook asked nix for something though nothing was built"

echo 1 >"$here/ends"
said=$(OUT_PATHS="/nix/store/a-one" "$bash" "$hook" "$here/nix" 2>&1) ||
  wrong "a signing that failed failed the build"
case $said in
*"/nix/store/a-one stays unsigned"*) ;;
*) wrong "a signing that failed said '$said', not which path stays unsigned" ;;
esac
echo "every path is signed, none asked of nothing, and a failure fails no build"
