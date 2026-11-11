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

  # With `lines` set, root's Nix reads its builders only from the folder
  # their own account writes, the service runs as that account with its
  # settings, and the lender's account needn't be trusted; unset, root
  # reads the lender's own and its account must be.
  nix-offload-lines-reach-root-alone =
    let
      machine =
        settings:
        bareNixos [
          self.nixosModules.nix-offload
          {
            users.users.someone = {
              isNormalUser = true;
              group = "users";
            };
            jitsusama.nix-offload = {
              account = "someone";
            }
            // settings;
          }
        ];
      lined = machine {
        lines = {
          project = "a-lab";
          prefix = "a-builder-";
          zones = [ "a-zone" ];
          startupScript = pkgs.writeText "startup-script" "echo a builder\n";
          images = [ "https://compute.example/images/a-" ];
          lent = [
            {
              uri = "ssh-ng://nix-offload?compress=true";
              folder = "/run/nix-offload";
            }
          ];
        };
      };
      bare = machine { };
      failing = config: lib.filter (assertion: !assertion.assertion) config.assertions;
      service = lined.config.systemd.services.nix-offload-lines.serviceConfig;
      # Read, not built: the settings name gcloud's store path, which
      # fromJSON won't take with its context.
      read = builtins.fromJSON (
        builtins.unsafeDiscardStringContext lined.config.environment.etc."nix-offload/lines.json".text
      );
    in
    holds "nix-offload-lines-reach-root-alone"
      (
        "with lines set, root's Nix reads builders the lender's account writes, "
        + "or the service doesn't run as its own account with its settings, "
        + "or the folder isn't that account's alone, or the lender's account must still be trusted; "
        + "or, unset, an untrusted lender's account is let through."
      )
      (
        lined.config.nix.settings.builders == "@/run/nix-offload-lines/machines"
        && service.User == "nix-offload-lines"
        && lib.hasSuffix " lines --config /etc/nix-offload/lines.json" service.ExecStart
        &&
          read.lent == [
            {
              uri = "ssh-ng://nix-offload?compress=true";
              instance = "/run/nix-offload/instance";
              token = "/run/nix-offload/cloud-token";
            }
          ]
        && read.hookKey == "/etc/nix/offload_ed25519.pub"
        && lib.hasSuffix "-startup-script" read.startupScript
        && read.images == [ "https://compute.example/images/a-" ]
        && lib.elem "d /run/nix-offload-lines 0755 nix-offload-lines nix-offload-lines -" lined.config.systemd.tmpfiles.rules
        && failing lined.config == [ ]
        && bare.config.nix.settings.builders == "@/run/nix-offload/machines"
        && failing bare.config != [ ]
      );

  # clippy finds nothing in nix-offload, its tests included.
  nix-offload-passes-clippy = passesClippy "nix-offload-passes-clippy" (
    pkgs.callPackage ./offload/package.nix { }
  );

  # A switch that changes a lender's unit reloads it rather than restarting
  # it, and the lender's own end starts the new one, handing over through
  # helpers that outlast the unit; nix-offload-decides-when holds when and
  # how. A VM lender runs
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
        && unit.Service.SuccessExitStatus == "100"
        && lib.hasInfix " --outlasting " (started unit);
    in
    holds "nix-offload-lenders-outlive-a-switch"
      (
        "a switch restarts a lender, cutting every build its builder runs, "
        + "or a lender that ends superseded isn't started again, "
        + "or its helpers don't outlast it to hand its builder over, "
        + "or a VM lender runs past those vmTests.extra names, "
        + "or a lender other than a VM one warms, or one doesn't, "
        + "or what warms a VM builder names a check core doesn't have."
      )
      (
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
