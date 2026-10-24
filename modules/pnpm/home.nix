# pnpm's settings for every install on a Linux machine. pnpm keeps its store
# at the top of the device a project sits on, and agents' work trees and
# quest workspaces are each a Btrfs subvolume, which counts as a device of its
# own. So each new tree made a store inside itself, 460 MB for
# agentic-harness.core, and downloaded every package again. Named here, the
# one store serves every tree, and pnpm clones its files into a subvolume, as
# Btrfs shares extents between the subvolumes of one filesystem: an install
# that made its own store took 4.6 s and made none. It is the store pnpm uses
# by default, the one the disk guard trims.
#
# pnpm 10 reads `rc` and pnpm 11 on reads `config.yaml`, each ignoring the
# other's, so both say the same.
{ config, ... }:
let
  store = "${config.xdg.dataHome}/pnpm/store";
in
{
  xdg.configFile."pnpm/rc".text = ''
    store-dir=${store}
  '';
  xdg.configFile."pnpm/config.yaml".text = ''
    storeDir: ${store}
  '';
}
