# Checks a webcam's microphone yields to other microphones in a virtual
# machine. A VM has no microphones, so two dummy sound cards stand in, and
# udev marks the second a webcam, as systemd's sound rules do for a USB
# camera with a microphone. WirePlumber picks the input it ranks highest, so
# the webcam's must rank below a USB microphone's 2100 and above a laptop's
# own, which starts from 1600. The script is microphones-vm-test.py.
{ pkgs }:
pkgs.testers.runNixOSTest {
  name = "microphones";

  nodes.machine = {
    imports = [ ../modules/pipewire/nixos.nix ];

    # Sound cards are opened to whoever sits at the seat, and nobody does
    # here, so the account reads them as a member of audio.
    users.users.joel = {
      isNormalUser = true;
      extraGroups = [ "audio" ];
    };
    environment.systemPackages = [ pkgs.jq ];
    boot.kernelModules = [ "snd-dummy" ];
    boot.extraModprobeConfig = "options snd-dummy enable=1,1 index=0,1";
    services.udev.extraRules = ''
      SUBSYSTEM=="sound", KERNEL=="card1", ENV{SOUND_FORM_FACTOR}="webcam"
    '';
  };

  # What VM tests' scripts share (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  testScript = builtins.readFile ./microphones-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
