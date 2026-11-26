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
  # account names, a realm apart from the machine's own moving the same
  # folders of its own before its own node too; nothing else of the
  # harness's moves, its worktrees and every realm's mesh messages above
  # all; an account naming no host still moves them.
  pi-state-moves-before-any-pi-starts =
    let
      pi = "${pkgs.emptyDirectory}/pi";
      node = "${pkgs.emptyDirectory}/node";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
      naming =
        modules: settings:
        account (bareNixos [
          self.nixosModules.workstation
          {
            home-manager.users.someone = {
              imports = modules;
              jitsusama = settings;
            };
          }
        ]);
      # The agents daemon comes with the graphical role, so it's named here.
      hosted = naming [ self.homeModules.agents ] {
        pi.hosts = {
          one = { inherit pi node proof; };
          mine = { inherit pi node proof; };
        };
        mycelium = {
          realm = "mine";
          alsoJoins.one.links = [ "two" ];
        };
      };
      bare = naming [ ] { };
      prefix = "own-subvolume-local-state-pi-agentic-harnesspi";
      shared = [
        "mesh-workflow-presence"
        "mesh-workflow-leases"
        "mesh-workflow-runs"
        "quest-workflow"
        "observability"
        "work-cut"
      ];
      realm = map (folder: "realms-one-${folder}") [
        "mesh-workflow-presence"
        "mesh-workflow-leases"
        "mesh-workflow-runs"
        "quest-workflow"
      ];
      writers = [
        "pi-host-one-attended.service"
        "pi-host-one-background.service"
        "pi-host-mine-attended.service"
        "pi-host-mine-background.service"
        "mycelium.service"
      ];
      # The other writers each folder waits for, by name rather than by
      # what the account has, so one renamed where it's made is seen.
      sweeps = {
        observability = [ "spend-watch.service" ];
        work-cut = [ "pi-sweep.service" ];
      };
      others = {
        hosted =
          sweeps
          // {
            quest-workflow = [
              "agents.service"
              "pi-sweep.service"
            ];
          }
          // lib.genAttrs realm (_: [ "mycelium-one.service" ]);
        bare = sweeps // {
          quest-workflow = [ "pi-sweep.service" ];
        };
      };
      sorted = names: lib.sort lib.lessThan (lib.toList names);
      movesBefore =
        home: expected: others: folder:
        let
          unit = home.systemd.user.services."${prefix}-${folder}" or null;
          wanted = expected ++ others.${folder} or [ ];
        in
        unit != null && sorted (unit.Unit.Before or [ ]) == sorted wanted;
      # Every unit moving some of the harness's state, by its name, so a
      # folder named wider, its worktrees or messages in it, is seen.
      moving =
        home: sorted (lib.filter (lib.hasPrefix prefix) (lib.attrNames home.systemd.user.services));
      named = folders: sorted (map (folder: "${prefix}-${folder}") folders);
      # Only the subvolumes module's own assertions, that each writer named
      # is a unit the account has, so another module's says nothing here;
      # none found means they're worded otherwise now, which fails.
      ordered =
        home:
        let
          says = check: lib.hasInfix "'s move comes before" check.message;
          checks = lib.filter says home.assertions;
        in
        checks != [ ] && lib.all (check: check.assertion) checks;
    in
    holds "pi-state-moves-before-any-pi-starts"
      (
        "pi's start-up state doesn't move into subvolumes of its own before the pi hosts, "
        + "the node and its other writers, or more of the harness's state moves too."
      )
      (
        lib.all (movesBefore hosted writers others.hosted) (shared ++ realm)
        && lib.all (movesBefore bare [ ] others.bare) shared
        && moving hosted == named (shared ++ realm)
        && moving bare == named shared
        && ordered hosted
        && ordered bare
      );
}
