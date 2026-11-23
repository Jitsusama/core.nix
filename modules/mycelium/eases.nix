# What the home realm's node may do to ease the machine when it's over its
# room and another has some (home.nix's `easesSleep` and `easesCarry`), and
# the arguments that tell the node so. It changes only when what easing may
# act on does, apart from how the node is run.
{
  config,
  lib,
  mycelium,
}:
let
  cfg = config.jitsusama.mycelium;
  guards = import ./carry-guards.nix { inherit lib; };
  # The node carries only where the source it's built from keeps the
  # person's and the leading sessions (carry-guards.nix).
  carries = cfg.easesCarry && guards.keepsTheLed mycelium;
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
        Whether the home realm's node makes the carries it chooses while
        this machine is over its room and another has some, under its own
        name, rather than only saying so. Mycelium, not this option, bounds
        what it carries: one parked session at a time, off the screen and
        part way through nothing, into a fence no weaker and behind no door
        wider than here's, as each machine declares them, and at most six a
        day. Only a parked session is carried, so with `easesSleep` off it
        carries only those the harness parked once its own rule held. The
        session's harness must take a carry from the node, or the carry is
        kept and told but never made. It never carries a session the person
        has written in or one leading others, nor one whose harness doesn't
        say which, and reads its limits back from what it keeps, so a
        restart lifts none. Those guards are mycelium's from its #381 on:
        where the source the node is built from has no line of code naming
        them, the node isn't told to carry and the switch warns, rather
        than the machine refusing to update. Off, the node only writes what
        it would do. To stop at once, set it false and switch, which
        restarts the node; a carry it already asked for is the harness's, so
        it goes on, and the sessions it carried stay where they went.
      '';
    };
  };

  # The node always writes, as it judges the machine, what it would do to
  # ease it, so its lines can be read before it may act. Where the machine
  # says so, and its source keeps the guards, it makes the carries it
  # chooses. Where the machine says so, it asks the harness to park the
  # idle sessions it would put to sleep, in the folder each session's
  # harness reads.
  args = [
    "--eases"
  ]
  ++ lib.optional carries "--eases-carry"
  ++ lib.optionals cfg.easesSleep [
    "--eases-sleep"
    "${config.xdg.stateHome}/pi/agentic-harness.pi/park-workflow/ease"
  ];

  # A warning rather than a refusal, since a refusal would stop the whole
  # machine updating, kernel and fixes too, over one flag the node can go
  # without. Only where a home realm runs a node, since elsewhere no node
  # is told anything.
  warnings = lib.optional (cfg.realm != null && cfg.easesCarry && !carries) (guards.missing mycelium);
}
