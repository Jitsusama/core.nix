# The commands that broke a point of docs/commands.md's convention before
# commands-agree, each with the points it still breaks, waiting while it comes
# to agree. The check fails on a command breaking a point it doesn't wait on,
# and on one waiting on a point it keeps, so the list only shrinks.
{
  # pi-carry answers --help in agentic-harness.pi since its #1892, and pi-verbs since its
  # #1717, but both run from the harness pi-packages fetches at run time. The store holds no
  # copy for this check to run, so it judges neither their help nor how they fit, and reads
  # their exits and inks only from core's one-line wrapper. For help alone, the harness's own
  # tests stand in: agentic-harness.pi/tests/scripts/pi-carry.test.ts runs pi-carry under node
  # with the flag its wrapper here passes, holding it to its help on stdout, nothing on stderr
  # and exit 0, while agentic-harness.pi/tests/lib/internal/verbs/asked.test.ts holds only
  # pi-verbs's help text and what asks for it. They stay listed rather than core taking the
  # harness as an input, which would pin a harness other than the one machines run, and rerun
  # every check for a help line.
  pi-carry = [ "helps" ];
  pi-verbs = [ "helps" ];
}
