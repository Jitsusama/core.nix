# Search by meaning: the launcher's things are found by what they mean as
# well as by their letters, so "dim the screen" finds Brightness. Each
# thing's meaning is made once, on this machine, from its name and the
# words it's known by, and kept; what's typed is made into one as it's
# asked, and the things nearest it are added below everything typing
# found, a few at most, each saying so in the launcher's foot
# (Meaning.qml). Meaning only ever adds: it never hides or outranks a
# match.
#
# The program is dictation's fifth, meaning, sharing its OpenVINO calls
# (modules/dictation/src/bin/meaning/main.rs says how), with
# bge-base-en-v1.5 (models.nix) on the NPU, falling back to the processor
# where there's none. It runs in background.slice and makes things'
# meanings only while nothing records, since a call's captions and
# dictation come first on the NPU; what's typed is asked at once. The
# model is let go a few minutes after the last ask, and what it compiles
# is kept in a cache of its own, so only the first start compiles. Each
# thing's meaning is kept in the service's state by its text, so a thing
# renamed is made again and the rest never are.
#
# The shell asks over $XDG_RUNTIME_DIR/daemons/meaning.sock in its daemon
# interface, only while the launcher shows, and never waits on it.
{ pkgs, ... }:
let
  dictation = pkgs.callPackage ../dictation/package.nix { };
  models = pkgs.callPackage ./models.nix { };
in
{
  systemd.user.services.meaning = {
    Unit = {
      Description = "Finds the launcher's things by their meaning";
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = toString [
        "${dictation}/bin/meaning serve"
        "%t/daemons/meaning.sock ${models} %S/meaning/meanings"
      ];
      ExecStopPost = "${pkgs.coreutils}/bin/rm -f %t/daemons/meaning.sock";
      Slice = "background.slice";
      CacheDirectory = "meaning";
      StateDirectory = "meaning";
      Restart = "on-failure";
      RestartSec = 5;
      Environment = [
        "MEANING_CACHE=%C/meaning"
        "LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ pkgs.level-zero ]}"
        "OV_NPU_LOG_LEVEL=LOG_ERROR"
        # pw-dump, to see whether anything records before making meanings.
        "PATH=${pkgs.lib.makeBinPath [ pkgs.pipewire ]}"
      ];
    };
    Install.WantedBy = [ "default.target" ];
  };

  # Read by Meaning.qml beside it, where Quickshell finds an import.
  xdg.configFile."quickshell/launcher/Meaning.js".source = ./Meaning.js;

  # Last of the sources, since it lists nothing of its own: it only adds
  # below what the others found.
  jitsusama.quickshell.launcher.providers.meaning = {
    order = 140;
    source = ./Meaning.qml;
  };
}
