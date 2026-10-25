# Presses dictation in an account with PipeWire running and watches what
# the service tells the shell. With no microphone, a press starts listening
# but no sound arrives, so the watchers never hear "listening", which is
# what the shell shows. With a stand-in microphone, they hear it once sound
# arrives, and a second press stops it. No model is loaded: the service
# loads Whisper only to transcribe, and what's heard here is silence, which
# it says rather than transcribes. The tests are dictation-listening-vm-test.py.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "dictation-listening";

  nodes.machine = {
    imports = [ "${self}/modules/pipewire/nixos.nix" ];
    users.users.alice.isNormalUser = true;
    environment.systemPackages = [
      (pkgs.callPackage "${self}/modules/dictation/package.nix" { })
      pkgs.socat
    ];
  };

  # What VM tests share, and pytest, which runs these tests (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  # Run by hand, the driver takes pytest's options from PYTEST_ADDOPTS, so
  # PYTEST_ADDOPTS='-k sounds' runs only the test with a microphone.
  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./dictation-listening-vm-test.py}")
  '';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
