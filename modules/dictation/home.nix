# Dictation: a chord (niri's binds.nix) runs `dictation toggle`, which
# starts listening and, pressed again, types what was said into the focused
# window. The service starts at the first press, from its socket, and holds
# Whisper's models in app.slice beside the apps; it lets them go after ten
# minutes unused. Quickshell watches the same socket to show when it's
# listening.
#
# The encoder runs on the NPU, whose plugin finds Level Zero only on the
# library path; on a machine with no NPU it runs on the CPU, several times
# slower. OpenVINO keeps what it compiles in the cache, which cuts the NPU's
# compile from half a minute to a read.
{ lib, pkgs, ... }:
let
  dictation = pkgs.callPackage ./package.nix { };
  model = pkgs.callPackage ./model.nix { };
in
{
  home.packages = [ dictation ];

  systemd.user.sockets.dictation = {
    Unit.Description = "Dictation's socket, which a chord and the shell reach it by";
    Socket.ListenStream = "%t/dictation.sock";
    Install.WantedBy = [ "sockets.target" ];
  };

  systemd.user.services.dictation = {
    Unit.Description = "Dictation, Whisper's encoder on the NPU";
    Service = {
      ExecStart = "${lib.getExe dictation} serve ${model}";
      CacheDirectory = "dictation";
      Environment = [
        "DICTATION_CACHE=%C/dictation"
        "LD_LIBRARY_PATH=${lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
        "PATH=${
          lib.makeBinPath [
            pkgs.libnotify
            pkgs.pipewire
            pkgs.wtype
          ]
        }"
      ];
    };
  };
}
