# bat, cat with code coloured, and its helpers for diffs, man pages and
# watching a command, in a theme drawn from the machine's.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) roles terminal;

  # bat reads TextMate's theme format, the theme's (../theme/tmtheme.nix),
  # its code on the terminal's own background.
  themeFile = import ../theme/tmtheme.nix {
    inherit
      lib
      pkgs
      roles
      terminal
      ;
  } roles.background;

  # bat finds a theme of its own only in its cache, which home-manager
  # rebuilds at every activation: 0.7 s of each switch on the laptop, and
  # 0.9 s of a test machine's boot, on the path to signing in. It's built
  # here once instead, and the account's cache folder is it.
  cache = pkgs.runCommandLocal "bat-cache" { } ''
    mkdir -p source/themes
    cp ${themeFile}/Jitsusama.tmTheme source/themes/
    ${lib.getExe config.programs.bat.package} cache --build --source source --target $out
  '';
  cacheFolder = "${config.xdg.cacheHome}/bat";
in
{
  imports = [ ../theme/home.nix ];

  programs.bat = {
    enable = true;
    config = {
      style = "changes";
      theme = "Jitsusama";
    };
    themes.Jitsusama = {
      src = themeFile;
      file = "Jitsusama.tmTheme";
    };
    extraPackages = [
      pkgs.bat-extras.batdiff
      pkgs.bat-extras.batman
      # batgrep has been broken since 2025-10-27.
      pkgs.bat-extras.batwatch
    ];
  };

  programs.zsh.shellAliases.cat = "bat";

  xdg.cacheFile.bat.source = cache;
  home.activation = {
    batCache = lib.mkForce (lib.hm.dag.entryAfter [ "linkGeneration" ] "");
    # A cache home-manager built before is a folder of its own, in the way of
    # the link to the one built here.
    batCacheBuiltOnce = lib.hm.dag.entryBefore [ "checkLinkTargets" ] ''
      if [ -d ${lib.escapeShellArg cacheFolder} ] && [ ! -L ${lib.escapeShellArg cacheFolder} ]; then
        run rm -rf ${lib.escapeShellArg cacheFolder}
      fi
    '';
  };

  # Man pages read through bat too, as plain text it colours: col strips the
  # overstrikes man uses for bold, and MANROFFOPT keeps them from being
  # written as escapes that col can't strip.
  home.sessionVariables = {
    MANPAGER = "sh -c 'col -bx | ${lib.getExe config.programs.bat.package} --language=man --plain'";
    MANROFFOPT = "-c";
  };
}
