# shellcheck shell=bash
# shellcheck disable=SC2310 # The rule is run in a condition, as it must fail.
# The check every-ruff-exception-hides-a-finding.nix builds: the rule must fail
# the planted tree with exactly the lines it expects, refuse the refused config
# with exactly its reasons before ruff runs, and fail when ruff can't check the
# broken one, before it judges core's own tree, so a rule that passes
# everything can't pass.
# usage: every-ruff-exception-hides-a-finding.sh <python> <rule> <planted>
#   <the planted tree's expected lines> <the refused config's> <tree>
set -euo pipefail
python=$1 rule=$2 planted=$3 said=$4 refused=$5 src=$6

judge() {
  "${python}" "${rule}" "$1" "$1/ruff.toml"
}

if judge "${planted}" 2> planted-said; then
  echo "the rule passed its planted tree, which it must fail"
  exit 1
fi
diff "${said}" planted-said

if judge "${planted}/refused" 2> refused-said; then
  echo "the rule passed its refused config, which it must refuse"
  exit 1
fi
diff "${refused}" refused-said

if judge "${planted}/broken" 2> broken-said; then
  echo "the rule passed a config ruff can't read, which it must fail"
  exit 1
fi
if ! grep -q "^ruff couldn't check .* with ruff.toml:$" broken-said; then
  echo "the rule failed a config ruff can't read without saying ruff couldn't check:"
  cat broken-said
  exit 1
fi

judge "${src}"
