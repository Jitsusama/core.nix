# A call heard and written down. A VM has no microphone, so a loopback
# stands in for one, what's played into it being what it hears; a
# PulseAudio recorder copied under a call program's name stands in for the
# call, and the same program's player for the others' side. Joel asks, the
# others answer, another program says something of its own meanwhile, and
# Joel answers back; the transcript must hold the three turns in order and
# nothing of the other program's. The daemon runs as the service does,
# with a stand-in for the summarizer's model, which a VM can't run
# (stand-in-summarizer.py): it's started once the call is written, holds
# a delay on sleep, is ended when another call starts, writing nothing,
# and is started again once that call ends, and its summary holds the
# call's own words; a notice says it's ready, which a stand-in for the
# shell's notices keeps, and a notice as the call started offered the
# line for its chat. The session says where its screen is only after the
# daemon starts, as niri does, and the summary's notice's Read It must
# still open it in the session's reader, which a stand-in for it notes.
# The next meeting is stopped
# from that notice, chosen on the stand-in as a click would, and joined
# again, which mustn't be recorded. The voices are
# espeak-ng's, made as the test builds. The tests are
# call-transcripts-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  dictation = pkgs.callPackage ../../../modules/dictation/package.nix { };
  model = pkgs.callPackage ../../../modules/dictation/parakeet.nix { };
  # The session's reader for Markdown, noting where it was shown and what.
  reader = pkgs.writeShellApplication {
    name = "stand-in-reader";
    text = ''echo "$WAYLAND_DISPLAY $1" > "$HOME/read"'';
  };
  readerEntry = pkgs.makeDesktopItem {
    name = "stand-in-reader";
    desktopName = "Stand-in Reader";
    exec = "stand-in-reader %f";
    mimeTypes = [ "text/markdown" ];
  };
  notices = pkgs.writeShellApplication {
    name = "stand-in-notices";
    text = ''
      exec ${
        pkgs.python3.withPackages (python: [ python.dbus-fast ])
      }/bin/python3 ${../../stand-in-notices.py} "$@"
    '';
  };
  summarizer = pkgs.writeShellApplication {
    name = "stand-in-summarizer";
    text = ''exec ${pkgs.python3}/bin/python3 ${./stand-in-summarizer.py} "$HOME"'';
  };
  # Joel's side, the others', and a program that isn't the call, each a
  # 16 kHz mono WAV from the call's start.
  clips =
    pkgs.runCommand "call-clips"
      {
        nativeBuildInputs = [
          pkgs.espeak-ng
          pkgs.sox
        ];
      }
      ''
        mkdir $out
        say() {
          espeak-ng -v "$1" -s 160 -w said.wav "$2"
          sox said.wav -r 16000 -c 1 -b 16 "$3"
        }
        say en-us "Can we move the review to Thursday?" ask.wav
        say en-gb "Thursday morning works for everyone." answer.wav
        say en-us "Perfect, thank you so much." thanks.wav
        say en-us+f3 "The weather today is sunny and warm." weather.wav
        sox ask.wav asked.wav pad 0.5 6
        sox asked.wav thanks.wav $out/mine.wav
        sox answer.wav $out/theirs.wav pad 4 4
        sox weather.wav $out/other.wav pad 4 4
      '';
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "call-transcripts";

  nodes.machine = {
    imports = [ "${self}/modules/pipewire/nixos.nix" ];
    # Parakeet on the CPU, as call-transcripts' checks run it, since a VM
    # has no NPU, needs the room.
    virtualisation.memorySize = 6144;
    virtualisation.cores = 4;
    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      dictation
      notices
      # gdbus, which the daemon hears logind's PrepareForSleep with.
      pkgs.glib.bin
      pkgs.jq
      pkgs.libnotify
      pkgs.pulseaudio
      pkgs.socat
      pkgs.xdg-utils
      reader
      readerEntry
    ];
    environment.etc."call-clips".source = clips;
    environment.etc."xdg/mimeapps.list".text = ''
      [Default Applications]
      text/markdown=stand-in-reader.desktop
    '';
  };

  values =
    _:
    pkgs.writeText "call-transcripts-values.json" (
      builtins.toJSON {
        inherit model;
        summarizer = "${summarizer}/bin/stand-in-summarizer";
        # The library the service names, given as it gives it.
        tokenizers = "${dictation.openvino-tokenizers}/lib/libopenvino_tokenizers.so";
      }
    );

  tests = ./call-transcripts-vm-test.py;
}
