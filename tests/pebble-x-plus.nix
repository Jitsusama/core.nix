# Plays tones in a virtual machine and measures what reaches a Pebble X Plus
# and another output, with the speakers' tuning running and without. A VM
# has no USB speakers, so a silent sink stands in for them with the USB id
# the tuning finds them by, and a second for headphones. 10.2 kHz sits in the
# tuning's deepest cut, 6 dB, and 1 kHz below all three, so through the
# tuning the first falls about 6 dB and the second about half of one. The
# headphones should hear both as they were played.
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
pkgs.testers.runNixOSTest {
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

  testScript = ''
    import math
    import shlex

    def as_joel(command):
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    # Plays a tone to the default sink while recording what reaches the given
    # one, and gives the level of the tone's middle second, away from its
    # edges. The recorder is linked to the sink's monitor by hand: left to
    # WirePlumber, a recording of a sink with a smart filter is given the
    # filter's input instead, which is the sound before the filter.
    def level(frequency, sink):
        as_joel(
            "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav & recording=$!;"
            " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
            f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
            f" pw-play tone-{frequency}.wav;"
            # pw-record exits with 1 when interrupted, so its file is the
            # evidence rather than its status.
            " kill -INT $recording; wait $recording; test -s heard.wav"
        )
        rms = as_joel(
            "sox heard.wav -n silence 1 0.01 0.1% trim 1 1 stat 2>&1"
            " | awk '/RMS +amplitude/ {print $3}'"
        )
        return float(rms)

    def decibels(heard, played):
        return 20 * math.log10(heard / played)

    def tuning_ready():
        machine.wait_for_unit("pebble-x-plus-tuning.service", "joel")
        machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q pebble_x_plus_tuning"
        )

    tones = (1000, 10200)
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_joel("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    for frequency in tones:
        as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")

    with subtest("the tuning starts with PipeWire"):
        tuning_ready()

    with subtest("without the tuning, the speakers hear what was played"):
        as_joel("systemctl --user stop pebble-x-plus-tuning")
        as_joel("pactl set-default-sink pebble")
        played = {frequency: level(frequency, "pebble") for frequency in tones}
        machine.log(f"played: {played}")
        assert all(value > 0.01 for value in played.values()), played

    with subtest("the speakers hear the tuning"):
        as_joel("systemctl --user start pebble-x-plus-tuning")
        tuning_ready()
        as_joel("pactl set-default-sink pebble")
        kept = decibels(level(1000, "pebble"), played[1000])
        cut = decibels(level(10200, "pebble"), played[10200])
        machine.log(f"1 kHz changed by {kept:.1f} dB, 10.2 kHz by {cut:.1f} dB")
        assert abs(kept) < 1, kept
        assert cut < -4, cut

    with subtest("the headphones hear what was played"):
        as_joel("pactl set-default-sink headphones")
        for frequency in tones:
            change = decibels(level(frequency, "headphones"), played[frequency])
            machine.log(f"{frequency} Hz changed by {change:.1f} dB on the headphones")
            assert abs(change) < 1, (frequency, change)
  '';
}
