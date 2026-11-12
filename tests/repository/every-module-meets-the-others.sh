# shellcheck shell=bash
# Every module core offers is brought into some example machine, so each
# meets every other before a machine repository's machine does: a module
# no role, hardware, example or other module brings would clash with the
# rest only on a real machine, past every check here.
#
# It takes the repository and a file of "<system>Modules <name>" lines, one
# for each output, and counts a module brought when a role, a hardware
# model, an example or a machine flake.nix builds itself, such as the
# guide's stick, names it as an output, or a role, a hardware model or
# another module imports its file by path. Checks and tests don't count,
# since they build a module alone rather than beside the others.
set -euo pipefail

repository=$1
outputs=$2

unbrought=()
while read -r system name; do
  # A name ends where a character a name can't hold follows, so guide isn't
  # taken as brought where only guide-stick is.
  named="$system\\.$name([^a-z0-9_-]|\$)"
  if rg --quiet --glob '*.nix' "$named" \
    "$repository/roles" "$repository/hardware" "$repository/examples" ||
    rg --quiet "self\\.$named" "$repository/flake.nix"; then
    continue
  fi
  if rg --quiet --glob '*.nix' --glob '!checks.nix' --glob '!**/checks/**' \
    --glob '!**/tests/**' --glob "!$name/**" \
    "(modules/|\\.\\./)$name/[a-z-]+\\.nix" \
    "$repository/roles" "$repository/hardware" "$repository/modules"; then
    continue
  fi
  unbrought+=("$system.$name")
done <"$outputs"

if ((${#unbrought[@]} > 0)); then
  echo "No role, hardware, example or other module brings these, so they meet" >&2
  echo "the rest only on a machine; import each in an example (examples/):" >&2
  printf '  %s\n' "${unbrought[@]}" >&2
  exit 1
fi
