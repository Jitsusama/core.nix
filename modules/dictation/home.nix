# Dictation: a chord (niri's binds.nix) runs `dictation toggle`, which
# starts listening and, pressed again, types what was said into the focused
# window. The service starts from its socket, and holds Whisper's models in
# app.slice beside the apps from the first press; it lets them go after ten
# minutes unused. Quickshell watches the same socket (Dictation.qml), and
# while it listens or types, the overview says so and the margin wears its
# mark (Listening.qml), so what's heard is never heard unseen.
#
# The encoder runs on the NPU, whose plugin finds Level Zero only on the
# library path; on a machine with no NPU it runs on the CPU, several times
# slower. OpenVINO keeps what it compiles in the cache, which cuts the NPU's
# compile from half a minute to a read.
{ quickshell }:
{ lib, pkgs, ... }:
let
  dictation = pkgs.callPackage ./package.nix { };
  model = pkgs.callPackage ./model.nix { };
in
{
  imports = [ quickshell ];

  home.packages = [ dictation ];

  xdg.configFile."quickshell/Dictation.qml".source = ./Dictation.qml;
  # Among what's on only now and then, beside the microphone's mute, since
  # both say what the microphone is doing.
  jitsusama.quickshell.overview.pieces.dictation = {
    order = 69;
    source = ./Listening.qml;
    label = "dictation";
    tier = "when";
    mark = "listening";
  };

  systemd.user.sockets.dictation = {
    Unit.Description = "Dictation's socket, which a chord and the shell reach it by";
    Socket.ListenStream = "%t/dictation.sock";
    Install.WantedBy = [ "sockets.target" ];
  };

  systemd.user.services.dictation = {
    Unit = {
      Description = "Dictation, Whisper's encoder on the NPU";
      # Stopped before a switch reloads the units, the shell's watch starts
      # it again through the socket at once, from the old unit, and the
      # switch's start then finds it running; restarted after the reload,
      # it comes back as the new one.
      X-SwitchMethod = "restart";
    };
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
            pkgs.wireplumber
            pkgs.wtype
          ]
        }"
      ];
    };
  };
}
