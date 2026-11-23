# shellcheck shell=bash
# Holds systemd-run-options.txt to the options the systemd it's given
# takes, as its parser lists every one in --introspect-cli: hidden ones
# too, as the deprecated --tty, which --help leaves out. The wrapper is
# built only once this passes, against the systemd it wraps, so a machine
# whose systemd takes other options fails its own build rather than reading
# calls by a table that isn't its systemd's. Usage:
# systemd-run-options-hold <options table> <systemd's systemd-run>
set -euo pipefail
table=$1
systemd_run=$2

"${systemd_run}" --introspect-cli >introspected
# The table has a row for each option, a letter and a long name, so it can't
# say an option with no long name or a second of either, which the wrapper
# wouldn't know and would read as bare, nor one whose value may be left
# out, which only a joined value gives, or any other way of taking one.
# One the wrapper would misread is refused here rather than guessed at.
odd=$(jq -r '.commands[] | select(.names | index("systemd-run")) | .options[]
  | select((.names | map(select(startswith("--"))) | length) != 1
    or (.names | map(select(test("^-[^-]$"))) | length) > 1
    or (.names | map(select(test("^-[^-]$") or startswith("--") | not)) | length) > 0
    or (.argument as $taking | ["no_argument", "required_argument"] | index($taking)) == null)
  | .names | join(",")' introspected)
if [[ -n "${odd}" ]]; then
  echo "systemd-run takes options the table can't say, so systemd-run.sh can't read them:"
  echo "${odd}"
  exit 1
fi
jq -r '.commands[] | select(.names | index("systemd-run")) | .options[]
  | ((.names | map(select(test("^-[^-]$"))) | first // "--") | ltrimstr("-"))
    + " " + (.names | map(select(startswith("--"))) | first | ltrimstr("--"))
    + (if .argument == "no_argument" then "" else "=" end)' introspected | sort >taken
# Their order is left out: the wrapper looks names up, never by place.
rows=$(grep -v '^#' "${table}")
printf '%s\n' "${rows}" | sort >table-rows
if ! differs=$(diff table-rows taken); then
  echo "systemd-run-options.txt isn't the options this systemd-run takes, which are:"
  echo "${differs}"
  echo "Judge each new one against names_its_place in systemd-run.sh too: an option"
  echo "naming a slice or another manager has to pass the call through unchanged."
  exit 1
fi
