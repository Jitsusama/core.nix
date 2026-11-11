# shellcheck shell=bash
# Fails on a line outside the theme and every tests folder that spells one of
# the code points the theme's table (modules/theme/glyphs.nix) names, as a
# script writes one, 0xf023 or an escape, since a surface that did would keep
# its mark when the table changes.
# usage: nothing-spells-a-mark.sh <tree>
set -euo pipefail

cd "$1"
codes=$(rg --only-matching --no-filename --replace "\$1" 'nerd "([0-9a-fA-F]+)"' \
  modules/theme/glyphs.nix | sort -u | paste -sd '|')
[[ -n ${codes} ]]

# ripgrep answers 0 for a match, 1 for none and 2 when it can't search, and
# only none passes.
status=0
rg --line-number --no-heading --hidden --ignore-case --glob '!modules/theme/**' \
  --glob '!**/tests/**' \
  -e "(0x|\\\\u\\{?|\\\\U000)0*(${codes})\\b" modules roles hardware examples flake.nix ||
  status=$?
if [[ ${status} = 0 ]]; then
  echo "The lines above spell a mark the theme names; use the theme's mark instead."
  exit 1
fi
[[ ${status} = 1 ]]
