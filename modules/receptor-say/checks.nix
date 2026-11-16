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
  # receptor's arguments alone; said off, each count is none; a refusal
  # stops there; and only a declared address or login, ASCII case aside,
  # is taken as the source's own.
  receptor-say-gives-mycelium-words-and-numbers-alone = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in receptor-say, its tests included.
  receptor-say-passes-clippy = passesClippy "receptor-say-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
