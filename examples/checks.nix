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
  self,
  ...
}:
{
  example-nixos = evaluates "example-nixos" (
    import ./nixos.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  # Evaluated once in checks.nix, since the rules' daemons-agree reads it too.
  example-laptop = evaluates "example-laptop" exampleLaptop;

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
  example-apple-laptop = evaluates "example-apple-laptop" (
    import ./apple-laptop.nix {
      core = self;
      inherit nixpkgs;
    }
  );
  example-darwin = evaluates "example-darwin" (
    import ./darwin.nix {
      core = self;
      inherit nix-darwin;
    }
  );
}
