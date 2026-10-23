# run-subtests, for the dev shell: some of a VM test's subtests, run on its
# driver in seconds. run-subtests.py says how it's used, and its tests are
# run-subtests-test.py. ruff and mypy read both through `nix fmt`, at their
# strictest, so the writers' own flake8 pass is left off.
{ writers }:
{
  tool = writers.writePython3Bin "run-subtests" { doCheck = false; } (
    builtins.readFile ./run-subtests.py
  );
  tests = writers.writePython3 "run-subtests-test" { doCheck = false; } (
    builtins.readFile ./run-subtests-test.py
  );
}
