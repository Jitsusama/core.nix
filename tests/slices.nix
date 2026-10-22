# Asks the kernel what disk latency each of the account's slices was given.
# The target is written by systemd as a slice starts, for the drive it finds
# behind /home, and the kernel takes it only for a drive whose queue it can
# watch, so reading the module's settings back would prove neither.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "slices";

  nodes.machine = {
    imports = [ self.nixosModules.slices ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
  };

  testScript = ''
    account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
    machine.wait_for_unit("user@1000.service")

    # A slice's cgroup exists only while something runs in it.
    for name in ["app", "session", "background"]:
        machine.succeed(
            f"systemd-run -M alice@ --user --slice={name}.slice --unit=in-{name}"
            " ${pkgs.coreutils}/bin/sleep infinity"
        )
        machine.wait_until_succeeds(
            f"systemctl -M alice@ --user is-active in-{name}.service", timeout=30
        )

    # The drive behind /home: the partition's parent, or the disk itself.
    source = machine.succeed("findmnt -nvo SOURCE -T /home").strip()
    parent = machine.succeed(f"lsblk -dno PKNAME {source}").strip()
    drive = parent or source.removeprefix("/dev/")
    number = machine.succeed(f"cat /sys/class/block/{drive}/dev").strip()

    def latency(name):
        return machine.succeed(f"cat {account}/{name}.slice/io.latency").strip()

    with subtest("what Joel uses has a millisecond on the drive behind /home"):
        t.assertEqual(latency("app"), f"{number} target=1000")
        t.assertEqual(latency("session"), f"{number} target=1000")

    with subtest("the work it starts has none, so it gives way"):
        t.assertEqual(latency("jobs"), "")
        t.assertEqual(latency("background"), "")
  '';
}
