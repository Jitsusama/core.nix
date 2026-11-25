# mycelium on NixOS: the link keys system.nix pins, and the folder for the
# socket agents of another account reach the node through (home.nix's
# `agentsUid`). A node runs as its account's own service, whose runtime
# folder no other account may enter, and on a machine lent for agents, such
# as a cloud VM, their sessions run as an account of their own, so the
# socket they reach their node through
# can't go there. It goes in /run/mycelium, made at boot for the account
# alone to write, as the node demands of the folder before it binds
# (mycelium's node/worker): a folder another account could change would let
# it put a socket of its own in the node's place.
{ mycelium }:
{ config, lib, ... }:
let
  cfg = config.jitsusama.mycelium;
in
{
  imports = [ (import ./system.nix { inherit mycelium; }) ];

  config = lib.mkIf (cfg.account != null) {
    systemd.tmpfiles.rules = [ "d /run/mycelium 0755 ${cfg.account} root -" ];
  };
}
