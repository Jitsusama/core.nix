# The checks of key-counts, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# The keyboards it reads are Linux's input devices.
onLinux {
  # key-counts' tests run as it builds: the same keys pressed in any order
  # keep the same file, which holds a total for each keyboard, layer,
  # modifiers and key and nothing more; a key held until it repeats counts
  # one hold; and a restart carries on from the file.
  key-counts-keeps-no-order = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in key-counts, its tests included.
  key-counts-passes-clippy = passesClippy "key-counts-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
