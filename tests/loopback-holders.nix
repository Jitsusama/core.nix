# Says which programs have a loopback camera open. What has to hold shows
# only with a real loopback and programs of another account holding it: the
# file names each one, by its program's own name even when Nix wraps it,
# leaves out the relay that fills the camera by its unit, however a program
# names itself, keeps each name to one line, catches a holder that changes
# without an open or close, empties when they close it, stays in a folder
# that outlives the camera's service and says nobody while it's down,
# follows the camera to its next loopback, takes no loopback of another
# name for it, and waits rather than failing while the camera isn't there.
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

  # Takes the name it's given, any bytes, then holds the camera open.
  renamed = pkgs.writers.writePython3Bin "renamed" { } ''
    import ctypes
    import os
    import sys
    import time

    PR_SET_NAME = 15
    name = sys.argv[1].encode().decode("unicode_escape").encode("latin-1")
    ctypes.CDLL(None).prctl(PR_SET_NAME, ctypes.c_char_p(name), 0, 0, 0)
    device = os.open(sys.argv[2], os.O_RDONLY)
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

  file = "/run/camera-holders/stand-in";
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
          unit = "stand-in.service";
          card = "Stand-in";
          ignoreUnits = [ "stand-in.service" ];
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
      environment.systemPackages = [
        pkgs.v4l-utils
        config.jitsusama.v4l2loopback.package.bin
      ];
    };

  testScript = ''
    machine.wait_for_unit("stand-in-holders.service")
    machine.wait_for_file("${file}")
    device = machine.succeed("cat /run/stand-in/device").strip()
    machine.succeed(f"udevadm wait --timeout 10 {device}")

    def holders():
        return machine.succeed("cat ${file}").split("\n")[:-1]

    def hold(unit, command, extra=""):
        machine.succeed(
            f"systemd-run --unit={unit} --uid=someone -p Type=exec {extra} -- {command}"
        )

    with subtest("the relay holding the camera isn't named"):
        relay = machine.succeed("systemctl show -P MainPID stand-in.service").strip()
        machine.wait_until_succeeds(
            f"readlink /proc/{relay}/fd/* | grep -q -x -F {device}", timeout=10
        )
        machine.sleep(1)
        t.assertEqual(holders(), [])

    with subtest("each program another account has it open with is named"):
        hold("held", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        hold("dozing", f"${holding "${doze}/bin/doze" "{device}"}")
        machine.wait_until_succeeds("grep -q -x doze ${file}", timeout=10)
        t.assertEqual(holders(), ["doze", "sleep"])
        mode = machine.succeed("stat -c %a ${file}").strip()
        t.assertEqual(mode, "644")

    with subtest("it empties once they close it"):
        machine.succeed("systemctl stop held.service dozing.service")
        machine.wait_until_succeeds("! test -s ${file}", timeout=10)

    with subtest("a program named as the relay is named, and a name is one line"):
        hold("posing", f"${pkgs.lib.getExe renamed} stand-in-relay {device}")
        hold("odd", f"'${pkgs.lib.getExe renamed}' 'a\\nb\\xff' {device}")
        machine.wait_until_succeeds("grep -q -x stand-in-relay ${file}", timeout=10)
        machine.wait_until_succeeds("grep -q '^a?b' ${file}", timeout=10)
        named = holders()
        t.assertEqual(len(named), 2, named)
        t.assertIn("stand-in-relay", named)
        machine.succeed("systemctl stop posing.service odd.service")
        machine.wait_until_succeeds("! test -s ${file}", timeout=10)

    with subtest("a holder that leaves the camera to its child is replaced by it"):
        hold(
            "forking",
            f"${pkgs.bash}/bin/bash -c 'exec 3< {device}; ${doze}/bin/doze & ${pkgs.coreutils}/bin/sleep 1'",
            # So the child outlives the parent's unit, as it would its shell.
            "-p KillMode=process",
        )
        machine.wait_until_succeeds("grep -q -x doze ${file}", timeout=10)
        machine.wait_until_succeeds("! grep -q -x bash ${file}", timeout=10)
        t.assertEqual(holders(), ["doze"])
        machine.succeed("pkill -u someone -x .doze-wrapped")
        machine.wait_until_succeeds("! test -s ${file}", timeout=10)

    with subtest("its folder outlives the camera's service, and says nobody while it's down"):
        folder = machine.succeed("stat -c %i /run/camera-holders").strip()
        hold("stopping", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        machine.wait_until_succeeds("grep -q -x sleep ${file}", timeout=10)
        # The camera stops while the program still holds it, so only the
        # service's stopping can empty the file.
        machine.succeed("systemctl stop stand-in.service")
        machine.wait_until_fails("systemctl is-active stand-in-holders.service", timeout=10)
        t.assertEqual(machine.succeed("cat ${file}"), "")
        machine.succeed("systemctl stop stopping.service")
        machine.succeed("systemctl start stand-in.service")
        machine.wait_for_unit("stand-in-holders.service")
        t.assertEqual(machine.succeed("stat -c %i /run/camera-holders").strip(), folder)

    with subtest("it follows the camera to the next loopback"):
        machine.succeed("systemctl restart stand-in.service")
        machine.wait_for_unit("stand-in-holders.service")
        device = machine.succeed("cat /run/stand-in/device").strip()
        machine.succeed(f"udevadm wait --timeout 10 {device}")
        hold("again", f"${holding "${pkgs.coreutils}/bin/sleep" "{device}"}")
        machine.wait_until_succeeds("grep -q -x sleep ${file}", timeout=10)
        machine.succeed("systemctl stop again.service")

    with subtest("another camera's loopback isn't taken for it"):
        other = machine.succeed("v4l2loopback-ctl add -n 'Other'").strip()
        machine.succeed(f"udevadm wait --timeout 10 {other}")
        machine.succeed("systemctl stop stand-in-holders.service")
        machine.succeed(f"echo {other} > /run/stand-in/device")
        machine.succeed("systemctl start stand-in-holders.service")
        hold("elsewhere", f"${holding "${pkgs.coreutils}/bin/sleep" "{other}"}")
        machine.sleep(4)
        t.assertEqual(holders(), [])
        machine.succeed("systemctl stop elsewhere.service")
        machine.succeed(f"echo {device} > /run/stand-in/device")

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
        machine.wait_until_succeeds("grep -q -x sleep ${file}", timeout=10)
        machine.succeed("systemctl stop found.service")
  '';
}
