# Call transcripts: every call Joel has is written down as it goes, on this
# machine, into a folder of its own in ~/Documents/Calls, named by when it
# started and what it was, so the file picker and search find it. A call
# is what modules/calls counts as one, the rule pause-for-calls follows.
# Nothing of a call's sound is kept: it's held in memory until it's been
# heard, and only the words are written.
#
# The program is dictation's second, call-transcripts, sharing its code
# but not its Whisper: it hears through NVIDIA's Parakeet
# (modules/dictation/parakeet.nix), which misheard a fifth fewer words of
# meetings. Whisper's one gain, the words dictation is given, a call never
# had: told them, Whisper wrote them back for a take with no speech, and a
# call came out as those words. Parakeet's encoder runs on the NPU at its
# background priority, behind dictation and anything live, and the service
# in background.slice, since nobody waits on a transcript; it's loaded
# with a call's first words and let go after. Its compiled models are kept
# in a cache of its own, since a load removes whatever else is in the
# folder it's given.
#
# Once a call is written down whole, it's summed up in its own words:
# Gemma 4 26B-A4B (summarizer.nix) on the GPU picks the lines worth
# reading and the words in each worth keeping, and the daemon copies
# those words from the line, keeping none it doesn't find there, so the
# summary holds only quotes, each with who said it to whom and when.
# Every model before wrote sentences of its own, and each bent or made
# up what was said, and a wrong summary is worse than none. The model
# runs in a process of its own, `call-transcripts ask`, for one summary
# at a time: only on the charger, never during a call, with a delay on
# sleep held, and only once the memory it needs is free
# (src/bin/call-transcripts/summarizer.rs says how). Its compiled model
# is kept in a cache of its own. Only a machine whose hardware names a
# device for it sums calls up (nixos.nix); elsewhere they're only written
# down, and the model isn't fetched.
#
# The shell reads the calls from the service, over
# $XDG_RUNTIME_DIR/daemons/calls.sock in the shell's daemon interface,
# never from the folder (Transcripts.qml). Each call is a thing in the
# launcher, in its Transcripts (Transcribed.qml): Return opens its
# transcript in the reader, and inside it can be opened, its folder
# opened or its path copied, with its summary, or its first lines, in
# the drawer.
# It's in the Recent of whoever a Tuple call was with
# (TranscriptSource.qml), and a meeting whose call was written down
# offers Open the Transcript.
{ people, quickshell }:
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for where it sums calls up.
  osConfig ? { },
  ...
}:
let
  dictation = pkgs.callPackage ../dictation/package.nix { };
  model = pkgs.callPackage ../dictation/parakeet.nix { };
  device = config.jitsusama.call-transcripts.summarizerDevice;
  # The program the daemon asks a summary's questions of.
  summarizer = pkgs.writeShellScript "call-summarizer" ''
    exec ${dictation}/bin/call-transcripts ask ${pkgs.callPackage ./summarizer.nix { }}
  '';
in
{
  imports = [
    people
    quickshell
  ];

  options.jitsusama.call-transcripts.summarizerDevice = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = osConfig.jitsusama.call-transcripts.summarizerDevice or null;
    defaultText = lib.literalExpression "osConfig.jitsusama.call-transcripts.summarizerDevice or null";
    description = ''
      The OpenVINO device calls are summed up on, or null for none, which
      leaves each call written down with no summary.
    '';
  };

  config.xdg.configFile = {
    "quickshell/Transcripts.qml".source = ./Transcripts.qml;
    "quickshell/Transcripts.js".source = ./Transcripts.js;
  };

  # Just after Tuple's calls, since a transcript is what a call leaves.
  config.jitsusama.quickshell.launcher.providers.transcripts = {
    order = 46;
    source = ./Transcribed.qml;
  };

  # After the programs that know who someone is, since it knows nobody of
  # its own and only lists what was said with them.
  config.jitsusama.people.sources.transcripts = {
    order = 40;
    source = ./TranscriptSource.qml;
  };

  # What Whisper compiled for calls before they moved to Parakeet, 0.9 GB
  # that nothing reads any more. Kept until no machine still has it.
  # And what Phi-3.5-mini compiled for the NPU, the summarizer before
  # Gemma, that nothing reads either.
  config.systemd.user.tmpfiles.rules = [
    "R %C/call-transcripts/whisper - - - -"
    "R %C/call-transcripts/summarizer - - - -"
  ];

  config.systemd.user.services.call-transcripts = {
    Unit = {
      Description = "Writes down what's said on each call";
      # Restarted after a switch reloads the units, it comes back as the
      # new one, and a call it was hearing goes on in the same transcript.
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = lib.concatStringsSep " " (
        [
          "${dictation}/bin/call-transcripts serve"
          "%t/daemons/calls.sock ${model}"
        ]
        ++ lib.optional (device != null) "${summarizer}"
        ++ [ "%h/Documents/Calls" ]
      );
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/calls.sock";
      Slice = "background.slice";
      CacheDirectory = "call-transcripts";
      Restart = "on-failure";
      RestartSec = 5;
      Environment = [
        "DICTATION_CACHE=%C/call-transcripts/parakeet"
        "SUMMARIZER_CACHE=%C/call-transcripts/gemma"
        "SUMMARIZER_DEVICE=${toString device}"
        "LD_LIBRARY_PATH=${lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OPENVINO_TOKENIZERS_PATH_GENAI=${dictation.openvino-tokenizers}/lib/libopenvino_tokenizers.so"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
        # The profile's last, for the program a transcript opens in:
        # xdg-open finds it by its desktop entry, which names it bare. A
        # file opens through systemd-run, in a unit of its own given this
        # PATH, so it has the session's screen (notices.rs says why).
        "PATH=${
          lib.makeBinPath [
            # cat, which systemd-inhibit runs to keep the laptop awake
            # through a call (follow.rs says why), and to hold a delay on
            # sleep while a summary is made (summarizer.rs).
            pkgs.coreutils
            # gdbus, which hears logind say the laptop is about to sleep.
            pkgs.glib.bin
            pkgs.libnotify
            pkgs.pipewire
            pkgs.systemd
            # wl-copy, which puts the line for a call's chat on the clipboard.
            pkgs.wl-clipboard
            pkgs.xdg-utils
          ]
        }:${config.home.profileDirectory}/bin"
      ];
    };
    Install.WantedBy = [ "default.target" ];
  };
}
