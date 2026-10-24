{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.bat;
in
{
  programs.bat = {
    enable = true;
    config.style = "changes";
    extraPackages = [
      pkgs.bat-extras.batdiff
      pkgs.bat-extras.batman
      # batgrep has been broken since 2025-10-27.
      pkgs.bat-extras.batwatch
    ];
  };

  # home-manager rebuilds bat's cache of themes and syntaxes at every
  # activation, which took 0.7 s of each switch and 0.9 s of each test
  # machine's boot, though with none of its own bat builds the same set it
  # already has inside. So the cache goes instead, rather than one built by
  # an older bat being left for a newer one to read.
  home.activation.batCache = lib.mkIf (cfg.themes == { } && cfg.syntaxes == { }) (
    lib.mkForce (
      lib.hm.dag.entryAfter [ "linkGeneration" ] ''
        run rm -rf ${lib.escapeShellArg "${config.xdg.cacheHome}/bat"}
      ''
    )
  );

  programs.zsh.shellAliases.cat = "bat";
}
