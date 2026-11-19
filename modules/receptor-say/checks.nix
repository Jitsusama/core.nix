# The checks of receptor-say, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: mycelium is run by its absolute path with
  # fixed words, a realm of its letters and counts held to u32, each
  # receptor's arguments alone, with any words its caller adds put after
  # its own and given as built; said off, each count is none; a refusal
  # stops there; and only a declared address or login, ASCII case aside,
  # is taken as the source's own. What a caller adds is proved by its own
  # crate's check, as repos-reads-the-disk proves the GitHub receptor's.
  receptor-say-gives-mycelium-words-and-numbers-alone = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in receptor-say, its tests included.
  receptor-say-passes-clippy = passesClippy "receptor-say-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
