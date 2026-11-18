# The programs' VM tests' checks, which checks.nix gathers with every
# other folder's; README.md beside this says what each test proves.
{
  self,
  onLinux,
  vmHostPkgs,
  ...
}:
let
  pkgs = vmHostPkgs;
in
onLinux {
  # Only Chrome, started, says whether it installed what the preferences
  # file names, and writes a session file as it browses.
  vm-chrome-says-its-open-tabs = import ./chrome-extension.nix { inherit self pkgs; };
}
