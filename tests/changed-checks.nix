# changed-checks, for the dev shell: the checks a change touches, by their
# derivations, built alone. changed-checks.py says how it's used, and its
# tests are changed-checks-test.py. ruff and mypy read both through
# `nix fmt`, at their strictest, so the writers' own flake8 pass is left off.
{ writers }:
{
  tool = writers.writePython3Bin "changed-checks" { doCheck = false; } (
    builtins.readFile ./changed-checks.py
  );
  tests = writers.writePython3 "changed-checks-test" { doCheck = false; } (
    builtins.readFile ./changed-checks-test.py
  );
}
