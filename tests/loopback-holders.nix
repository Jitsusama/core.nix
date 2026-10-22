# Says which programs have a loopback camera open. What has to hold shows
# only with a real loopback and programs of another account holding it: the
# file names each one, by its program's own name even when Nix wraps it,
# leaves out the relay that fills the camera, empties when they close it,
# follows the camera when its service makes a new loopback, and waits
# rather than failing while the camera it's pointed at isn't there.
{ self, pkgs }:
let
  # Holds the loopback it's given open as a relay would, writing nothing.
  relay = pkgs.writers.writePython3Bin "stand-in-relay" { } ''
    import os
    import sys
    import time

    device = os.open(sys.argv[1], os.O_RDWR)
    while True:
        time.sleep(60)
  '';

  # A program as Nix wraps one, whose process is .doze-wrapped: it waits, so
  # it holds whatever it's handed open until it's stopped.
  doze =
    pkgs.runCommand "doze"
      {
        nativeBuildInputs = [ pkgs.makeWrapper ];
        waits = pkgs.writeCBin "doze" "#include <unistd.h>\nint main(void) { for (;;) pause(); }\n";
      }
      ''
        mkdir -p $out/bin
        cp $waits/bin/doze $out/bin/doze
        wrapProgram $out/bin/doze
      '';

  # Runs a program, as someone else, with the camera it's given held open.
  holding = program: device: "${pkgs.bash}/bin/bash -c 'exec 3< ${device}; exec ${program} 300'";
in
pkgs.testers.runNixOSTest {
  name = "loopback-holders";

  nodes.machine =
    { config, ... }:
    let
      loopback = "${config.jitsusama.v4l2loopback.package.bin}/bin/v4l2loopback-ctl";
    in
    {
      imports = [ "${self}/modules/v4l2loopback/nixos.nix" ];
      jitsusama.v4l2loopback = {
        enable = true;
        holders.stand-in = {
          device = "/run/stand-in/device";
          output = "/run/stand-in/in-use";
          unit = "stand-in.service";
          ignore = [ "stand-in-relay" ];
        };
      };
      systemd.services.stand-in = {
        wantedBy = [ "multi-user.target" ];
        # As the camera's relay does, since nothing else loads the module.
        wants = [ "modprobe@v4l2loopback.service" ];
        after = [ "modprobe@v4l2loopback.service" ];
        serviceConfig.RuntimeDirectory = "stand-in";
        preStart = "${loopback} add -n 'Stand-in' > /run/stand-in/device";
        script = ''exec ${pkgs.lib.getExe relay} "$(cat /run/stand-in/device)"'';
        postStop = ''${loopback} delete "$(cat /run/stand-in/device)" || true'';
      };
      users.users.someone = {
        isNormalUser = true;
        extraGroups = [ "video" ];
      };
      environment.systemPackages = [ pkgs.v4l-utils ];
    };

  testScript = ''
    machine.wait_for_unit("stand-in-holders.service")
    machine.wait_for_file("/run/stand-in/in-use")
    device = machine.succeed("cat /run/stand-in/device").strip()
    machine.succeed(f"udevadm wait --timeout 10 {device}")

    def holders():
        return machine.succeed("cat /run/stand-in/in-use").split()

    def hold(unit, command):
        machine.succeed(
            f"systemd-run --unit={unit} --uid=someone -p Type=exec -- {command}"
        )

    with subtest("the relay holding the camera isn't named"):
        t.assertEqual(holders(), [])

    with subtest("each program another account has it open with is named"):
        hold("held", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        hold("dozing", f"${holding "${doze}/bin/doze" "{device}"}")
        machine.wait_until_succeeds("grep -q -x doze /run/stand-in/in-use", timeout=10)
        t.assertEqual(holders(), ["doze", "sleep"])
        mode = machine.succeed("stat -c %a /run/stand-in/in-use").strip()
        t.assertEqual(mode, "644")

    with subtest("it empties once they close it"):
        machine.succeed("systemctl stop held.service dozing.service")
        machine.wait_until_succeeds("! test -s /run/stand-in/in-use", timeout=10)

    with subtest("it follows the camera to the next loopback"):
        machine.succeed("systemctl restart stand-in.service")
        machine.wait_for_unit("stand-in-holders.service")
        device = machine.succeed("cat /run/stand-in/device").strip()
        machine.succeed(f"udevadm wait --timeout 10 {device}")
        hold("again", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        machine.wait_until_succeeds("grep -q -x sleep /run/stand-in/in-use", timeout=10)
        machine.succeed("systemctl stop again.service")

    with subtest("it waits while the camera isn't there, and finds it when it is"):
        machine.succeed("systemctl stop stand-in-holders.service")
        machine.succeed("echo /dev/video-not-yet > /run/stand-in/device")
        machine.succeed("systemctl start stand-in-holders.service")
        machine.sleep(4)
        machine.succeed("systemctl is-active stand-in-holders.service")
        restarts = machine.succeed("systemctl show -P NRestarts stand-in-holders.service")
        t.assertEqual(restarts.strip(), "0")
        t.assertEqual(holders(), [])
        machine.succeed(f"echo {device} > /run/stand-in/device")
        hold("found", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        machine.wait_until_succeeds("grep -q -x sleep /run/stand-in/in-use", timeout=10)
        machine.succeed("systemctl stop found.service")
  '';
}
