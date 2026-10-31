# Asks a-call-is-on, as root, whether a call is on while an account's
# PipeWire records and while it doesn't, as an update with nobody at it
# asks. A VM has no microphone, so a silent source stands in for one, and a
# PulseAudio recorder copied under a call program's name stands in for the
# call. The tests are calls-vm-test.py, which pytest runs inside the driver.
{ pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "calls";

  nodes.machine = {
    imports = [ ../modules/pipewire/nixos.nix ];

    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      (pkgs.callPackage ../modules/calls/package.nix { })
      pkgs.jq
      pkgs.pulseaudio
    ];
    services.pipewire.extraConfig.pipewire."10-silent-microphone"."context.objects" = [
      {
        factory = "adapter";
        args = {
          "factory.name" = "support.null-audio-sink";
          "node.name" = "silent-microphone";
          # PipeWire's own recipe for a source with no device under it.
          "media.class" = "Audio/Source/Virtual";
          "audio.position" = [ "MONO" ];
          "object.linger" = true;
        };
      }
    ];
  };

  tests = ./calls-vm-test.py;
}
