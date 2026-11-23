# Whether a mycelium source keeps a carry off the sessions the person writes
# in and those leading others, as its #381 on does, so eases.nix tells the
# node to carry only where it does. A machine on core's own lock always
# has them; this is for one overriding core's mycelium input. It changes
# only when what marks those guards in mycelium's source does.
{ lib }:
let
  # What mycelium's #381 brought: a session's `driven` beside the lanes it
  # `leads`, each file of its source with a line of code that says it, as a
  # pattern a whole line matches, so a line comment naming either doesn't.
  # Text inside a block comment, a string or code compiled out still
  # counts, since the lines are read as text, not as Rust. One without
  # them takes the flag and carries without those guards. The source is
  # read rather than its date or its name, since a branch cut before #381
  # may be dated after it. This shows the code names them, not that it
  # calls them, and of the source the module was given, assuming its
  # package is built from that, as mycelium's own flake does.
  guards = {
    "src/carry/ease.rs" = "[[:space:]]*pub(\\([a-z:]+\\))? driven: Option<bool>,.*";
    "src/carry/easing.rs" = "[[:space:]]*(pub(\\([a-z:]+\\))? )?fn leads[(<].*";
  };
  said =
    pattern: text: lib.any (line: builtins.match pattern line != null) (lib.splitString "\n" text);
  # Why `file` of `mycelium` doesn't keep its guard, or null where it does.
  short =
    mycelium: file:
    let
      path = "${mycelium}/${file}";
    in
    if !(builtins.pathExists path) then
      "${file} is missing"
    else if said guards.${file} (builtins.readFile path) then
      null
    else
      "${file} has no line of code matching ${guards.${file}}";
  shorts =
    mycelium:
    if mycelium ? outPath then
      lib.filter (why: why != null) (map (short mycelium) (lib.attrNames guards))
    else
      [ "it has no source to read" ];
in
{
  # Whether `mycelium`, the source a machine's node is built from, has
  # each guard's line. A source with no file to read has none.
  keepsTheLed = mycelium: shorts mycelium == [ ];

  # What the switch says where easesCarry is set and `mycelium` lacks
  # them: which mycelium it read and which guard each file lacks. Core's
  # own check reads its lock's source, but a machine overriding the input
  # to a newer mycelium that wrote either line in a form the patterns
  # don't accept is warned here first, so the warning names what was
  # looked for.
  missing =
    mycelium:
    "jitsusama.mycelium.easesCarry is set, but the mycelium the node is built from ("
    + (mycelium.rev or mycelium.dirtyRev or "a source with no revision")
    + ") lacks the guards mycelium's #381 brought to keep a carry off the sessions the person "
    + "writes in and those leading others: "
    + lib.concatStringsSep "; " (shorts mycelium)
    + ". So the node only writes the carries it would make. Take a mycelium with them, or where "
    + "it has them in other words, change carry-guards.nix.";
}
