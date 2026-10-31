# Call transcripts: every call Joel has is written down as it goes, on this
# machine, into a folder of its own in ~/Documents/Calls, named by when it
# started and what it was, so the file picker and search find it. A call
# is what modules/calls counts as one, the rule pause-for-calls follows.
# Nothing of a call's sound is kept: it's held in memory until Whisper has
# heard it, and only the words are written.
#
# The program is dictation's second, call-transcripts, sharing its Whisper
# (modules/dictation/src/bin/call-transcripts/main.rs says how), but not
# the words dictation is given: told them, Whisper writes them back for a
# take with no speech, and a call came out as those words. Whisper runs
# on the NPU at its background priority, behind dictation and anything
# live, and the service in background.slice, since nobody waits on a
# transcript; it's loaded with a call's first words and let go after.
# Its compiled models are kept in a cache of its own, since a load removes
# whatever else is in the folder it's given.
#
# Once a call is written down whole and no call is being heard, Whisper is
# let go and Phi-3.5-mini (summarizer.nix) sums the call up beside its
# transcript, on the NPU at the same priority, its compiled model in a
# cache of its own beside Whisper's. A call with too little said is told
# so with no model asked, and a summary naming what the call never did
# is written once more, told not to, and isn't kept if it still does. A
# notice says it's ready, and its Read It opens the summary.
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
  model = pkgs.callPackage ../dictation/model.nix { };
  summarizer = pkgs.callPackage ./summarizer.nix { };
in
{
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
        "%t/daemons/calls.sock ${model} ${summarizer} %h/Documents/Calls"
      ];
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/calls.sock";
      Slice = "background.slice";
      CacheDirectory = "call-transcripts";
      Restart = "on-failure";
      RestartSec = 5;
      Environment = [
        "DICTATION_CACHE=%C/call-transcripts/whisper"
        "SUMMARIZER_CACHE=%C/call-transcripts/summarizer"
        "LD_LIBRARY_PATH=${lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OPENVINO_TOKENIZERS_PATH_GENAI=${pkgs.openvino-tokenizers}/lib/libopenvino_tokenizers.so"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
        # The profile's last, for the program a summary opens in: xdg-open
        # finds it by its desktop entry, which names it bare.
        "PATH=${
          lib.makeBinPath [
            pkgs.libnotify
            pkgs.pipewire
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
