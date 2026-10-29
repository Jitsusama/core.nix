# zoxide, which jumps to the folders visited most. Its shell setup is written
# when the account is built rather than asked of zoxide at each shell start,
# in the place home-manager would put it, after compinit. Its queries leave
# the database as they found it (queries-leave-the-database-alone.patch), so
# what watches the file hears only a folder visited.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  setup = pkgs.runCommandLocal "zoxide.zsh" { } ''
    ${lib.getExe config.programs.zoxide.package} init zsh > $out
  '';
in
{
  programs.zoxide = {
    enable = true;
    package = pkgs.zoxide.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./queries-leave-the-database-alone.patch ];
    });
    enableZshIntegration = false;
  };

  programs.zsh.initContent = lib.mkOrder 851 "source ${setup}";
}
