# shellcheck shell=bash
# Writes a tokenizer's IR with our chat template in place of the export's,
# and fails unless the one template it then holds is ours, so nothing from
# the model's repository is left for GenAI to render (tidy-model.nix).
# usage: tidy-template.sh <the export's IR> <our template> <where it goes>
set -euo pipefail

ir=$1
template=$2
out=$3
sed -E "s#<chat_template value=\"[^\"]*\"#<chat_template value=\"$template\"#" "$ir" >"$out"
if [ "$(grep -c '<chat_template value=' "$out")" != 1 ] ||
  ! grep -qF "<chat_template value=\"$template\"" "$out"; then
  echo "$ir's chat template isn't ours alone"
  exit 1
fi
