# shellcheck shell=bash
# shellcheck disable=SC2310 # The rule is run in a condition, as it must fail.
# The check every-ruff-exception-hides-a-finding.nix builds: the rule must fail
# a planted tree with exactly the lines it expects, refuse a planted config
# with exactly its reasons before ruff runs, and fail when ruff can't read a
# third, before it judges core's own tree, so a rule that passes everything
# can't pass. Each planted file but clean.py holds what the planted config's
# comments say, and each tree is a folder of its own, so none reads another's
# files.
# usage: every-ruff-exception-hides-a-finding.sh <python> <rule>
#   <the folder of planted configs> <the planted tree's expected lines>
#   <the refused config's> <tree>
set -euo pipefail
python=$1 rule=$2 configs=$3 said=$4 refusals=$5 src=$6
config=${configs}/every-ruff-exception-hides-a-finding

mkdir planted refused broken
cp "${config}-planted.toml" planted/ruff.toml
touch planted/clean.py
for name in hidden prefixed twice pair; do
  echo "import os" > "planted/${name}.py"
done
echo "print(1)" > planted/printed.py
echo "import os  # noqa: F401" > planted/noqa.py
cp "${config}-refused.toml" refused/ruff.toml
cp "${config}-broken.toml" broken/ruff.toml
echo "import os" | tee refused/b.py > broken/b.py

judge() {
  "${python}" "${rule}" "$1" "$1/ruff.toml"
}

if judge planted 2> planted-said; then
  echo "the rule passed its planted tree, which it must fail"
  exit 1
fi
if ! diff "${said}" planted-said; then
  echo "the rule failed its planted tree saying other lines (< expected, > said)"
  exit 1
fi

if judge refused 2> refused-said; then
  echo "the rule passed its refused config, which it must refuse"
  exit 1
fi
if ! diff "${refusals}" refused-said; then
  echo "the rule refused its refused config saying other reasons (< expected, > said)"
  exit 1
fi

if judge broken 2> broken-said; then
  echo "the rule passed a config ruff can't read, which it must fail"
  exit 1
fi
if ! grep -q "^ruff couldn't check broken with ruff.toml, exit [1-9]" broken-said \
  || ! grep -q "Unknown rule selector .NOPE1. in .select. from .*/broken/ruff.toml" \
    broken-said; then
  echo "the rule failed a config ruff can't read without saying ruff couldn't read NOPE1:"
  cat broken-said
  exit 1
fi

judge "${src}"
