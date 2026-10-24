# The checks of linux-id, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # linux-id's pinentry speaks pinentry's protocol and asks the askpass in
  # the words the program set, as its own tests say.
  the-pinentry-asks-the-shell =
    pkgs.runCommandCC "the-pinentry-asks-the-shell" { nativeBuildInputs = [ pkgs.rustc ]; }
      ''
        rustc --edition 2024 --test -D warnings -o tests ${./pinentry.rs}
        ./tests
        touch $out
      '';
}
