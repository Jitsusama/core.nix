# Dictation: a chord (niri's binds.nix) runs `dictation hold`, which
# starts listening and, pressed again, types what was said into the focused
# window; held, it types when let go, which the system's dictation-chord
# (nixos.nix) hears. The service starts from its socket, and holds Whisper's
# models in app.slice beside the apps from the first press; it lets them go
# after ten minutes unused. Quickshell watches the same socket
# (Dictation.qml), and while it listens or types a surface of its own at the
# bottom centre says so (Speaking.qml), so what's heard is never heard unseen.
#
# Whisper runs whole on the NPU through GenAI's static pipeline
# (src/genai.rs), whose plugin finds Level Zero only on the library path,
# and whose tokenizer is a library GenAI looks for beside its own unless
# it's named; on a machine with no NPU the encoder runs on the CPU and the
# decoder after it, several times slower. What's compiled is kept in the
# cache, so the NPU's compile, about 85 s under load, is paid once, and
# later loads take about a second.
#
# Whisper writes a name it doesn't know as a word it does, Sarnia as
# "Sanita", so the words Joel uses that it gets wrong are declared, and it
# reads them before each transcription as what came before (src/words.rs).
{ quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  dictation = pkgs.callPackage ./package.nix { };
  model = pkgs.callPackage ./model.nix { };
  inherit (config.jitsusama.dictation) words;
  wordsFile = pkgs.writeText "dictation-words" (lib.concatLines words);
in
{
  imports = [ quickshell ];

  options.jitsusama.dictation.words = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [
      "NixOS"
      "Quickshell"
    ];
    description = ''
      Words and names Whisper should write as they're given here, one
      word or phrase each. Whisper reads them before each transcription,
      as text it already heard, and leans toward their spelling.
    '';
  };

  config = {
    home.packages = [ dictation ];

    xdg.configFile."quickshell/Dictation.qml".source = ./Dictation.qml;
    # Its own surface rather than a glyph in the margin, since it's for the
    # moment it's used and says more than a glyph can: the time, a wave.
    jitsusama.quickshell.surfaces.Speaking = ./Speaking.qml;
    xdg.configFile."quickshell/surfaces/Speaking.js".source = ./Speaking.js;

    systemd.user.sockets.dictation = {
      Unit.Description = "Dictation's socket, which a chord and the shell reach it by";
      Socket.ListenStream = "%t/dictation.sock";
      Install.WantedBy = [ "sockets.target" ];
    };

    systemd.user.services.dictation = {
      Unit = {
        Description = "Dictation, Whisper on the NPU";
        # Stopped before a switch reloads the units, the shell's watch
        # starts it again through the socket at once, from the old unit,
        # and the switch's start then finds it running; restarted after the
        # reload, it comes back as the new one.
        X-SwitchMethod = "restart";
      };
      Service = {
        ExecStart = "${lib.getExe dictation} serve ${model}";
        CacheDirectory = "dictation";
        Environment = [
          "DICTATION_CACHE=%C/dictation"
          "LD_LIBRARY_PATH=${lib.makeLibraryPath [ pkgs.level-zero ]}"
          "OPENVINO_TOKENIZERS_PATH_GENAI=${pkgs.openvino-tokenizers}/lib/libopenvino_tokenizers.so"
          "OV_NPU_LOG_LEVEL=LOG_ERROR"
          "PATH=${
            lib.makeBinPath [
              pkgs.libnotify
              pkgs.pipewire
              pkgs.wireplumber
              pkgs.wtype
            ]
          }"
        ]
        ++ lib.optional (words != [ ]) "DICTATION_WORDS=${wordsFile}";
      };
    };
  };
}
