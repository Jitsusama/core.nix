# The checks of the example machines, which use core only through its
# outputs and so evaluate every module there is; checks.nix gathers them with
# every other folder's.
{
  evaluates,
  exampleLaptop,
  holds,
  lib,
  nix-darwin,
  nixpkgs,
  pkgs,
  self,
  ...
}:
let
  appleLaptop = import ./apple-laptop.nix {
    core = self;
    inherit nixpkgs;
  };
in
{
  example-nixos = evaluates "example-nixos" (
    import ./nixos.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  # Evaluated once in checks.nix, since the rules' daemons-agree reads it too.
  example-laptop = evaluates "example-laptop" exampleLaptop;
  # The laptop's initrd alone, by its derivation, so changed-checks says when
  # a change moves what boots and unlocks the disk: merge-net needs then runs
  # the VM tests that boot through it before the change merges. #4513 moved
  # it through a library hibernate-unlock reads, which no area names. Only
  # the path is written, so building it builds no initrd.
  example-laptop-initrd = pkgs.writeText "example-laptop-initrd" (
    builtins.unsafeDiscardStringContext exampleLaptop.config.system.build.initialRamdisk.drvPath
  );
  # The laptop's kernel and Qt alone, by their derivations, as its initrd is,
  # so changed-checks says when a change moves a build that takes a builder
  # past half an hour: merge-net needs then asks for a machine's system built
  # with the change before it merges. #6748's kernel held every switch from
  # 17:14 to about 22:15 on 26 November, as each main after it compiled it
  # again. A nixpkgs bump or an overlay under every package moves these too.
  example-laptop-kernel = pkgs.writeText "example-laptop-kernel" (
    builtins.unsafeDiscardStringContext exampleLaptop.config.boot.kernelPackages.kernel.drvPath
  );
  example-laptop-qt = pkgs.writeText "example-laptop-qt" (
    builtins.unsafeDiscardStringContext (
      lib.concatMapStringsSep "\n" (qt: qt.drvPath) [
        exampleLaptop.pkgs.qt6.qtbase
        exampleLaptop.pkgs.qt6.qtdeclarative
      ]
    )
  );

  # A system account whose manager runs, as one lingering for a builder's
  # logins does, starts none of a person's services: their home is
  # /var/empty, so a unit keeping files there fails and restarts all day, as
  # linux-id did 2,700 times in a builder's account on 15 November. Each user
  # unit core starts by itself says whose it is, with ConditionUser.
  system-accounts-start-no-persons-services =
    let
      inherit (exampleLaptop.config.systemd) user;
      # nixpkgs' own, which core leaves as nixpkgs has them: what every
      # manager needs (its bus, activation and tmpfiles' cleaning), and the
      # sockets of sound, podman and speech, which start nothing until
      # something connects.
      theirs = [
        "nixos-activation.service"
        "systemd-tmpfiles-clean.timer"
        "dbus.socket"
        "pipewire.socket"
        "pipewire-pulse.socket"
        "podman.socket"
        "speech-dispatcher.socket"
      ];
      startsItself =
        unit:
        lib.any (target: lib.elem target unit.wantedBy) [
          "default.target"
          "timers.target"
          "sockets.target"
          "paths.target"
        ];
      unsaid =
        kind: units:
        lib.mapAttrsToList (name: _: "${name}.${kind}") (
          lib.filterAttrs (
            name: unit:
            startsItself unit && !(unit.unitConfig ? ConditionUser) && !(lib.elem "${name}.${kind}" theirs)
          ) units
        );
      found = lib.concatLists [
        (unsaid "service" user.services)
        (unsaid "timer" user.timers)
        (unsaid "socket" user.sockets)
        (unsaid "path" user.paths)
      ];
    in
    holds "system-accounts-start-no-persons-services"
      "every account's manager starts these, a system account's too, with no ConditionUser saying whose they are: ${lib.concatStringsSep ", " found}"
      (found == [ ]);
  example-apple-laptop = evaluates "example-apple-laptop" appleLaptop;
  # Asahi's kernel, as example-laptop-kernel is the XPS's.
  example-apple-laptop-kernel = pkgs.writeText "example-apple-laptop-kernel" (
    builtins.unsafeDiscardStringContext appleLaptop.config.boot.kernelPackages.kernel.drvPath
  );
  example-darwin = evaluates "example-darwin" (
    import ./darwin.nix {
      core = self;
      inherit nix-darwin;
    }
  );
}
