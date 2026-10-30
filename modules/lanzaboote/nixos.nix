# Secure Boot with keys that never leave the machine, and a disk that unlocks
# only when the machine booted what it should. lanzaboote signs every
# generation it installs, and seals the disk's TPM key to a systemd-pcrlock
# policy it rewrites after every rebuild. How a machine gets there the first
# time is in docs/installing.md.
{ lanzaboote }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  enrolment = config.boot.lanzaboote.autoEnrollKeys;
  lanzabootePackages = lanzaboote.packages.${pkgs.stdenv.hostPlatform.system};

  # Leaving Microsoft's keys out makes lanzaboote pass sbctl
  # --yes-this-might-brick-my-machine, which also skips sbctl's refusal to
  # enrol keys that would stop the firmware running an option ROM it ran
  # this boot. This asks sbctl that question on its own. It exports to a
  # scratch directory, which needs no setup mode, so it changes nothing.
  optionRomCheck = pkgs.writeShellApplication {
    name = "option-rom-check";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.sbctl
    ];
    text = builtins.readFile ./option-rom-check.sh;
  };

  # Where lanzaboote leaves the keys' files for systemd-boot to enrol.
  autoKeys = "${config.boot.loader.efi.efiSysMountPoint}/loader/keys/auto";

  # Whether the programs the firmware starts from the ESP are signed by the
  # machine's keys, which it enforces once systemd-boot enrols them: a
  # restart to enrol them with one unsigned would leave a machine the
  # firmware refuses to start. Those are systemd-boot, at its own path and
  # the fallback one, and each generation's stub; the kernels and initrds
  # beside them are checked by the stub, by their hashes, and a kernel an
  # earlier boot loader left in EFI/nixos starts nothing.
  #
  # Refused, it takes the keys' files away as well, since systemd-boot
  # enrols whatever it finds there at the next start, whoever restarts;
  # that boot readies them again and checks once more.
  signedCheck = pkgs.writeShellApplication {
    name = "secure-boot-signed-check";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.jq
      pkgs.sbctl
    ];
    runtimeEnv = {
      # Named, or sbctl looks for the ESP with lsblk.
      ESP_PATH = config.boot.loader.efi.efiSysMountPoint;
      AUTO_KEYS = autoKeys;
    };
    text = builtins.readFile ./signed-check.sh;
  };

  # How many boots a new generation gets to reach multi-user.target before
  # systemd-boot passes it over for the last one that did.
  tries = 3;

  # Where the boot says a newer generation never finished booting, for the
  # session to hear.
  fellBack = "/run/boot-fell-back";

  # Says when the generation booted isn't the newest because a newer one ran
  # out of tries, in the journal as a warning and in a file the session reads
  # (fell-back.sh).
  fellBackCheck = pkgs.writeShellApplication {
    name = "boot-fell-back";
    runtimeInputs = [
      config.systemd.package
      pkgs.jq
    ];
    runtimeEnv = {
      TRIES = tries;
      FELL_BACK = fellBack;
      FELL_BACK_SAID = "${./fell-back.jq}";
    };
    text = builtins.readFile ./fell-back.sh;
  };

  # What the first boot does to have systemd-boot enrol the keys, as a
  # program an installer runs inside the installed system before that boot,
  # so systemd-boot enrols them at the very first, before any disk asks for
  # a PIN. It's the unit's own commands, read from it, so the two can't
  # differ; the unit then finds the files there and does nothing.
  prepared = config.systemd.services.prepare-sb-auto-enroll;
  enrolBeforeFirstBoot = pkgs.writeShellScript "secure-boot-enrolment" ''
    set -euo pipefail
    export PATH=${lib.escapeShellArg prepared.environment.PATH}
    ${prepared.preStart}
    cd "$(mktemp -d)"
    ${prepared.script}
  '';

  # Takes PCR 0 out of the TPM's policy as the machine shuts down with a
  # firmware update from fwupd waiting (firmware-update.sh), making the
  # policy again as the boot does, by that unit's own command.
  firmwareUpdate = pkgs.writeShellApplication {
    name = "tpm-follows-a-firmware-update";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.efibootmgr
      pkgs.gnugrep
      pkgs.gnused
    ];
    runtimeEnv = {
      PCRLOCK = "${config.systemd.package}/lib/systemd/systemd-pcrlock";
      MAKE_POLICY = pkgs.writeShellScript "make-policy" (
        lib.last (lib.toList config.systemd.services.systemd-pcrlock-make-policy.serviceConfig.ExecStart)
      );
    };
    text = builtins.readFile ./firmware-update.sh;
  };

  # Gives the loader an entry of its own, first in BootOrder (loader-first.sh),
  # by the path lanzaboote installs its signed systemd-boot to.
  loaderFirst = pkgs.writeShellApplication {
    name = "loader-first-in-boot-order";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.efibootmgr
      pkgs.gnugrep
      pkgs.gnused
      pkgs.util-linux
    ];
    runtimeEnv = {
      ESP = config.boot.loader.efi.efiSysMountPoint;
      LOADER = "\\EFI\\systemd\\systemd-bootx64.efi";
      LABEL = "Linux Boot Manager";
    };
    text = builtins.readFile ./loader-first.sh;
  };

  # Every NvPCR this systemd defines, each pointed at /dev/null, which is how
  # systemd reads a definition it should ignore. Taken from the package, so a
  # new one in a later systemd is masked as well.
  maskedNvpcrs = pkgs.runCommand "nvpcrs-masked" { } ''
    mkdir "$out"
    for definition in ${config.systemd.package}/lib/nvpcr/*.nvpcr; do
      ln -s /dev/null "$out/$(basename "$definition")"
    done
  '';
in
{
  imports = [ lanzaboote.nixosModules.lanzaboote ];

  options.jitsusama.lanzaboote.nvpcrs = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = ''
      Whether systemd keeps its NvPCRs: TPM registers it measures the
      hardware, the disk's key and each login into, for a policy or a remote
      check to read. Nothing in core reads them, and on a firmware TPM the
      secret that anchors them costs a second and a half to unseal at every
      boot, before the machine reaches sysinit.target.
    '';
  };

  config = {
    # lanzaboote installs and signs systemd-boot itself.
    boot.loader.systemd-boot.enable = lib.mkForce false;
    boot.loader.efi.canTouchEfiVariables = true;
    # Boot the newest generation straight away, rather than after nixpkgs's
    # five seconds in the menu. Holding a key, such as Space, at power-on
    # brings the menu up, for an older generation.
    boot.loader.timeout = 0;
    # The signed kernel keeps the command line it was signed with under Secure
    # Boot, so the menu's editor could only offer an edit that does nothing.
    boot.loader.systemd-boot.editor = false;
    # The TPM and the PIN are asked for by the systemd initrd.
    boot.initrd.systemd.enable = true;

    boot.lanzaboote = {
      enable = true;
      pkiBundle = "/var/lib/sbctl";

      # The stub turns HWP on before it hashes the kernel and initrd, which
      # some firmware leaves off with the CPU at its lowest clock. The patch
      # says why.
      package = lanzabootePackages.lzbt.override {
        stub = lanzabootePackages.stub.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [ ./stub-hwp.patch ];
        });
      };

      # The first boot makes the keys, and systemd-boot enrols them into the
      # firmware on the next, so no key is ever made anywhere else.
      autoGenerateKeys.enable = true;
      autoEnrollKeys = {
        enable = true;
        # Restarting only when the firmware is in setup mode, below.
        autoReboot = false;
        # Only Joel's keys, so nothing Microsoft ever signed boots here, such as
        # a shim that shows a fake disk prompt to learn the PIN. lanzaboote
        # calls leaving Microsoft's keys out a risk of bricking, because a
        # graphics card's ROM that isn't trusted leaves no picture; a laptop's
        # picture comes from its firmware, whose setup screen can always
        # restore the factory keys. A machine with a graphics card needs
        # Microsoft's keys.
        includeMicrosoftKeys = false;
        allowBrickingMyMachine = true;
        # A machine whose firmware runs option ROMs sets this, so the firmware
        # keeps running them by their checksums from the TPM's event log. It
        # can't be on everywhere: sbctl refuses to enrol with it on a machine
        # whose firmware ran none. The check below says which one a machine
        # is.
        includeChecksumsFromTPM = lib.mkDefault false;
      };

      # The disk's TPM key opens only for the same firmware (PCR 0), the same
      # boot loader and signed kernel, initrd and command line (PCR 4), and the
      # same Secure Boot keys and state (PCR 7). systemd-pcrlock covers at most
      # four generations, and one of them is always the booted one: pcrlock
      # leaves PCR 4 out of the policy when it can't account for the boot it's
      # running in, until the next reboot.
      protectedSystem = "/run/booted-system";
      measuredBoot = {
        enable = true;
        pcrs = [
          0
          4
          7
        ];
      };
      configurationLimit = 4;
      # A new generation that can't boot, such as a kernel that panics or an
      # initrd that can't open the disk, is tried three times and then passed
      # over, rather than left for the boot menu, which systemd-boot never
      # shows for it.
      bootCounting.initialTries = tries;
    };

    # A boot counts once the machine's services are up, not just its disk.
    systemd.targets.boot-complete.after = [ "multi-user.target" ];
    # The last good generation boots with no tries to count, so nothing
    # blesses it and boot-complete.target is never reached; this runs on
    # every boot instead, reading only the other entries' tries.
    systemd.services.boot-fell-back = {
      description = "Say when a newer generation never finished booting";
      wantedBy = [ "multi-user.target" ];
      unitConfig.RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = lib.getExe fellBackCheck;
      };
    };
    # Each account but the system's hears it once a sign-in, from the file
    # appearing rather than from looking for it.
    systemd.user.paths.boot-fell-back = {
      description = "Hear when a newer generation never finished booting";
      wantedBy = [ "default.target" ];
      unitConfig.ConditionUser = "!@system";
      pathConfig.PathExists = fellBack;
    };
    systemd.user.services.boot-fell-back = {
      description = "Say a newer generation never finished booting";
      unitConfig.ConditionUser = "!@system";
      # Active until the session ends, so the path doesn't start it again.
      # The account's manager starts before anything shows notices, so it
      # waits for one to take the name, however long that is.
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        TimeoutStartSec = "infinity";
        ExecStart = lib.getExe (
          pkgs.writeShellApplication {
            name = "boot-fell-back-told";
            runtimeInputs = [
              pkgs.glib.bin
              pkgs.libnotify
            ];
            runtimeEnv.FELL_BACK = fellBack;
            text = builtins.readFile ./fell-back-told.sh;
          }
        );
      };
    };

    # A firmware update changes PCR 0, after which the TPM would refuse the
    # disk at every boot, and systemd-pcrlock can't make a policy from a
    # boot its policy doesn't cover. So a machine updating its firmware
    # through fwupd frees PCR 0 as it shuts down with the update waiting,
    # and the first boot on the new firmware locks it again. Started at
    # boot to do nothing, it does its work as it's stopped, after the TPM's
    # units and the filesystems, since no unit can start during shutdown.
    # Without measured boot, there's no policy to free.
    systemd.services.tpm-follows-a-firmware-update =
      lib.mkIf (config.services.fwupd.enable && config.boot.lanzaboote.measuredBoot.enable)
        {
          description = "Free PCR 0 for a firmware update fwupd has waiting";
          wantedBy = [ "multi-user.target" ];
          after = [
            "local-fs.target"
            "tpm2.target"
            "systemd-pcrlock-make-policy.service"
            "sys-firmware-efi-efivars.mount"
          ];
          unitConfig.ConditionSecurity = "measured-uki";
          serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = lib.getExe' pkgs.coreutils "true";
            ExecStop = lib.getExe firmwareUpdate;
          };
        };

    # The firmware measures into PCR 4 every boot entry it tries before the
    # one that starts, so whatever its own order puts ahead of our loader
    # is in PCR 4 too, and systemd-pcrlock, unable to account for it,
    # leaves PCR 4 out of the disk's policy. Each boot gives the loader an
    # entry of its own, first in BootOrder, so the firmware starts it
    # straight away on every machine, as it does wherever its own entry for
    # the disk happens to come first. A stick still starts from the
    # firmware's boot menu.
    systemd.services.loader-first-in-boot-order = {
      description = "Put the loader first in the firmware's boot order";
      wantedBy = [ "multi-user.target" ];
      after = [ "sys-firmware-efi-efivars.mount" ];
      unitConfig = {
        ConditionPathIsDirectory = "/sys/firmware/efi/efivars";
        RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
      };
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = lib.getExe loaderFirst;
      };
    };

    # Measured boot holds sysinit.target back while systemd-pcrlock makes
    # the disk's policy, two to three seconds, longest on new firmware. In
    # that wait every unit asked for pulls sysinit.target's wants in again,
    # and nixpkgs's wrappers unit, a oneshot that doesn't stay active, ran
    # five times in a second, hit its start limit and was left failed,
    # though each run worked. It runs once a boot now, as a unit that sets
    # the machine up should; a switch still runs it again whenever the
    # wrappers change, since its script's store path changes the unit.
    systemd.services.suid-sgid-wrappers.serviceConfig.RemainAfterExit =
      lib.mkIf config.boot.lanzaboote.measuredBoot.enable true;

    # systemd-boot enrols the keys it's given only while the firmware is in
    # setup mode, with Secure Boot off, so they're given only then, and the
    # machine restarts for it. A firmware that already enforces, as one
    # keeping the keys a reinstall carried does, needs neither.
    #
    # Once it enforces, the files go. Left on the ESP, with systemd-boot set
    # to enrol them by force, they put this machine's keys back the moment
    # anyone clears the firmware's to install it afresh, should its own disk
    # boot before the stick: a countdown before any disk asks for a PIN. A
    # firmware cleared on its own still gets them back, from the next boot
    # that gets past the disk.
    #
    # lanzaboote's install ends by predicting the TPM's policy for later
    # boots, and systemd-pcrlock refuses to rewrite a policy from a boot the
    # policy doesn't cover, as one whose firmware lost its keys isn't. By
    # then the keys' files are on the ESP and every boot program is signed,
    # so the restart goes by those, after a failure as after success, and
    # the policy stays the one the keys' next boot matches.
    systemd.services.prepare-sb-auto-enroll = lib.mkIf enrolment.enable {
      # OnSuccess only follows a unit that ends.
      serviceConfig.RemainAfterExit = lib.mkForce false;
      unitConfig.ConditionSecurity = "!uefi-secureboot";
      unitConfig.OnSuccess = [ "secure-boot-enrol-restart.service" ];
      unitConfig.OnFailure = [ "secure-boot-enrol-restart.service" ];
      # Without checksums or Microsoft's keys, the firmware would stop
      # running any option ROM it runs now, so enrolment goes ahead only when
      # it runs none. A machine that does run one keeps its factory keys and
      # is told what to set.
      preStart = lib.mkIf (!enrolment.includeMicrosoftKeys && !enrolment.includeChecksumsFromTPM) (
        lib.getExe optionRomCheck
      );
    };
    # At the system's top, where an installer finds it by the profile.
    system.systemBuilderCommands = lib.mkIf enrolment.enable ''
      ln -s ${enrolBeforeFirstBoot} $out/secure-boot-enrolment
    '';

    systemd.services.secure-boot-enrol-restart = lib.mkIf enrolment.enable {
      description = "Restart for systemd-boot to enrol the Secure Boot keys";
      unitConfig = {
        ConditionSecurity = "!uefi-secureboot";
        # Only with the keys' files there to enrol, which a failure before
        # they're made, such as an option ROM's refusal, leaves none of.
        ConditionPathExists = map (file: "${autoKeys}/${file}.auth") [
          "PK"
          "KEK"
          "db"
        ];
        RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
        SuccessAction = "reboot";
      };
      serviceConfig = {
        Type = "oneshot";
        ExecStart = lib.getExe signedCheck;
      };
    };

    systemd.services.secure-boot-enrolled = lib.mkIf enrolment.enable {
      description = "Take the enrolled Secure Boot keys' files off the ESP";
      wantedBy = [ "multi-user.target" ];
      unitConfig = {
        ConditionSecurity = "uefi-secureboot";
        ConditionPathExists = autoKeys;
        RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
      };
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${pkgs.coreutils}/bin/rm -r ${autoKeys}";
      };
    };

    # The install check runs it on a machine with checksums on, to see it
    # refuse an option ROM.
    system.build.optionRomCheck = optionRomCheck;

    # systemd-tpm2-setup keeps the secret that anchors the TPM's NvPCRs in
    # /var and on the ESP, where the next boot's initrd finds it. Its unit
    # isn't ordered after the ESP's mount, so a machine that mounts it late
    # finds no ESP, and every boot makes a new secret and overwrites the last.
    # It also asks the TPM to define each NvPCR at every boot and takes the
    # refusal that it already exists as its answer, which tss2 logs as an
    # error; systemd still reports its own failures without it.
    systemd.services.systemd-tpm2-setup = lib.mkIf config.jitsusama.lanzaboote.nvpcrs {
      unitConfig.RequiresMountsFor = [ config.boot.loader.efi.efiSysMountPoint ];
      environment.TSS2_LOG = "esys+none";
    };

    # With no NvPCR defined, systemd-tpm2-setup only checks the TPM's storage
    # key. logind measures each first login into the login NvPCR, and takes
    # that unit being masked as the machine not keeping NvPCRs.
    environment.etc.nvpcr = lib.mkIf (!config.jitsusama.lanzaboote.nvpcrs) {
      source = maskedNvpcrs;
    };
    systemd.units."systemd-pcrlogin@.service" = lib.mkIf (!config.jitsusama.lanzaboote.nvpcrs) {
      enable = false;
    };

    environment.systemPackages = [ pkgs.sbctl ];
  };
}
