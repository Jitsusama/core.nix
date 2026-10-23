# Records tones from a stand-in Shure MV7 and another microphone in a virtual
# machine, with the MV7's voice filter running and without. A VM has no USB
# microphones, so a player that presents itself as a source stands in for
# each: one with the USB id the filter finds the MV7 by, and a headset
# without it. A quiet tone at -41 dBFS RMS, the level speech measured from
# the real mic, should come through the filter 12 dB louder; a loud one,
# peaking at -6 dBFS, should be held under the filter's -1 dBFS. A tone at
# the room's level, -70, should come up much less than 12 dB, since the
# expander takes it down; and a 7 kHz tone at the level the speaker's s
# sounds measured, -46, much less too, since the de-esser does, while 1 kHz
# at that level comes up the full 12. The headset should be heard as played.
{ pkgs }:
pkgs.testers.runNixOSTest {
  name = "shure-mv7-voice";

  nodes.machine = {
    imports = [ ../modules/shure-mv7/nixos.nix ];

    users.users.joel.isNormalUser = true;
    environment.systemPackages = [ pkgs.sox ];
  };

  testScript = ''
    import shlex

    def as_joel(command):
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    mv7 = '{ node.name = mv7 media.class = Audio/Source alsa.components = "USB14ed:1012" }'
    headset = "{ node.name = headset media.class = Audio/Source }"

    # Plays a tone as the given microphone and records it as a program
    # would, asking for that microphone by name, then gives the peak and RMS
    # of the recording's middle second in dBFS. With the filter running,
    # the recording waits until WirePlumber has linked the stand-in to it.
    def level(tone, source, filtered):
        name = "mv7" if "USB14ed" in source else "headset"
        ready = (
            "pw-link -l | grep -q '<- " + name + ":'"
            if filtered
            else "pw-cli ls Node | grep -q 'node.name = \"" + name + "\"'"
        )
        # Gives up after 20 s, so a stand-in that never appears fails the
        # test rather than hanging it.
        linked = " n=0; until " + ready + "; do n=$((n + 1)); [ $n -lt 200 ] || exit 1; sleep 0.1; done;"
        as_joel(
            f"pw-play -P '{source}' {tone}.wav & player=$!;"
            + linked
            + f" pw-record --format f32 --channels 1 --target {name} heard.wav & recording=$!;"
            " sleep 3; kill -INT $recording; wait $recording; kill $player; wait $player;"
            # pw-record and an interrupted pw-play exit non-zero, so the file
            # is the evidence rather than their status.
            " test -s heard.wav"
        )
        stats = as_joel(
            "sox heard.wav -n silence 1 0.01 0.01% trim 0.5 1 stats 2>&1"
            " | awk '/Pk lev dB/ {print $4} /RMS lev dB/ {print $4}'"
        ).split()
        return float(stats[0]), float(stats[1])

    def filter_ready():
        machine.wait_for_unit("shure-mv7-voice.service", "joel")
        machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q shure_mv7_voice"
        )

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_joel("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    # 0.01261 is -41 dBFS RMS for a sine, 0.5 a peak of -6 dBFS.
    as_joel("sox -n -r 48000 -c 1 quiet.wav synth 6 sine 1000 vol 0.01261")
    as_joel("sox -n -r 48000 -c 1 loud.wav synth 6 sine 1000 vol 0.5")
    # -70 dBFS RMS, and -46 at 7 kHz and at 1 kHz.
    as_joel("sox -n -r 48000 -c 1 room.wav synth 6 sine 1000 vol 0.000447")
    as_joel("sox -n -r 48000 -c 1 hiss.wav synth 6 sine 7000 vol 0.00709")
    as_joel("sox -n -r 48000 -c 1 tone.wav synth 6 sine 1000 vol 0.00709")

    with subtest("the filter starts with PipeWire"):
        filter_ready()

    with subtest("without the filter, the mic is heard as played"):
        as_joel("systemctl --user stop shure-mv7-voice")
        quiet = level("quiet", mv7, False)
        machine.log(f"quiet tone: peak {quiet[0]} dBFS, RMS {quiet[1]} dBFS")
        assert abs(quiet[1] + 41) < 1, quiet

    with subtest("the filter brings a quiet voice up 12 dB"):
        as_joel("systemctl --user start shure-mv7-voice")
        filter_ready()
        peak, rms = level("quiet", mv7, True)
        machine.log(f"quiet tone through the filter: peak {peak} dBFS, RMS {rms} dBFS")
        assert abs(rms - quiet[1] - 12) < 1, (rms, quiet)

    with subtest("the filter holds a loud voice under -1 dBFS"):
        peak, rms = level("loud", mv7, True)
        machine.log(f"loud tone through the filter: peak {peak} dBFS, RMS {rms} dBFS")
        assert -12 < peak <= -0.9, peak

    with subtest("the filter turns the room down between words"):
        peak, rms = level("room", mv7, True)
        machine.log(f"room tone through the filter: RMS {rms} dBFS")
        assert rms < -70 + 12 - 5, rms

    with subtest("the filter turns s sounds down and leaves the voice"):
        _, s_sound = level("hiss", mv7, True)
        _, voice = level("tone", mv7, True)
        machine.log(f"7 kHz through the filter: {s_sound} dBFS, 1 kHz: {voice} dBFS")
        assert abs(voice - (-46 + 12)) < 1, voice
        assert s_sound < voice - 6, (s_sound, voice)

    with subtest("another microphone is heard as played"):
        peak, rms = level("quiet", headset, False)
        machine.log(f"headset: peak {peak} dBFS, RMS {rms} dBFS")
        assert abs(rms + 41) < 1, rms
  '';
}
