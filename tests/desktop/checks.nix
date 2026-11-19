# The checks of what the desktop test's areas judge by, apart from any VM,
# which checks.nix gathers with every other folder's; README.md beside this
# says what each file here is.
{ pkgs, ... }:
let
  python = pkgs.python3.withPackages (python: [ python.pytest ]);
in
{
  # How keeps-pace holds a launcher under load to its time frozen, by the
  # share of the processors the builder gave the guest, proven on the
  # figures of runs that failed on a busy builder. Only the two files are
  # copied, so a change to another area's file rebuilds none of this.
  a-loaded-desktop-is-judged-by-the-builders-share =
    pkgs.runCommandLocal "a-loaded-desktop-is-judged-by-the-builders-share" { }
      ''
        cp ${./keeps_pace.py} keeps_pace.py
        cp ${./keeps-pace-test.py} keeps-pace-test.py
        PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 ${python}/bin/python -m pytest \
          -p no:cacheprovider --import-mode=importlib --quiet keeps-pace-test.py
        touch $out
      '';
}
