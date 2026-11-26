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
  # The folders every pi reads as it starts move before any pi host, the
  # node and each folder's other writers start, whichever realms an
  # account names, a realm apart from the machine's own moving its state
  # whole; nothing else of the harness's moves, its worktrees and the
  # mesh's messages above all; an account naming no host still moves them.
  pi-state-moves-before-any-pi-starts =
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
          mine = { inherit pi node proof; };
        };
        mycelium.realm = "mine";
      };
      bare = naming { };
      prefix = "own-subvolume-local-state-pi-agentic-harnesspi";
      shared = [
        "mesh-workflow-presence"
        "mesh-workflow-leases"
        "mesh-workflow-runs"
        "quest-workflow"
        "observability"
        "work-cut"
      ];
      writers = [
        "pi-host-one-attended.service"
        "pi-host-one-background.service"
        "pi-host-mine-attended.service"
        "pi-host-mine-background.service"
        "mycelium.service"
      ];
      # The other writers each folder waits for, where the account has them.
      others =
        home:
        let
          has = unit: lib.optional (home.systemd.user.services ? ${unit}) "${unit}.service";
        in
        {
          quest-workflow = has "agents" ++ has "pi-sweep";
          observability = has "spend-watch";
          work-cut = has "pi-sweep";
        };
      sorted = names: lib.sort lib.lessThan (lib.toList names);
      movesBefore =
        home: expected: folder:
        let
          unit = home.systemd.user.services."${prefix}-${folder}" or null;
        in
        unit != null
        && sorted (unit.Unit.Before or [ ]) == sorted (expected ++ (others home).${folder} or [ ]);
      # Every unit moving some of the harness's state, by its name, so a
      # folder named wider, its worktrees or messages in it, is seen.
      moving =
        home: sorted (lib.filter (lib.hasPrefix prefix) (lib.attrNames home.systemd.user.services));
      named = folders: sorted (map (folder: "${prefix}-${folder}") folders);
      # Only the subvolumes module's own assertions, that each writer named
      # is a unit the account has, so another module's says nothing here.
      ordered =
        home:
        lib.all (check: check.assertion) (
          lib.filter (check: lib.hasInfix "'s move comes before" check.message) home.assertions
        );
    in
    holds "pi-state-moves-before-any-pi-starts"
      (
        "pi's start-up state doesn't move into subvolumes of its own before the pi hosts, "
        + "the node and its other writers, or more of the harness's state moves too."
      )
      (
        lib.all (movesBefore hosted writers) (shared ++ [ "realms-one" ])
        && lib.all (movesBefore bare [ ]) shared
        && moving hosted == named (shared ++ [ "realms-one" ])
        && moving bare == named shared
        && ordered hosted
        && ordered bare
      );
}
