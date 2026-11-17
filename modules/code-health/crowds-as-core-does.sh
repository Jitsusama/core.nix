#!/usr/bin/env bash
# Code health and core's every-long-file-says-why name the same folders
# crowded, and the ones expected. Each planted folder sits at or past the
# limit of 40 entries beside a note that gives its reason in one way, or
# fails to, and both are run over the planted tree. Takes core's rule;
# code-health, git and python3 come from PATH.
set -euo pipefail

rule=${1:?usage: crowds-as-core-does.sh RULE}
tree=$(mktemp -d)
into=$(mktemp -d)
said=$(mktemp)
trap 'rm -rf "${tree}" "${into}" "${said}"' EXIT
# The planted repository is read with no one's git config, run by hand too.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

# plant FOLDER FILES NOTE TEXT: FILES files and the note.
plant() {
  local folder=${1} files=${2} note=${3} text=${4} n
  mkdir -p "${tree}/${folder}"
  for ((n = 0; n < files; n++)); do
    : >"${tree}/${folder}/${n}.txt"
  done
  printf '%s\n' "${text}" >"${tree}/${folder}/${note}"
}

# How each note gives its reason, each folder past the limit.
plant silent 41 README.md "Each file is a case."
plant opening 41 README.md "Many entries because each is a case."
# The reason past the 40 lines a code file's opening is read in.
late=""
for _ in $(seq 1 45); do
  late+=$'A line.\n'
done
plant late 41 README.md "${late}Many entries because each is a case."
plant wrapped 41 README.md "$(printf -- '- Many entries\n  because each is one.')"
plant agents 41 AGENTS.md "(Many entries because each is one.)"
plant after 41 README.md "Too Many entries becausewords."
plant before 41 README.md "TooMany entries because each is one."
plant under 41 README.md "_Many entries because_ each is one."
# How entries are counted, none giving a reason: the note counts, and so
# does a folder in the folder.
plant full 39 README.md "At the limit."
plant over 40 README.md "One past it."
plant nested/deep 41 README.md "Deeper down."
for n in $(seq 0 40); do
  plant "folders/${n}" 1 README.md "One of many."
done
printf '%s\n' "Folders only." >"${tree}/folders/README.md"
expected=$(printf '%s\n' after before folders nested/deep over silent under)

status=0
python3 "${rule}" "${tree}" /dev/null 2>"${said}" >/dev/null || status=$?
if ((status > 1)) || grep -q Traceback "${said}"; then
  cat "${said}" >&2
  echo "core's rule failed to run" >&2
  exit 1
fi
core=$(sed -n 's/ holds more than .*//p' "${said}" | sort)

git -C "${tree}" init -q
git -C "${tree}" add -A
git -C "${tree}" -c user.name=planted -c user.email=planted@example.com \
  commit -q -m planted
code-health --repo "planted=${tree}" --ref planted=HEAD --into "${into}" >/dev/null
health=$(python3 -c '
import json, sys
from pathlib import Path
named = json.loads(next(Path(sys.argv[1]).glob("*-named.json")).read_text())
print("\n".join(sorted(named["planted"]["crowded-folders"])))
' "${into}")

if [[ "${core}" != "${expected}" || "${health}" != "${expected}" ]]; then
  printf 'expected these folders crowded:\n%s\n' "${expected}" >&2
  printf 'core'\''s rule calls these crowded:\n%s\n' "${core}" >&2
  printf 'code health calls these crowded:\n%s\n' "${health}" >&2
  exit 1
fi
printf 'both call these crowded:\n%s\n' "${health}"
