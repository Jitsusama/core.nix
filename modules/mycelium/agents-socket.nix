# The socket a machine's agents reach the home realm's node through when
# they run as an account apart from the node's, as on a cloud VM (home.nix's
# `agentsUid`), and the arguments that give the node it. It changes only
# when how such agents reach their node does, apart from how the node is run.
{
  config,
  lib,
  isLinux,
}:
let
  cfg = config.jitsusama.mycelium;
in
{
  options.agentsUid = lib.mkOption {
    type = lib.types.nullOr lib.types.ints.unsigned;
    default = null;
    example = 1001;
    description = ''
      The account whose sessions the home realm's node answers through a
      socket of their own, at `/run/mycelium/<realm>-worker.sock`, or null
      for none. A machine lent for agents, such as a cloud VM, runs their
      sessions as an account apart from the node's, which can't reach the
      node's own socket; the node refuses a process of any other account
      there. Linux only, on a machine whose NixOS module names the node's
      account in `jitsusama.mycelium.account`, which makes the folder.
    '';
  };

  # The node's own flags still say worker, mycelium's word for a machine
  # lent for agents. The socket is outside the node's runtime folder, which
  # the agents' account can't enter, in the folder the machine's NixOS
  # module makes for the node's account (nixos.nix).
  args = lib.optionals (cfg.agentsUid != null) [
    "--worker-socket"
    "/run/mycelium/${cfg.realm}-worker.sock"
    "--worker-uid"
    (toString cfg.agentsUid)
  ];

  assertions = [
    {
      assertion = cfg.agentsUid != null -> (isLinux && cfg.realm != null);
      message = "jitsusama.mycelium.agentsUid needs Linux, whose NixOS module makes /run/mycelium, and jitsusama.mycelium.realm, the realm whose node answers its sessions.";
    }
  ];
}
