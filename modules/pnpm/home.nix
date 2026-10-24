# pnpm's store for every install on a Linux machine. pnpm keeps its store at
# the top of the device a project sits on, and agents' work trees and quest
# workspaces are each a Btrfs subvolume, which counts as a device of its own.
# So each new tree made a store inside itself, 460 MB for
# agentic-harness.core, and downloaded every package again. Named here, the
# one store serves every tree, and pnpm clones its files into a subvolume, as
# Btrfs shares extents between the subvolumes of one filesystem: an install in
# a new tree then took 4.6 s and made no store of its own. It is the store
# pnpm uses by default, the one the disk guard trims.
#
# It is said in the environment rather than in pnpm's config files, which are
# the ones `pnpm config set` writes. pnpm 10 reads `npm_config_store_dir` and
# pnpm 11 on reads `pnpm_config_store_dir`, each ignoring the other's, so both
# are set, for shells and for what the user manager starts.
{ config, ... }:
let
  store = "${config.xdg.dataHome}/pnpm/store";
  variables = {
    npm_config_store_dir = store;
    pnpm_config_store_dir = store;
  };
in
{
  home.sessionVariables = variables;
  systemd.user.sessionVariables = variables;
}
