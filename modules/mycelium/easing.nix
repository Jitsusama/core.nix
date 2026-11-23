# How far the home realm's node may act to ease its machine (home.nix's
# `easesSleep` and `easesCarry`), and the flags that say so to the node. It
# changes only when what the node may do to ease a machine does, apart from
# how the node is run. The node always writes what it would do, so a day of
# those lines can be read before a machine lets it act; each act is the
# machine's own to allow.
{ lib }:
{
  options = {
    easesSleep = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether the home realm's node asks the harness to park the idle
        sessions it would put to sleep while this machine is over its room
        and another has some, rather than only saying so. The harness parks
        one only once its own rule holds, and it wakes on its next message.
        Off, the node only writes what it would do.
      '';
    };
    easesCarry = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Whether the home realm's node makes the carries it would make while
        this machine is held over its room and another has some, rather
        than only saying so: an idle session at a time, off the screen and
        nobody's but the agents', never one orchestrating others, at most
        one each half hour and six a day, as the mycelium this flake locks
        keeps them; one older than its #381 may take the person's or an
        orchestrator's, so a machine overriding that input to an older one
        leaves this off. The session's harness carries it only once nothing
        of its own would be lost, and says why it refused otherwise. Off,
        the node only writes what it would do.
      '';
    };
  };

  # The home realm's node's easing flags, for `cfg` and the account's state
  # folder: it asks the harness to park sessions in the folder each
  # session's harness reads, and makes its carries under its own name,
  # which each session's harness takes only from its own machine's node.
  args =
    cfg: stateHome:
    [ "--eases" ]
    ++ lib.optionals cfg.easesSleep [
      "--eases-sleep"
      "${stateHome}/pi/agentic-harness.pi/park-workflow/ease"
    ]
    ++ lib.optional cfg.easesCarry "--eases-carry";
}
