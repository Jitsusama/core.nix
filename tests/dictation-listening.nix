# Presses dictation in an account with PipeWire running and watches what
# the service tells the shell. With no microphone, a press starts listening
# but no sound arrives, so the watchers never hear "listening", which is
# what the shell shows. With a stand-in microphone, they hear it once sound
# arrives, and a second press stops it. No model is loaded: the service
# loads Whisper only to transcribe, and what's heard here is silence, which
# it says rather than transcribes. The tests are dictation-listening-vm-test.py.
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "dictation-listening";

  nodes.machine = {
    imports = [ "${self}/modules/pipewire/nixos.nix" ];
    users.users.alice.isNormalUser = true;
    environment.systemPackages = [
      (pkgs.callPackage "${self}/modules/dictation/package.nix" { })
      pkgs.socat
    ];
  };

  tests = ./dictation-listening-vm-test.py;
}
