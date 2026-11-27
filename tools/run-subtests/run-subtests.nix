# run-subtests, for the dev shell: some of a VM test's subtests, run on its
# driver in seconds. run-subtests.py says how it's used, and its tests are
# run-subtests-test.py and run-subtests-quarantine-test.py, which pytest runs
# as checks.nix says. ruff and mypy read the program and its tests through
# `nix fmt`, at their strictest, so the writers' own flake8 pass is left off.
{ writers }:
writers.writePython3Bin "run-subtests" { doCheck = false; } (builtins.readFile ./run-subtests.py)
