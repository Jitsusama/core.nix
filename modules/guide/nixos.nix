# The guide on a machine it installed, for the steps that finish it there
# and the repairs after: the program, built with the machine's own systemd
# so its bootctl and cryptenroll are the ones the machine boots with, and
# what it reads about the machine (src/installed/mod.rs's Facts).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  guide = pkgs.callPackage ./package.nix { systemd = config.systemd.package; };
  showStatus = "${config.systemd.package}/bin/busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager SetShowStatus s";
  # Each account's keys in the TPM, as its home-manager modules name them
  # (modules/ssh-tpm-agent/home.nix's jitsusama.tpmKeys).
  named = lib.concatLists (
    lib.mapAttrsToList (
      account: home:
      map (key: {
        inherit account;
        inherit (key) file pin;
        comment = lib.concatStringsSep " " (
          [ "${account}@${config.networking.hostName}" ] ++ lib.optional (key.label != "") key.label
        );
      }) (home.jitsusama.tpmKeys or [ ])
    ) (config.home-manager.users or { })
  );
  # The accounts that keep their login key's spare on each YubiKey
  # (modules/ssh-tpm-agent/home.nix's jitsusama.yubikeySpare).
  spares =
    lib.mapAttrsToList
      (account: home: {
        inherit account;
        folder = "${home.home.homeDirectory}/.ssh";
        comment = "${account}@yubikey";
      })
      (
        lib.filterAttrs (_: home: home.jitsusama.yubikeySpare or false) (config.home-manager.users or { })
      );
in
{
  options.jitsusama.guide.spares = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          account = lib.mkOption {
            type = lib.types.str;
            description = "Whose spare it is.";
          };
          folder = lib.mkOption {
            type = lib.types.str;
            description = "Where the account keeps its SSH keys, such as its ~/.ssh.";
          };
          comment = lib.mkOption {
            type = lib.types.str;
            description = "What its public half says, before the YubiKey's name.";
          };
        };
      }
    );
    default = spares;
    defaultText = lib.literalMD "every account with `jitsusama.yubikeySpare`, from home-manager";
    description = ''
      The accounts that keep a spare SSH key on each YubiKey the machine's
      rescue bundle names, which the guide keeps there at boot. A YubiKey
      holds one resident SSH key, so at most one account can.
    '';
  };

  options.jitsusama.guide.keys = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          account = lib.mkOption {
            type = lib.types.str;
            description = "Whose key it is.";
          };
          file = lib.mkOption {
            type = lib.types.str;
            description = "Where it's kept, without the .tpm and .pub ssh-tpm-keygen adds.";
          };
          comment = lib.mkOption {
            type = lib.types.str;
            description = "What its public half says it is.";
          };
          pin = lib.mkOption {
            type = lib.types.bool;
            description = "Whether it asks for a PIN before it's used.";
          };
        };
      }
    );
    default = named;
    defaultText = lib.literalMD "every account's `jitsusama.tpmKeys`, from home-manager";
    description = ''
      The SSH keys the accounts keep in the TPM, which the guide makes at
      boot, as each account, while one is missing.
    '';
  };

  config = {
    assertions = [
      {
        assertion = lib.length config.jitsusama.guide.spares <= 1;
        message = "Only one account can keep its spare SSH key on the YubiKeys, since each holds one, and ${
          lib.concatMapStringsSep " and " (spare: spare.account) config.jitsusama.guide.spares
        } want to: set jitsusama.yubikeySpare = false for all but one.";
      }
    ];

    # Run by hand, the guide asks for root once through run0, which systemd
    # lets a person do only when polkit says they may.
    security.polkit.enable = true;

    environment.systemPackages = [ guide ];
    environment.etc."guide/machine.json".text = builtins.toJSON {
      disk_name = config.jitsusama.disk.name;
      inherit (config.jitsusama.guide) keys spares;
    };

    # Every boot, before anyone signs in, the guide looks at the machine on
    # the first console, as root, so nothing has to be remembered or asked
    # for. It shows only when there's something to do: the first boot after
    # the install, or a TPM seal the boot no longer matches. Otherwise it
    # leaves at once. It follows the unit that makes the disk's measured-boot
    # policy, which its seal has to match, and lanzaboote's, which restarts
    # the first boot to enrol the keys, and Plymouth's quitting, whose
    # splash would hide it, and holds the sign-in back until it's done;
    # Ctrl+C leaves it for the next boot.
    #
    # Where the console is the screen the guide draws on, systemd's lines
    # about the units still starting would land across it, so it asks the
    # manager to hold them while it runs and to go back to the machine's own
    # setting however it leaves, as org.freedesktop.systemd1(5) suggests for
    # SetShowStatus.
    systemd.services.guide-installed = {
      description = "Finish the machine the guide installed";
      wantedBy = [ "multi-user.target" ];
      after = [
        "plymouth-quit.service"
        "prepare-sb-auto-enroll.service"
        "systemd-pcrlock-make-policy.service"
        "systemd-user-sessions.service"
      ];
      before = [
        "display-manager.service"
        "getty@tty1.service"
      ];
      serviceConfig = {
        Type = "oneshot";
        ExecStartPre = "${showStatus} no";
        ExecStart = "${guide}/bin/guide installed";
        ExecStopPost = "${showStatus} ''";
        StandardInput = "tty";
        StandardOutput = "tty";
        StandardError = "journal";
        TTYPath = "/dev/tty1";
        TTYReset = true;
        TTYVHangup = true;
        TTYVTDisallocate = true;
        Environment = [ "TERM=linux" ];
        # Its journal and log, beside the rescue bundle the install left.
        StateDirectory = "guide";
      };
    };
  };
}
