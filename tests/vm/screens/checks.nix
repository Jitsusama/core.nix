# The screens VM test's checks, which checks.nix gathers with every other
# folder's; README.md beside this says what each area proves.
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
  # Whether the shell's surfaces come back after a screen goes shows only
  # with niri on the screen itself, not nested in Cage. Each area is a
  # machine of its own, so they run side by side (screens.nix).
  vm-screens-come-and-go = import ./screens.nix {
    inherit self pkgs;
    area = "come-and-go";
  };
  vm-screens-keep-places = import ./screens.nix {
    inherit self pkgs;
    area = "keep-places";
  };
  vm-screens-keep-the-launcher = import ./screens.nix {
    inherit self pkgs;
    area = "keep-the-launcher";
  };
}
