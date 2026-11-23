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
  # What mycelium's #381 brought, which keeps a carry off the sessions the
  # person writes in and those leading others: a session's `driven` beside
  # the lanes it `leads`, each file of its source with the words that say
  # it. One without them takes the flag and carries without those guards.
  # The source is read rather than its date or its name, since a branch cut
  # before #381 may be dated after it.
  guards = {
    "src/carry/ease.rs" = "pub driven: Option<bool>";
    "src/carry/easing.rs" = "fn leads(";
  };
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
        one each half hour and six a day, and only onto a fence kind as
        strict as the session's own, as an orchestrator's ordered carry
        goes. Kinds are as each machine declares them, and a Mac's strict
        reads more than Linux's, so a strict session carried from Linux to a
        Mac lands looser in what it may read and write. The node only tells
        the session's harness, which makes the carry itself once nothing of
        its own would be lost, and says why it refused otherwise. The guards
        are mycelium's, from its #381 on, read from the source a machine's
        node is built from, so with this on a machine whose mycelium lacks
        them fails to evaluate. Off, the node only writes what it would do.
        To stop at once, set it false and switch, which restarts the node;
        a carry it already asked for is the harness's, so it goes on, and
        the sessions it carried stay where they went.
      '';
    };
  };

  # Whether `mycelium`, the source a machine's node is built from, keeps
  # the carries easesCarry promises. A source with no file to read keeps
  # nothing, and one renaming either word fails core's check of its own
  # lock before any machine taking it is refused.
  keepsTheLed =
    mycelium:
    mycelium ? outPath
    && lib.all (
      file:
      let
        path = "${mycelium}/${file}";
      in
      builtins.pathExists path && lib.hasInfix guards.${file} (builtins.readFile path)
    ) (lib.attrNames guards);

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
