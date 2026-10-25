# Plays tones in a virtual machine and measures what reaches the XPS 14's
# speakers and its headphone jack, with the speaker tuning running and
# without. A VM has neither, so two silent sinks stand in for them under the
# laptop's own names. 631 Hz sits in the tuning's deepest cut and 1356 Hz on
# one of its peaks, so through the tuning the first falls about 10 dB further
# than the second (9.6 dB down and 0.4 dB up, the limiter's input gain adding
# 3.7 dB to both). Full-scale tones at the bass peak and that peak must reach
# the speakers no louder than the limiter's ceiling. The headphones should
# hear both as they were played. The tests are speakers-vm-test.py, which
# pytest runs inside the driver.
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
  # The sinks' names, which the tests read from this file as their values
  # fixture.
  values = pkgs.writeText "speakers-values.json" (builtins.toJSON { inherit speakers headphones; });
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

  # What VM tests share, and pytest, which runs these tests (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  # Run by hand, the driver takes pytest's options from PYTEST_ADDOPTS, so
  # PYTEST_ADDOPTS='-k ceiling' runs only the limiter's test.
  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./speakers-vm-test.py}", values="${values}")
  '';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
