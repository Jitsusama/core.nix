# The checks of mpv, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # mpv plays a moment of video and sound with the account's settings and
  # those of its controller, console, right-click menu and statistics,
  # saying nothing about any: mpv names an option it doesn't know and goes
  # on without it, and each script does the same for a key, so a setting a
  # new release drops would vanish without a word. A video file opens in
  # mpv.
  mpv-accepts-its-configuration =
    let
      home = account graphical;
      mpv = lib.findFirst (package: lib.hasPrefix "mpv" (lib.getName package)) null home.home.packages;
      files = home.xdg.configFile;
      opens = graphical.config.xdg.mime.defaultApplications."video/mp4" or "";
      scripts = [
        "osc"
        "console"
        "context_menu"
        "stats"
      ];
    in
    assert opens == "mpv.desktop";
    pkgs.runCommandLocal "mpv-accepts-its-configuration" { nativeBuildInputs = [ mpv ]; } ''
      export HOME=$PWD FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      mkdir -p mpv/script-opts
      cp ${files."mpv/mpv.conf".source} mpv/mpv.conf
      ${lib.concatMapStrings (script: ''
        cp ${files."mpv/script-opts/${script}.conf".source} mpv/script-opts/${script}.conf
      '') scripts}
      # Kept open, mpv would hold the last frame and never finish.
      mpv --config-dir=$PWD/mpv --vo=null --ao=null --force-window=no --keep-open=no \
        --save-position-on-quit=no --frames=10 \
        --script-opts-append=osc-visibility=always \
        'av://lavfi:testsrc=duration=1:size=64x64' > said 2>&1 || { cat said; exit 1; }
      if grep -i 'error\|unknown\|not found\|invalid' said; then exit 1; fi
      touch $out
    '';
}
