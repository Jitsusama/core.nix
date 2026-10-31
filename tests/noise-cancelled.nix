# A microphone offered noise-cancelled. A VM has no microphone, so a
# loopback stands in for one, carrying a device's id as a real one's node
# does, what's played into it being what it hears. The noise-cancelled program must
# offer it a second time, named for it; hear it only while something
# records from that one; and give back its noise much quieter than the
# microphone itself does, while its voice still comes through. The noise
# alone is sox's pink noise; the voice is a person's, over a room's noise
# 10 dB under it, both from the recordings DeepFilterNet's repository
# keeps, since a synthetic voice is taken down as if it were noise. The tests
# are noise-cancelled-vm-test.py, which pytest runs inside the driver.
{ self, pkgs }:
let
  dictation = pkgs.callPackage ../modules/dictation/package.nix { };
  model = pkgs.callPackage ../modules/noise-cancelled/model.nix { };
  # Noise alone, then a person speaking over a room's noise, each a 48 kHz
  # mono WAV. The voice has a second of the room before it, while the
  # denoiser learns it.
  clips = pkgs.runCommand "noise-cancelled-clips" { nativeBuildInputs = [ pkgs.sox ]; } ''
    mkdir $out
    sox -R -n -r 48000 -c 1 -b 16 $out/noise.wav synth 4 pinknoise vol 0.03
    to=(-r 48000 -c 1 -b 16)
    sox ${model.source}/assets/clean_freesound_33711.wav "''${to[@]}" voice.wav \
      remix 1 pad 1 0.5 norm -20
    sox ${model.source}/assets/noise_freesound_573577.wav "''${to[@]}" room.wav \
      remix 1 repeat 9 trim 0 "$(soxi -D voice.wav)" norm -30
    # Each at its own level, since sox would otherwise halve both to mix them.
    sox -m -v 1 voice.wav -v 1 room.wav $out/spoken.wav
  '';
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "noise-cancelled";

  nodes.machine = {
    imports = [ "${self}/modules/pipewire/nixos.nix" ];
    virtualisation.memorySize = 2048;
    virtualisation.cores = 2;
    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      dictation
      pkgs.jq
      pkgs.sox
    ];
    environment.etc."noise-cancelled-clips".source = clips;
  };

  values = _: pkgs.writeText "noise-cancelled-values.json" (builtins.toJSON { inherit model; });

  tests = ./noise-cancelled-vm-test.py;
}
