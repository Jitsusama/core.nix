# shellcheck shell=bash
# Every repository rule that's a script over the source tree, run here in
# about a minute rather than through the flake's checks.
#
# rules.nix, beside this, reads tests/rules/checks.nix for each rule whose
# check only reads files, and gives each as the command its check runs, with
# its waiting list as the check passes it and the programs the checks build
# with. This runs them all at once over the tracked files of the repository
# it's run in, as the flake sees them, prints a line for each that passes,
# names and counts each it can't run, and prints each failure's output whole.
# A rule added to checks.nix runs here with no change to this.
#
#   bash tools/rules-here/rules-here.sh
#
# RULES_HERE_TREE and RULES_HERE_PLAN, when set, name a tree to read and a
# plan rules.nix gave, in place of the checkout's files and evaluating it
# here, which is how checks.nix tests this where there's no Nix or git.
# Its help and failures take the shape every command of ours does
# (docs/commands.md), by hand.
set -euo pipefail

case ${1:-} in
-h | --help)
  cat << 'EOF'
rules-here  every repository rule that's a script, over this tree

EXAMPLES
  bash tools/rules-here/rules-here.sh   each rule, passed or failed

EXIT
  0 every rule run passed · 1 a rule failed · 64 asked wrongly
  66 not in a git checkout · 69 the rules or their programs couldn't be read
EOF
  exit 0
  ;;
'') ;;
*)
  printf '%s\n' "✕ rules-here takes no words" \
    "  try  bash tools/rules-here/rules-here.sh --help" >&2
  exit 64
  ;;
esac

work=$(mktemp -d "${TMPDIR:-/tmp}/rules-here.XXXXXX")
trap 'rm -rf "$work"' EXIT

# The tracked files, as they are in the working tree, are what a check's
# flake reads; what git ignores or doesn't track, a build's output among it,
# isn't, so the rules read a copy of those alone.
tree=${RULES_HERE_TREE:-}
if [ -z "$tree" ]; then
  if ! top=$(git rev-parse --show-toplevel 2> /dev/null); then
    printf '%s\n' "✕ rules-here reads a git checkout, and this isn't in one" \
      "  try  cd into core.nix's checkout first" >&2
    exit 66
  fi
  tree=$work/tree
  mkdir "$tree"
  git -C "$top" ls-files -z --cached --deduplicate |
    while IFS= read -r -d '' file; do
      if [ -e "$top/$file" ] || [ -L "$top/$file" ]; then printf '%s\0' "$file"; fi
    done |
    tar -C "$top" --null -T - -cf - | tar -C "$tree" -xf -
fi

shell='' run_names=() run_commands=() run_paths=() not_run=() needs=()
if [ -n "${RULES_HERE_PLAN:-}" ]; then
  # A plan given was made where its programs already are, as a check's is.
  # shellcheck source=/dev/null # rules.nix writes it, setting the six above.
  . "$RULES_HERE_PLAN"
else
  plan=$work/plan.sh
  read='rules: rules { root = /. + builtins.getEnv "RULES_TREE"; }'
  if ! RULES_TREE=$tree nix eval --raw --impure --file "$tree/tools/rules-here/rules.nix" \
    --apply "$read" > "$plan" 2> "$work/plan.err"; then
    printf '%s\n' "✕ couldn't read the rules from tests/rules/checks.nix" >&2
    sed 's/^/  /' "$work/plan.err" >&2
    exit 69
  fi
  # shellcheck source=/dev/null # rules.nix writes it, setting the six above.
  . "$plan"
  # An empty array is unset to macOS's bash 3.2 under set -u, so each use
  # of one is guarded by its count, as this is.
  if [ ${#needs[@]} -gt 0 ] && ! nix build --no-link "${needs[@]}" 2> "$work/needs.err"; then
    printf '%s\n' "✕ couldn't fetch the programs the rules run with" >&2
    sed 's/^/  /' "$work/needs.err" >&2
    exit 69
  fi
fi

# Each rule in a folder of its own, all at once, since a check may write
# into the folder it runs in.
for ((index = 0; index < ${#run_names[@]}; index++)); do
  folder=$work/runs/$index
  mkdir -p "$folder"
  (
    cd "$folder"
    rc=0
    out=$folder/out RULES_TREE=$tree PATH=${run_paths[index]} \
      "$shell" -euo pipefail -c "${run_commands[index]}" > said 2>&1 || rc=$?
    echo "$rc" > rc
  ) &
done
wait

failed=()
for ((index = 0; index < ${#run_names[@]}; index++)); do
  folder=$work/runs/$index
  read -r rc < "$folder/rc"
  if [ "$rc" = 0 ] && [ -e "$folder/out" ]; then
    echo "passed   ${run_names[index]}"
  else
    failed+=("$index")
  fi
done
for ((index = 0; index < ${#not_run[@]}; index += 2)); do
  echo "not run  ${not_run[index]}: ${not_run[index + 1]}"
done
for ((each = 0; each < ${#failed[@]}; each++)); do
  index=${failed[each]}
  echo
  echo "✕ ${run_names[index]} failed"
  sed 's/^/  /' "$work/runs/$index/said"
done
echo
echo "$((${#run_names[@]} - ${#failed[@]})) passed, ${#failed[@]} failed," \
  "$((${#not_run[@]} / 2)) not run here, as their checks build"
[ ${#failed[@]} = 0 ]
