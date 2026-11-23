# What the home realm's node may do to ease the machine when it's over its
# room and another has some (home.nix's `easesSleep` and `easesCarry`), and
# the arguments that tell the node so. It changes only when what easing may
# act on does, apart from how the node is run.
{ config, lib }:
let
  cfg = config.jitsusama.mycelium;
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
        kept and told but never made. Off, the node only writes what it
        would do.
      '';
    };
  };

  # The node always writes, as it judges the machine, what it would do to
  # ease it, so its lines can be read before it may act. Where the machine
  # says so, it makes the carries it chooses. Where the machine says so,
  # it asks the harness to park the idle sessions it would put to sleep,
  # in the folder each session's harness reads.
  args = [
    "--eases"
  ]
  ++ lib.optional cfg.easesCarry "--eases-carry"
  ++ lib.optionals cfg.easesSleep [
    "--eases-sleep"
    "${config.xdg.stateHome}/pi/agentic-harness.pi/park-workflow/ease"
  ];
}
