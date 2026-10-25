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
# The script is shure-mv7-voice-vm-test.py.
{ pkgs }:
pkgs.testers.runNixOSTest {
  name = "shure-mv7-voice";

  nodes.machine = {
    imports = [ ../modules/shure-mv7/nixos.nix ];

    users.users.joel.isNormalUser = true;
    environment.systemPackages = [ pkgs.sox ];
  };

  # What VM tests' scripts share (tests/lib), vm_pytest among them.
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  testScript = ''import vm_pytest; vm_pytest.run(globals(), "${./shure-mv7-voice-vm-test.py}")'';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}
