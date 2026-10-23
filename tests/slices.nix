# Asks the kernel what disk latency each slice was given, the account's and
# the system's.
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

    machine.succeed(
        "systemd-run --slice=machine.slice --unit=in-machine"
        " ${pkgs.coreutils}/bin/sleep infinity"
    )
    machine.wait_for_unit("in-machine.service")

    def latency(name, under=account):
        return machine.succeed(f"cat {under}/{name}.slice/io.latency").strip()

    with subtest("what Joel uses has a millisecond on the drive behind /home"):
        t.assertEqual(latency("app"), f"{number} target=1000")
        t.assertEqual(latency("session"), f"{number} target=1000")

    with subtest("the work it starts has none, so it gives way"):
        t.assertEqual(latency("jobs"), "")
        t.assertEqual(latency("background"), "")

    with subtest("among the system's, every account has it, and machines none"):
        t.assertEqual(latency("user", "/sys/fs/cgroup"), f"{number} target=1000")
        t.assertEqual(latency("machine", "/sys/fs/cgroup"), "")
        t.assertEqual(latency("system", "/sys/fs/cgroup"), "")

    # systemd turns a share into bytes of the memory the kernel reports.
    total = int(machine.succeed("awk '/MemTotal/ {print $2}' /proc/meminfo")) * 1024

    def floor(path):
        return int(machine.succeed(f"cat {path}/memory.min"))

    def near(path, share):
        t.assertAlmostEqual(floor(path), total * share, delta=2**20, msg=path)

    with subtest("apps keep a hard share of memory, and every slice above more"):
        near(f"{account}/app.slice", 0.13)
        t.assertEqual(floor(f"{account}/session.slice"), 2**30)
        near(account, 0.15)
        near("/sys/fs/cgroup/user.slice/user-1000.slice", 0.15)
        near("/sys/fs/cgroup/user.slice", 0.15)
        t.assertEqual(floor(f"{account}/jobs.slice"), 0)

    jobs = f"{account}/jobs.slice"
    timed = f"{jobs}/jobs-timed.slice"

    with subtest("timed work is inside idle jobs, far above its holder"):
        machine.wait_until_succeeds(
            "systemctl -M alice@ --user is-active timed.service", timeout=30
        )
        t.assertEqual(machine.succeed(f"cat {jobs}/cpu.idle").strip(), "1")
        t.assertEqual(machine.succeed(f"cat {timed}/cpu.weight").strip(), "10000")
        machine.wait_until_succeeds(
            "systemctl -M alice@ --user is-active awaited.service", timeout=30
        )
        t.assertEqual(
            machine.succeed(f"cat {jobs}/jobs-awaited.slice/cpu.weight").strip(), "1000"
        )
        t.assertEqual(
            machine.succeed(f"cat {jobs}/jobs.service/cpu.weight").strip(), "100"
        )

    with subtest("work in jobs moves itself in, as pi's waiting shell does"):
        machine.succeed(
            "systemd-run -M alice@ --user --slice=jobs.slice --unit=mover"
            " --expand-environment=no -p KillMode=process ${pkgs.bash}/bin/bash -c"
            f" 'echo $$ > {timed}/timed.service/cgroup.procs;"
            " cat /proc/self/cgroup > /tmp/moved; exec sleep infinity'"
        )
        machine.wait_until_succeeds("grep -q timed.service /tmp/moved", timeout=30)
  '';
}
