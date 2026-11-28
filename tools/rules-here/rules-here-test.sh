# shellcheck shell=bash
# rules-here's test: given a plan of stand-in-rules.nix, it passes a tree that
# keeps the rule, naming the rule that builds as not run, and fails once a
# Rust assert! with no message is planted, naming the rule and printing what
# it found.
#
#   bash rules-here-test.sh RULES-HERE PLAN
set -euo pipefail
script=$1
plan=$2

fail() {
  printf '%s\n' "$1" "rules-here said:" >&2
  sed 's/^/  /' said >&2
  exit 1
}

mkdir kept
printf '%s\n' 'fn held(ok: bool) {' '    assert!(ok, "it was not held");' '}' > kept/held.rs
cp -r kept planted
printf '%s\n' 'fn planted(ok: bool) {' '    assert!(ok);' '}' >> planted/held.rs

# Runs rules-here over the folder named, keeping what it said and its exit.
run() {
  status=0
  RULES_HERE_TREE=$PWD/$1 RULES_HERE_PLAN=$plan "$BASH" "$script" > said 2>&1 || status=$?
}

run kept
[ "$status" = 0 ] || fail "rules-here exited $status on a tree that keeps every rule"
grep -qxF 'passed   every-assert-says-what-was-wrong' said ||
  fail "rules-here didn't say the rule it ran passed"
grep -qxF 'not run  comments-name-what-exists: its check builds with callPackage' said ||
  fail "rules-here didn't name the rule it can't run, with why"
grep -qxF '1 passed, 0 failed, 1 not run here, as their checks build' said ||
  fail "rules-here didn't count what it ran and what it didn't"

run planted
[ "$status" = 1 ] ||
  fail "rules-here exited $status, not 1, on a tree holding an assert! with no message"
grep -qxF '✕ every-assert-says-what-was-wrong failed' said ||
  fail "rules-here didn't name the rule that failed"
grep -qF 'held.rs:5' said || fail "rules-here didn't print what the failing rule found"
grep -qxF '0 passed, 1 failed, 1 not run here, as their checks build' said ||
  fail "rules-here didn't count the failure"
