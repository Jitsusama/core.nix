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
  # pi-carry and pi-verbs answer --help, but they run from the harness pi-packages fetches
  # at run time, so the store holds no copy for this check to run. The harness's own tests
  # prove it instead: pi-carry's in tests/lib/internal/carry/help.test.ts and
  # tests/scripts/pi-carry.test.ts, and pi-verbs's in tests/lib/internal/verbs/asked.test.ts.
  # They stay listed, rather than core taking the harness as an input, which would rerun
  # every check for a help line.
  pi-carry = [ "helps" ];
  pi-verbs = [ "helps" ];
}
