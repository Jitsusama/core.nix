# Plays tones in a virtual machine and measures what reaches a Pebble X Plus
# and another output, with the speakers' tuning running and without. A VM
# has no USB speakers, so a silent sink stands in for them with the USB id
# the tuning finds them by, and a second for headphones. 10.2 kHz sits in the
# tuning's deepest cut, 6 dB, and 1 kHz below all three, so through the
# tuning the first falls about 6 dB and the second about half of one. The
# headphones should hear both as they were played. The stand-in speakers are
# then pulled out and made again, as at a replug, and must hear the tuning
# again with nobody choosing them. The script is pebble-x-plus-vm-test.py.
{ pkgs }:
let
  silentSink = name: properties: {
    factory = "adapter";
    args = {
      "factory.name" = "support.null-audio-sink";
      "node.name" = name;
      "media.class" = "Audio/Sink";
      "audio.position" = [
        "FL"
        "FR"
      ];
      "monitor.channel-volumes" = true;
      "object.linger" = true;
    }
    // properties;
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "pebble-x-plus";

  nodes.machine = {
    imports = [ ../modules/creative-pebble-x-plus/nixos.nix ];

    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      pkgs.pulseaudio
      pkgs.sox
    ];
    services.pipewire.extraConfig.pipewire."10-desk-sinks"."context.objects" = [
      (silentSink "pebble" { "alsa.components" = "USB041e:329a"; })
      (silentSink "headphones" { })
    ];
  };

  tests = ./pebble-x-plus-vm-test.py;
}
