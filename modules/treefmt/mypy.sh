# shellcheck shell=bash
# mypy as `nix fmt` runs it (treefmt.nix): from a Python carrying the
# libraries a repository's scripts import, with the test driver's names and
# VM tests' helpers on its path, so a VM test's script is read as it runs.
# Its help takes the shape every command of ours does (docs/commands.md), by
# hand, and -h still reaches mypy's own, since that's where its flags are.
if [[ $# -eq 1 && $1 == --help ]]; then
  cat << 'EOF'
mypy  check Python's types, as nix fmt does

  mypy, from a Python carrying the libraries a repository's
  scripts import, with VM tests' driver and helpers on its
  path. nix fmt runs it on every .py file at once.

EXAMPLES
  mypy script.py  check one script
  mypy -h         mypy's own help, and its flags

EXIT
  what mypy exits
EOF
  exit 0
fi
MYPYPATH="@path@" exec "@python@/bin/mypy" "$@"
