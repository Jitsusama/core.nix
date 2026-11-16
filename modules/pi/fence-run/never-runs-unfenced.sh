# shellcheck shell=bash
# pi-fence, given as its one argument, refuses a kind it doesn't know with
# its usage, and where it can't lay the fence, as in a build with neither
# bubblewrap nor landrun, refuses either kind, saying why, rather than
# running the command unfenced (checks.nix).
set -u
fence=$1
export HOME=$PWD/home
mkdir -p "$HOME" work
cd work || exit 1

status=0
"$fence" --kind loose -- true 2>asked || status=$?
if [ "$status" != 2 ] || ! grep -q "usage: pi-fence" asked; then
  echo "a wrong kind wasn't refused with the usage ($status)" >&2
  exit 1
fi

for kind in fence strict; do
  status=0
  "$fence" --kind "$kind" -- touch ran 2>said || status=$?
  if [ "$status" != 126 ] || [ -e ran ] || ! grep -q "^pi-fence: " said; then
    echo "--kind $kind ran unfenced, or refused without a reason ($status)" >&2
    cat said >&2
    exit 1
  fi
done
