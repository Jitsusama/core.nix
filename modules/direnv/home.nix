# direnv, which loads a folder's .envrc, with nix-direnv caching the
# development shells they use. Its shell hook is written when the account is
# built rather than asked of direnv at each shell start.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  hook = pkgs.runCommandLocal "direnv.zsh" { } ''
    ${lib.getExe config.programs.direnv.package} hook zsh > $out
  '';
in
{
  programs.direnv = {
    enable = true;
    enableZshIntegration = false;
    nix-direnv.enable = true;
  };

  programs.zsh.initContent = "source ${hook}";
}
