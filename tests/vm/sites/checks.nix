# The local sites' VM tests' checks, which checks.nix gathers with every
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
  # Whether the sites' server and the system daemon find each other shows
  # only with the account's manager running both units as their modules
  # render them.
  vm-the-system-page-reads-its-daemon = import ./system-page.nix { inherit self pkgs; };
}
