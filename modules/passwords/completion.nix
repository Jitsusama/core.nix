# zsh's completion for passwords, as `passwords completion zsh` writes it:
# what home.nix installs, and what passwords-completes-in-zsh types into,
# so the check holds the very file an account gets.
{
  callPackage,
  lib,
  runCommandLocal,
}:
let
  passwords = callPackage ./package.nix { };
in
runCommandLocal "passwords-zsh-completion" { } ''
  mkdir -p $out/share/zsh/site-functions
  ${lib.getExe passwords} completion zsh > $out/share/zsh/site-functions/_passwords
  grep -q '^#compdef passwords$' $out/share/zsh/site-functions/_passwords
''
