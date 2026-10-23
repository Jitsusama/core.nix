# Checks a webcam's microphone yields to other microphones in a virtual
# machine. A VM has no microphones, so two dummy sound cards stand in, and
# udev marks the second a webcam, as systemd's sound rules do for a USB
# camera with a microphone. WirePlumber picks the input it ranks highest, so
# the webcam's must rank below a USB microphone's 2100 and above a laptop's
# own, which starts from 1600.
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

  testScript = ''
    import shlex

    def as_joel(command):
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    # The rank WirePlumber gave the microphone on a card, by the card's
    # number.
    def rank(card):
        return as_joel(
            "pw-dump | jq -r '.[] | .info.props? // empty"
            ' | select(."media.class" == "Audio/Source"'
            f' and ."alsa.card" == {card}) | ."priority.session"'
            "'"
        ).strip()

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_joel("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    machine.wait_until_succeeds(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-dump'"
        " | jq -e '[.[] | select(.info.props?.\"media.class\" == \"Audio/Source\")] | length == 2'"
    )

    with subtest("a webcam's microphone yields to a USB microphone"):
        webcam = rank(1)
        machine.log(f"the webcam's microphone ranks {webcam}")
        assert webcam == "1900", webcam

    with subtest("other microphones keep their rank"):
        other = rank(0)
        machine.log(f"the other microphone ranks {other}")
        assert other not in ("", "1900"), other
  '';
}
