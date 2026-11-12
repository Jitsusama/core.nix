# shellcheck shell=bash
# Fails on a line outside the theme that names a colour or a face of its own,
# which would stay put when the theme changes. The pattern is built in
# checks.nix from the colours and faces it lists.
# usage: nothing-names-its-own-look.sh <tree> <pattern>
set -euo pipefail

tree=$1
pattern=$2
cd "${tree}"

# The launcher's colour answers, and the colours they're tested on, name
# colours because colours are what's asked, and eero's views and cadence's
# axes are tested on a theme's terminal colours; none is a look. checks.nix beside this lists the
# faces looked for, which is no look either. ripgrep answers 0 for a match, 1
# for none and 2 when it can't search, and only none passes.
status=0
rg --line-number --no-heading --hidden --glob '!modules/theme/**' \
  --glob '!modules/quickshell/launcher/Colours.js' --glob '!tests/colour-cases.js' \
  --glob '!modules/eero/tests/osaka-jade.json' --glob '!modules/cadence/tests/theme.json' \
  --glob '!tests/repository/checks.nix' \
  -e "${pattern}" modules roles hardware tests examples flake.nix ||
  status=$?
if [[ ${status} = 0 ]]; then
  echo "The lines above name a colour or a face; name the theme's role or font instead."
  exit 1
fi
[[ ${status} = 1 ]]
