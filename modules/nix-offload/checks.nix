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
  # new one; nix-offload-decides-when holds when that is. A VM lender runs
  # for each of vmTests.extra and no more. Every VM lender, and no other,
  # warms each builder it stands, by default with a check core still has.
  nix-offload-lenders-outlive-a-switch =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.nix-offload ];
            jitsusama.nix-offload = {
              helper = pkgs.writeShellScript "a-helper" "";
              vmTests = {
                prefix = "a-vm-";
                more.prefix = "a-vm2-";
                extra = [ { prefix = "a-vm3-"; } ];
              };
              more.prefix = "a-more-";
            };
          };
        }
      ]);
      lenders = map (name: home.systemd.user.services.${name}) [
        "nix-offload"
        "nix-offload-vm"
        "nix-offload-vm-more"
        "nix-offload-vm-3"
        "nix-offload-more"
      ];
      warm = home.jitsusama.nix-offload.vmTests.warm;
      started = unit: lib.concatStringsSep " " (lib.toList unit.Service.ExecStart);
      warms = unit: lib.hasInfix " --warm " (started unit);
      named = lib.all (
        installable:
        lib.any (check: lib.hasSuffix "#checks.x86_64-linux.${check}.driver" installable) (
          lib.attrNames self.checks.x86_64-linux
        )
      ) warm;
      reloads =
        unit:
        unit.Unit.X-ReloadIfChanged
        && lib.hasSuffix " -HUP $MAINPID" unit.Service.ExecReload
        && unit.Service.RestartForceExitStatus == "100"
        && unit.Service.SuccessExitStatus == "100";
    in
    holds "nix-offload-lenders-outlive-a-switch" (
      "a switch restarts a lender, cutting every build its builder runs, "
      + "or a lender that ends superseded isn't started again, "
      + "or a VM lender runs past those vmTests.extra names, "
      + "or a lender other than a VM one warms, or one doesn't, "
      + "or what warms a VM builder names a check core doesn't have."
    ) (
      lib.all reloads lenders
      && !(home.systemd.user.services ? nix-offload-vm-4)
      && warm != [ ]
      && named
      && lib.all warms (lib.sublist 1 3 lenders)
      && !lib.any warms [
        (lib.head lenders)
        (lib.last lenders)
      ]
    );
}
