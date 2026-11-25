# The check of pi's state folders' own subvolumes, which host-checks.nix
# gathers beside the hosts': it changes with home.nix beside it and the
# hosts' names alone. That a named folder really moves is the subvolumes
# VM test's to show (tests/vm/disks).
{
  account,
  bareNixos,
  holds,
  lib,
  pkgs,
  proofOf,
  self,
  ...
}:
{
  # The folders every pi reads as it starts move before any pi host and
  # the node start, whichever realms an account names, and its worktrees
  # don't move at all; an account naming no host still moves them.
  pi-state-moves-before-its-hosts =
    let
      pi = "${pkgs.emptyDirectory}/pi";
      node = "${pkgs.emptyDirectory}/node";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
      naming =
        settings:
        account (bareNixos [
          self.nixosModules.workstation
          { home-manager.users.someone.jitsusama = settings; }
        ]);
      hosted = naming {
        pi.hosts = {
          one = { inherit pi node proof; };
          two = { inherit pi node proof; };
        };
        mycelium.realm = "mine";
      };
      bare = naming { };
      folders = [
        "mesh-workflow"
        "quest-workflow"
        "observability"
        "work-cut"
      ];
      unitOf = folder: "own-subvolume-local-state-pi-agentic-harnesspi-${folder}";
      writers = [
        "pi-host-one-attended.service"
        "pi-host-one-background.service"
        "pi-host-two-attended.service"
        "pi-host-two-background.service"
        "mycelium.service"
      ];
      sorted = names: lib.sort lib.lessThan (lib.toList names);
      movesBefore =
        home: expected: folder:
        let
          unit = home.systemd.user.services.${unitOf folder} or null;
        in
        unit != null
        && sorted (unit.Unit.Before or [ ]) == sorted expected
        && lib.elem "default.target" (lib.toList (unit.Install.WantedBy or [ ]));
      names = home: lib.attrNames home.systemd.user.services;
    in
    holds "pi-state-moves-before-its-hosts"
      (
        "pi's start-up state doesn't move into subvolumes of its own before the pi hosts "
        + "and the node, or its worktrees move too."
      )
      (
        lib.all (movesBefore hosted writers) folders
        && lib.all (movesBefore bare [ ]) folders
        && !(lib.elem (unitOf "work-trees") (names hosted))
        # Each writer named is a unit the account has.
        && lib.all (check: check.assertion) hosted.assertions
      );
}
