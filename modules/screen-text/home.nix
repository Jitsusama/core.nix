# Screenshot text: the words in each screenshot Joel takes are read once,
# on this machine, as it lands in ~/Pictures/Screenshots, so the launcher
# finds the screenshot by them and copies them out. What's read is kept in
# the service's state, never beside the pictures, and goes when its
# picture does.
#
# The program is dictation's third, screen-text, sharing its OpenVINO
# calls (modules/dictation/src/bin/screen-text/main.rs says how), with
# PP-OCRv5's models (models.nix) on the NPU at its background priority,
# and the service in background.slice, since nobody waits on a
# screenshot's words. The folder is watched through inotify, so nothing is
# looked at while nothing changes; the models are loaded for what waits to
# be read and let go a little after. What they compile is kept in a cache
# of its own, so only the first start compiles.
#
# The shell asks for the words over
# $XDG_RUNTIME_DIR/daemons/screen-text.sock in its daemon interface, and
# never reads a picture itself.
{ config, pkgs, ... }:
let
  dictation = pkgs.callPackage ../dictation/package.nix { };
  models = pkgs.callPackage ./models.nix { };
  # Where niri's screenshot-path and the shell's region pictures save.
  screenshots = "${config.home.homeDirectory}/Pictures/Screenshots";
in
{
  systemd.user.services.screen-text = {
    Unit = {
      Description = "Reads the words in each screenshot";
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = toString [
        "${dictation}/bin/screen-text serve"
        "%t/daemons/screen-text.sock ${models} ${screenshots} %S/screen-text/records.json"
      ];
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/screen-text.sock";
      Slice = "background.slice";
      CacheDirectory = "screen-text";
      StateDirectory = "screen-text";
      Restart = "on-failure";
      RestartSec = 5;
      Environment = [
        "SCREEN_TEXT_CACHE=%C/screen-text"
        "LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
      ];
    };
    Install.WantedBy = [ "default.target" ];
  };
}
