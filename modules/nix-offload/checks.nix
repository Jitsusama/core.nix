# The checks of nix-offload, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
onLinux {
  # When a builder is asked for and given back, from the queue's jobs, the
  # builder's line and the journal, its tests run as it builds.
  nix-offload-decides-when = pkgs.callPackage ./offload/package.nix { };

  # clippy finds nothing in nix-offload, its tests included.
  nix-offload-passes-clippy = passesClippy "nix-offload-passes-clippy" (
    pkgs.callPackage ./offload/package.nix { }
  );

  # A switch that changes a lender's unit reloads it rather than restarting
  # it, and the lender's own end once nothing uses its builder starts the
  # new one; nix-offload-decides-when holds when that is.
  nix-offload-lenders-outlive-a-switch =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.nix-offload ];
            jitsusama.nix-offload = {
              helper = pkgs.writeShellScript "a-helper" "";
              vmTests.prefix = "a-vm-";
              more.prefix = "a-more-";
            };
          };
        }
      ]);
      lenders = map (name: home.systemd.user.services.${name}) [
        "nix-offload"
        "nix-offload-vm"
        "nix-offload-more"
      ];
      reloads =
        unit:
        unit.Unit.X-ReloadIfChanged
        && lib.hasSuffix " -HUP $MAINPID" unit.Service.ExecReload
        && unit.Service.RestartForceExitStatus == "100"
        && unit.Service.SuccessExitStatus == "100";
    in
    holds "nix-offload-lenders-outlive-a-switch"
      "a switch restarts a lender, cutting every build its builder runs, or a lender that ends superseded isn't started again."
      (lib.all reloads lenders);
}
