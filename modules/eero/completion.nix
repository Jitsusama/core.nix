# zsh's completion for eero, as `eero completion zsh` writes it: what
# home.nix installs, and what eero-completes-in-zsh types into, so the
# check holds the very file an account gets.
{
  callPackage,
  lib,
  runCommandLocal,
}:
let
  eero = callPackage ./package.nix { };
in
runCommandLocal "eero-zsh-completion" { } ''
  mkdir -p $out/share/zsh/site-functions
  ${lib.getExe eero} completion zsh > $out/share/zsh/site-functions/_eero
  grep -q '^#compdef eero$' $out/share/zsh/site-functions/_eero
''
