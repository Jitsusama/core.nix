# Gives the harness's state that every pi reads as it starts and on each
# turn Btrfs subvolumes of their own, beside home.nix rather than in it.
# A pi's screen waits for its extensions' handlers, and on 25 November,
# with the disk serving 10,900 reads a second at 4 ms each, screens took
# 24 s to 20 min to start, almost all of it in those handlers. Most of the
# processes waiting then waited on the home's one tree, on its root
# (btrfs_read_lock_root_node) or an inode's lock, behind every other writer
# there (modules/subvolumes/home.nix says what that lock cost). In a
# subvolume of its own, each folder's root is locked only by pi's own
# writes; how much of a start that saves is measured only after a login.
#
# Four folders, not the whole state: its worktrees, some 1,700 of them,
# couldn't be copied within any wait a login could give, so they stay in
# the home's tree. These four are what the start and turn paths read: the
# mesh's presence and messages, the quests' records, the run ledgers'
# SQLite files, and the work tool's records of the trees it cut.
#
# Every pi writes in them, and pi always runs, so a folder moves only at a
# login, before the account's pi hosts and the node start; a pi started
# by hand before the move keeps a file open there, and the folder stays
# until the next login that finds nothing holding it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  launch = import ./host-launch.nix { inherit lib; };
  state = lib.removePrefix "${config.home.homeDirectory}/" "${config.xdg.stateHome}/pi/agentic-harness.pi";
  folders = [
    "mesh-workflow"
    "quest-workflow"
    "observability"
    "work/cut"
  ];
  # Each host holds many conversations, and starts each conversation's
  # own unit itself, so a move before the hosts is before them all.
  hosts = lib.concatMap (
    realm: map (kind: "pi-host-${realm}-${kind}.service") (lib.attrNames launch.kinds)
  ) (lib.attrNames config.jitsusama.pi.hosts);
  # The node reads and writes the mesh's folders too, where it runs.
  node = lib.optional (
    lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null config != null
  ) "mycelium.service";
in
{
  imports = [ ../subvolumes/home.nix ];

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    jitsusama.subvolumes.folders = lib.genAttrs (map (folder: "${state}/${folder}") folders) (_: {
      before = hosts ++ node;
      # The run ledgers are the largest, and a reflink copies only their
      # extents' records, so two minutes is far past what each takes.
      seconds = 120;
    });
  };
}
