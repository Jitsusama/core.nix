#!/usr/bin/env bash
# Code health and core's every-long-file-says-why name the same folders
# crowded. Each planted folder holds one entry past the limit beside a note
# that gives its reason in one way, or fails to, and both are run over the
# planted tree. Takes core's rule; code-health, git and python3 come from
# PATH.
set -euo pipefail

rule=${1:?usage: crowds-as-core-does.sh RULE}
tree=$(mktemp -d)
into=$(mktemp -d)

# plant FOLDER NOTE TEXT: 41 files and the note, past the limit of 40.
plant() {
  local folder=${1} note=${2} text=${3} n
  mkdir -p "${tree}/${folder}"
  for n in $(seq 0 40); do
    : >"${tree}/${folder}/${n}.txt"
  done
  printf '%s\n' "${text}" >"${tree}/${folder}/${note}"
}

plant silent README.md "Each file is a case."
plant opening README.md "Many entries because each is a case."
# The reason past the 40 lines a code file's opening is read in.
late=""
for _ in $(seq 1 45); do
  late+=$'A line.\n'
done
plant late README.md "${late}Many entries because each is a case."
plant wrapped README.md "$(printf -- '- Many entries\n  because each is one.')"
plant agents AGENTS.md "(Many entries because each is one.)"
plant after README.md "Too Many entries becausewords."
plant before README.md "TooMany entries because each is one."
plant under README.md "_Many entries because_ each is one."

core=$(python3 "${rule}" "${tree}" /dev/null 2>&1 >/dev/null |
  sed -n 's/ holds more than .*//p' | sort || true)

git -C "${tree}" init -q
git -C "${tree}" add -A
git -C "${tree}" -c user.name=planted -c user.email=planted@example.com \
  commit -q -m planted
code-health --repo "planted=${tree}" --ref planted=HEAD --into "${into}" >/dev/null
health=$(python3 -c '
import json, sys
from pathlib import Path
named = json.loads(next(Path(sys.argv[1]).glob("*-named.json")).read_text())
print("\n".join(sorted(named["planted"].get("crowded-folders", []))))
' "${into}")

if [[ "${core}" != "${health}" ]]; then
  printf 'core'\''s rule calls these folders crowded:\n%s\n' "${core}" >&2
  printf 'code health calls these crowded:\n%s\n' "${health}" >&2
  exit 1
fi
printf 'both call these crowded:\n%s\n' "${health}"
