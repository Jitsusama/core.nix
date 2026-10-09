# Plays tones in a virtual machine and measures what reaches the XPS 14's
# speakers and its headphone jack, with the speaker tuning running and
# without. A VM has neither, so two silent sinks stand in for them under the
# laptop's own names. 631 Hz sits in the tuning's deepest cut and 1356 Hz on
# one of its peaks, so through the tuning the first falls about 10 dB further
# than the second (18.6 and 8.6 dB, the limiter's input gain taking 5.3 dB off
# both). The headphones should hear both as they were played.
{ pkgs }:
let
  speakers = "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Speaker__sink";
  headphones = "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Headphones__sink";
  silentSink = name: {
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
    };
  };
in
pkgs.testers.runNixOSTest {
  name = "speakers";

  nodes.machine = {
    imports = [ ../hardware/dell-xps-14-da14260/speakers.nix ];

    users.users.joel.isNormalUser = true;
    environment.systemPackages = [
      pkgs.pulseaudio
      pkgs.sox
    ];
    services.pipewire.extraConfig.pipewire."10-laptop-sinks"."context.objects" = map silentSink [
      speakers
      headphones
    ];
  };

  testScript = ''
    import math
    import shlex

    speakers = "${speakers}"
    headphones = "${headphones}"

    def as_joel(command):
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    # Plays a tone to the default sink while recording what reaches the given
    # one, and gives the level of the middle second, away from the edges. The
    # recorder is linked to the sink's monitor by hand: left to WirePlumber, a
    # recording of a sink with a smart filter is given the filter's input
    # instead, which is the sound before the filter has touched it.
    def level(frequency, sink):
        as_joel(
            "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav & recording=$!;"
            " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
            f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
            f" sleep 0.5; pw-play tone-{frequency}.wav; sleep 0.5;"
            # pw-record exits with 1 when interrupted, so its file is the
            # evidence rather than its status.
            " kill -INT $recording; wait $recording; test -s heard.wav"
        )
        rms = as_joel("sox heard.wav -n trim 1.5 1 stat 2>&1 | awk '/RMS +amplitude/ {print $3}'")
        return float(rms)

    def decibels(heard, played):
        return 20 * math.log10(heard / played)

    def tuning_ready():
        machine.wait_for_unit("speaker-tuning.service", "joel")
        machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q speaker_tuning"
        )

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_joel("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    for frequency in (631, 1356):
        as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")

    with subtest("the tuning starts with PipeWire"):
        tuning_ready()

    with subtest("without the tuning, the speakers hear what was played"):
        as_joel("systemctl --user stop speaker-tuning")
        as_joel(f"pactl set-default-sink {speakers}")
        played = {frequency: level(frequency, speakers) for frequency in (631, 1356)}
        machine.log(f"played: {played}")
        assert all(value > 0.01 for value in played.values()), played

    with subtest("the speakers hear the tuning"):
        as_joel("systemctl --user start speaker-tuning")
        tuning_ready()
        as_joel(f"pactl set-default-sink {speakers}")
        cut = decibels(level(631, speakers), played[631])
        lift = decibels(level(1356, speakers), played[1356])
        machine.log(f"631 Hz changed by {cut:.1f} dB, 1356 Hz by {lift:.1f} dB")
        assert lift - cut > 6, (cut, lift)

    with subtest("the headphones hear what was played"):
        as_joel(f"pactl set-default-sink {headphones}")
        for frequency in (631, 1356):
            change = decibels(level(frequency, headphones), played[frequency])
            machine.log(f"{frequency} Hz changed by {change:.1f} dB on the headphones")
            assert abs(change) < 1, (frequency, change)
  '';
}
