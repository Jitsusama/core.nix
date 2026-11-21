# The tools the desktop test's reader of what the shell reported runs on (reports.sh), beside
# the journal the VM gives it, in one list so the check that runs the reader off any VM
# (checks.nix) runs it on the same tools as the VM (../desktop.nix).
pkgs: [
  pkgs.coreutils
  pkgs.gnugrep
  pkgs.gnused
]
