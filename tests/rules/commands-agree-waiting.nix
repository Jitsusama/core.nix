# The commands that broke a point of docs/commands.md's convention before
# commands-agree, each with the points it still breaks, waiting while it comes
# to agree. The check fails on a command breaking a point it doesn't wait on,
# and on one waiting on a point it keeps, so the list only shrinks.
{
  nix-offload = [
    "kit"
    "exits"
    "helps"
  ];
  # pi-carry and pi-verbs answer --help in agentic-harness.pi since its #1892, but they run
  # from the harness pi-packages fetches at run time, so the store holds no copy for this
  # check to run, and it judges neither their help nor how they fit. The harness's own tests
  # stand in: tests/scripts/pi-carry.test.ts runs pi-carry as its wrapper here does, holding
  # it to its help on stdout, nothing on stderr and exit 0, while
  # tests/lib/internal/verbs/asked.test.ts holds only pi-verbs's help text and what asks for
  # it. They stay listed rather than core taking the harness as an input, which would pin a
  # harness other than the one machines run, and rerun every check for a help line.
  pi-carry = [ "helps" ];
  pi-verbs = [ "helps" ];
}
