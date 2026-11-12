# The daemons that broke a point of docs/daemons.md's convention before
# daemons-agree, each with the points it still breaks, waiting while it comes
# to agree. The check fails on a daemon breaking a point it doesn't wait on,
# and on one waiting on a point it keeps, so the list only shrinks.
{
  agents = [ "shown" ];
  calls = [
    "held"
    "named"
    "shown"
    "speaks"
  ];
  clipboard-history = [
    "held"
    "placed"
    "fits"
    "shown"
  ];
  dictation = [
    "placed"
    "shown"
    "speaks"
  ];
  google = [
    "shown"
    "speaks"
  ];
  location = [
    "named"
    "shown"
    "speaks"
  ];
  machine = [
    "shown"
    "speaks"
  ];
  meaning = [
    "held"
    "shown"
    "speaks"
  ];
  noise-cancelled = [
    "named"
    "shown"
    "speaks"
  ];
  notices = [ "shown" ];
  passwords = [
    "shown"
    "speaks"
  ];
  people = [ "speaks" ];
  programs = [
    "shown"
    "speaks"
  ];
  screen-text = [
    "held"
    "shown"
    "speaks"
  ];
  sound = [
    "shown"
    "speaks"
  ];
  timekeeping = [
    "placed"
    "shown"
    "speaks"
  ];
  timers = [
    "shown"
    "speaks"
  ];
  windows = [ "speaks" ];
}
