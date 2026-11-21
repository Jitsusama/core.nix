# The checks of what the desktop test's areas judge by, apart from any VM,
# which checks.nix gathers with every other folder's; README.md beside this
# says what each file here is.
{ pkgs, ... }:
let
  python = pkgs.python3.withPackages (python: [ python.pytest ]);
in
{
  # How keeps-pace counts what its builder stole from the guest in each turn,
  # and the share that gives, proven on the figures of a VM held back on a
  # builder. Only the two files are copied, so a change to another area's
  # file rebuilds none of this.
  a-loaded-desktop-counts-what-its-builder-took =
    pkgs.runCommandLocal "a-loaded-desktop-counts-what-its-builder-took" { }
      ''
        cp ${./keeps_pace.py} keeps_pace.py
        cp ${./keeps-pace-test.py} keeps-pace-test.py
        PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 ${python}/bin/python -m pytest \
          -p no:cacheprovider --import-mode=importlib --quiet keeps-pace-test.py
        touch $out
      '';

  # How an area's wait on what the launcher reports ends, and what one that
  # never passes says, apart from any VM, with the reader it waits through
  # run against a stand-in journal on the shell and tools the VM gives it.
  # Only those three files are copied, for the same reason.
  a-desktop-wait-says-what-it-last-heard =
    pkgs.runCommandLocal "a-desktop-wait-says-what-it-last-heard"
      {
        REPORTS_SHELL = pkgs.lib.getExe pkgs.bash;
        REPORTS_TOOLS = pkgs.lib.makeBinPath [
          pkgs.coreutils
          pkgs.gnugrep
          pkgs.gnused
        ];
      }
      ''
        cp ${./heard.py} heard.py
        cp ${./heard-test.py} heard-test.py
        cp ${./reports.sh} reports.sh
        PYTHONPATH=. PYTHONDONTWRITEBYTECODE=1 ${python}/bin/python -m pytest \
          -p no:cacheprovider --import-mode=importlib --quiet heard-test.py
        touch $out
      '';
}
