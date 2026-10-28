# The checks of hibernate, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # hibernate-unlock's tests run as it builds: the TPM's counts are read
  # from tpm2_readclock and kept whole, a firmware start moves one of them
  # and a wake neither, only sleeps that write an image are kept, and a disk
  # counts as asking only when every token asks for something typed.
  hibernate-unlock-lets-go-only-after-the-disk-asked = pkgs.callPackage ./unlock/package.nix { };
  # clippy finds nothing in it, its tests included.
  hibernate-unlock-passes-clippy = passesClippy "hibernate-unlock-passes-clippy" (
    pkgs.callPackage ./unlock/package.nix { }
  );
}
