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
# Once a call is written down whole, a notice says so, and its Open the
# Transcript opens it in the session's reader. Nothing sums it up: the
# daemon is given no summarizer, since Phi-3.5-mini, the one it had,
# wrote summaries of hour-long calls that named decisions never made,
# and a wrong summary is worse than none. A model that sums a long call
# up truly is given to it once one is found.
#
# The shell reads the calls from the service, over
# $XDG_RUNTIME_DIR/daemons/calls.sock in the shell's daemon interface,
# never from the folder.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  dictation = pkgs.callPackage ../dictation/package.nix { };
  model = pkgs.callPackage ../dictation/parakeet.nix { };
in
{
  # What Whisper compiled for calls before they moved to Parakeet, 0.9 GB
  # that nothing reads any more. Kept until no machine still has it.
  systemd.user.tmpfiles.rules = [ "R %C/call-transcripts/whisper - - - -" ];

  systemd.user.services.call-transcripts = {
    Unit = {
      Description = "Writes down what's said on each call";
      # Restarted after a switch reloads the units, it comes back as the
      # new one, and a call it was hearing goes on in the same transcript.
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = lib.concatStringsSep " " [
        "${dictation}/bin/call-transcripts serve"
        "%t/daemons/calls.sock ${model} %h/Documents/Calls"
      ];
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/calls.sock";
      Slice = "background.slice";
      CacheDirectory = "call-transcripts";
      Restart = "on-failure";
      RestartSec = 5;
      Environment = [
        "DICTATION_CACHE=%C/call-transcripts/parakeet"
        "LD_LIBRARY_PATH=${lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OPENVINO_TOKENIZERS_PATH_GENAI=${pkgs.openvino-tokenizers}/lib/libopenvino_tokenizers.so"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
        # The profile's last, for the program a transcript opens in:
        # xdg-open finds it by its desktop entry, which names it bare. A
        # file opens through systemd-run, in a unit of its own given this
        # PATH, so it has the session's screen (notices.rs says why).
        "PATH=${
          lib.makeBinPath [
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
