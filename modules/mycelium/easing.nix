# How far the home realm's node may act to ease its machine (home.nix's
# `easesSleep` and `easesCarry`), and the flags that say so to the node. It
# changes only when what the node may do to ease a machine does, apart from
# how the node is run. The node always writes what it would do, a line each
# time its choice changes, to the `eases` folder in its state, a file a day,
# so those lines can be read before a machine lets it act; each act is the
# machine's own to allow. mycelium's PROTOCOL.md (A Machine Easing Itself)
# says what each choice holds to.
{ lib }:
let
  # When the first of mycelium's main to keep a carry off the sessions the
  # person writes in and those leading others was written: its #381, merged
  # as 8f1b0e66, whose lastModified this is. One older takes the flag and
  # makes carries without those guards, so it's a floor and not a name.
  sparesTheLed = 1795430328;
in
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
        one each half hour and six a day, and onto ground as firm as the
        session's fence, as an orchestrator's ordered carry is. Ground is
        judged as `declares` says, so a carry from Linux to a Mac lands
        looser in what it may read and write. The session's harness carries
        it only once nothing of its own would be lost, and says why it
        refused otherwise. It needs a mycelium from its #381 on, which
        keeps the person's and the leading sessions, so with this on a
        machine overriding that input to an older one fails to evaluate.
        Off, the node only
        writes what it would do. To stop at once, set it false and switch,
        which restarts the node; the sessions it carried stay where they
        went.
      '';
    };
  };

  # Whether `mycelium`, the flake input a machine's node is built from,
  # keeps the carries easesCarry promises. An input saying no time is
  # taken to keep nothing.
  keepsTheLed = mycelium: (mycelium.lastModified or 0) >= sparesTheLed;

  # The home realm's node's easing flags, for `cfg` and the account's state
  # folder: it asks the harness to park sessions in the folder each
  # session's harness reads, and makes its carries under its own name,
  # which each session's harness takes only from its own machine's node,
  # as mycelium's PROTOCOL.md says.
  args =
    cfg: stateHome:
    [ "--eases" ]
    ++ lib.optionals cfg.easesSleep [
      "--eases-sleep"
      "${stateHome}/pi/agentic-harness.pi/park-workflow/ease"
    ]
    ++ lib.optional cfg.easesCarry "--eases-carry";
}
