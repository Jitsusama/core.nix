# A call heard and written down. A VM has no microphone, so a loopback
# stands in for one, what's played into it being what it hears; a
# PulseAudio recorder copied under a call program's name stands in for the
# call, and the same program's player for the others' side. Joel asks, the
# others answer, another program says something of its own meanwhile, and
# Joel answers back; the transcript must hold the three turns in order and
# nothing of the other program's. The voices are espeak-ng's, made as the
# test builds. The tests are call-transcripts-vm-test.py, which pytest runs
# inside the driver.
{ self, pkgs }:
let
  dictation = pkgs.callPackage ../modules/dictation/package.nix { };
  model = pkgs.callPackage ../modules/dictation/model.nix { };
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
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "call-transcripts";

  nodes.machine = {
    imports = [ "${self}/modules/pipewire/nixos.nix" ];
    # Whisper on the CPU, as dictation's checks run it, since a VM has no
    # NPU, needs the room.
    virtualisation.memorySize = 4096;
    virtualisation.cores = 4;
    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      dictation
      pkgs.jq
      pkgs.pulseaudio
      pkgs.socat
    ];
    environment.etc."call-clips".source = clips;
  };

  values = _: pkgs.writeText "call-transcripts-values.json" (builtins.toJSON { inherit model; });

  tests = ./call-transcripts-vm-test.py;
}
