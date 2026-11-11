# The checks of daemon-serve, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: a watch is answered whole and then told
  # only the records that moved and the keys that went; a watch of one
  # place hears only that place's rows; a paced watch hears all that moved
  # in one line a pace, and asked again at once hears what it held back;
  # and an ask of the daemon's own is the daemon's to answer, one it
  # doesn't know refused.
  daemon-serve-tells-each-watch-what-moved = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in daemon-serve, its tests included.
  daemon-serve-passes-clippy = passesClippy "daemon-serve-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
