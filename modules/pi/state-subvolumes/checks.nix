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
      hosts = {
        one = { inherit pi node proof; };
        two = { inherit pi node proof; };
        mine = { inherit pi node proof; };
      };
      # The agents daemon comes with the graphical role, so it's named
      # here; the shell, which comes with it too, isn't, and its watch is
      # named in the module unchecked. Realm one is joined, two isn't.
      hosted = naming [ self.homeModules.agents ] {
        pi.hosts = hosts;
        mycelium = {
          realm = "mine";
          alsoJoins.one.links = [ "two" ];
        };
      };
      # With no home realm named, every realm is home, so none moves apart.
      homeless = naming [ ] { pi.hosts = hosts; };
      bare = naming [ ] { };
      harness = ".local/state/pi/agentic-harness.pi";
      shared = [
        "mesh-workflow/presence"
        "mesh-workflow/leases"
        "mesh-workflow/runs"
        "quest-workflow"
        "observability"
        "work/cut"
      ];
      ofRealm =
        realm:
        map (folder: "realms/${realm}/${folder}") [
          "mesh-workflow/presence"
          "mesh-workflow/leases"
          "mesh-workflow/runs"
          "quest-workflow"
        ];
      realm = ofRealm "one" ++ ofRealm "two";
      homeNode = [ "mycelium.service" ];
      hostsOf = lib.concatMap (realm: [
        "pi-host-${realm}-attended.service"
        "pi-host-${realm}-background.service"
      ]);
      writers = hostsOf [
        "one"
        "two"
        "mine"
      ];
      # The other writers each folder waits for, by name rather than by
      # what the account has, so one renamed where it's made is seen.
      sweeps = {
        observability = [ "spend-watch.service" ];
        "work/cut" = [ "pi-sweep.service" ];
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
          // lib.genAttrs (ofRealm "one") (_: [ "mycelium-one.service" ]);
        homeless = sweeps // {
          quest-workflow = [ "pi-sweep.service" ];
        };
      };
      sorted = names: lib.sort lib.lessThan (lib.toList names);
      # Each folder by its path, not its unit's name, which can't tell
      # work/cut from work-cut.
      folders = home: home.jitsusama.subvolumes.folders;
      movesBefore =
        home: expected: others: folder:
        let
          moved = (folders home)."${harness}/${folder}" or null;
          wanted = expected ++ others.${folder} or [ ];
        in
        moved != null && sorted moved.before == sorted wanted;
      # Every folder moving some of the harness's state, or holding it, so
      # a folder named wider, its worktrees or messages in it, is seen.
      moving =
        home:
        sorted (
          lib.filter (folder: lib.hasPrefix "${harness}/" folder || lib.hasPrefix folder harness) (
            lib.attrNames (folders home)
          )
        );
      named = paths: sorted (map (folder: "${harness}/${folder}") paths);
      # Only the assertions that a harness folder's writers are units the
      # account has, so another module's says nothing here; none found
      # means they're worded otherwise now, which fails.
      ordered =
        home:
        let
          says =
            check:
            let
              inherit (check) message;
            in
            lib.hasPrefix "~/${harness}/" message && lib.hasInfix "'s move comes before" message;
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
        lib.all (movesBefore hosted (writers ++ homeNode) others.hosted) (shared ++ realm)
        && lib.all (movesBefore homeless writers others.homeless) shared
        && lib.all (movesBefore bare [ ] others.homeless) shared
        && moving hosted == named (shared ++ realm)
        && moving homeless == named shared
        && moving bare == named shared
        && ordered hosted
        && ordered homeless
        && ordered bare
      );
}
