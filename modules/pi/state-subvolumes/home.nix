# Gives the harness's state that every pi reads as it starts and on each
# turn Btrfs subvolumes of their own, in a folder of its own beside pi's
# home.nix.
# A pi's screen waits for its extensions' handlers, and on 25 November,
# with the disk serving 10,900 reads a second at 4 ms each, screens took
# up to 20 min to start, almost all of it in those handlers. Most of the
# processes waiting then waited on the home's one tree, on its root
# (btrfs_read_lock_root_node) or an inode's lock, behind every other writer
# there (modules/subvolumes/home.nix says what that lock cost). In a
# subvolume of its own, each folder's root is locked only by its own
# writers; how much of a start that saves is measured only after a boot.
#
# The folders are those the start and turn paths write and read: the
# mesh's presence, leases and runs, the quests' session records, the run
# ledgers' SQLite files, the work tool's records of the trees it cut, and
# each further realm's own state. Some stay in the home's tree, since a
# copy each boot must fit the wait every pi start gives it: the worktrees,
# some 1,700 of them, and the mesh's messages, 135,907 files on the
# workstation measured on 26 November, where the largest folder moved here held 6,180. At the
# 12,000 files in 25 s Chrome's copy took on a loaded disk, that is about
# 13 s, and two minutes is the most any is given.
#
# Every pi writes in them, and pi always runs, so a folder moves only as
# the account's manager starts, which a lingering account's does at boot
# alone, before its pi hosts, the node and the folder's other writers
# start; a pi started by hand before the move keeps a file open there, and
# the folder stays until the next start that finds nothing holding it.
# A subvolume's edge refuses a rename across it. Where the harness's mesh,
# quest and work records rename, read on 26 November, they stage a file
# beside where it lands or file a message into its day's folder, so none
# crosses these folders' edges.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  launch = import ../host-launch.nix { inherit lib; };
  harness = "${config.xdg.stateHome}/pi/agentic-harness.pi";
  state = lib.removePrefix "${config.home.homeDirectory}/" harness;
  realms = lib.attrNames config.jitsusama.pi.hosts;
  homeRealm = lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null config;
  # Each host holds many conversations, and starts each conversation's
  # own unit itself, so a move before the hosts is before them all.
  hosts = lib.concatMap (
    realm: map (kind: "pi-host-${realm}-${kind}.service") (lib.attrNames launch.kinds)
  ) realms;
  # The node reads and writes the mesh's folders too, where it runs.
  node = lib.optional (homeRealm != null) "mycelium.service";
  # Units outside pi's hosts that write a folder, named only where the
  # account has them: a move named before a unit it hasn't is refused.
  present = lib.filter (unit: config.systemd.user.services ? ${lib.removeSuffix ".service" unit});
  # The sweep removes ended sessions' trees and their records, and the
  # agents daemon watches orchestrators.json, a watch an exchange of the
  # folder under it would leave on the old one; named, it holds the move
  # back at a switch while it runs, as a writer does.
  writers = {
    "mesh-workflow/presence" = [ ];
    "mesh-workflow/leases" = [ ];
    "mesh-workflow/runs" = [ ];
    quest-workflow = present [
      "agents.service"
      "pi-sweep.service"
    ];
    observability = present [ "spend-watch.service" ];
    "work/cut" = present [ "pi-sweep.service" ];
  }
  # A realm other than the machine's own keeps its state under realms/,
  # whole, and its folders are small, so each moves as one.
  // lib.genAttrs (map (realm: "realms/${realm}") (
    lib.filter (realm: homeRealm != null && realm != homeRealm) realms
  )) (_: [ ]);
in
{
  imports = [ ../../subvolumes/home.nix ];

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    jitsusama.subvolumes.folders = lib.mapAttrs' (
      folder: others:
      lib.nameValuePair "${state}/${folder}" {
        before = hosts ++ node ++ others;
        # The largest is the run ledgers, and a reflink copies only their
        # extents' records, so two minutes is far past what each takes.
        seconds = 120;
      }
    ) writers;
  };
}
