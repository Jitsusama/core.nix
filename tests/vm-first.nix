# A check's virtual machine goes ahead of the builds beside it: a process
# named as qemu is, run by a build user in the Nix daemon's cgroup as
# batch at nice 5, has every thread put to the normal policy at nice -10,
# a thread it starts afterwards included, while a compiler beside it stays
# as it was.
{ self, pkgs }:
let
  # Python under qemu's name, so the kernel names it as qemu: three threads
  # at once, and a fourth once vm-first has had time to look.
  fakeQemu = pkgs.runCommandLocal "fake-qemu" { } ''
    mkdir -p $out/bin
    cp ${pkgs.python3}/bin/python3 $out/bin/qemu-system-x86_64
    cp ${pkgs.bash}/bin/bash $out/bin/cc1
  '';
  guest = pkgs.writeText "guest.py" ''
    import threading
    import time

    for _ in range(3):
        threading.Thread(target=time.sleep, args=(600,)).start()
    time.sleep(4)
    threading.Thread(target=time.sleep, args=(600,)).start()
    time.sleep(600)
  '';
  # Starts a program as a build user, batch at nice 5, inside the daemon's
  # cgroup, as the daemon would.
  asBuild = command: ''
    setsid setpriv --reuid=nixbld1 --regid=nixbld --clear-groups \
      chrt -b 0 nice -n 5 ${command} > /dev/null 2>&1 < /dev/null &
    echo $! > /sys/fs/cgroup/build.slice/nix-daemon.service/cgroup.procs
    echo $!
  '';
in
pkgs.testers.runNixOSTest {
  name = "vm-first";

  nodes.machine = {
    imports = [ self.nixosModules.nix-daemon ];
  };

  testScript = ''
    machine.wait_for_unit("vm-first.service")
    machine.succeed("systemctl start nix-daemon.service")

    def scheduling(pid):
        return machine.succeed(
            f"ps -L -o ni=,cls= -p {pid} | awk '{{print $1, $2}}' | sort -u"
        ).strip()

    vm = machine.succeed(
        """${asBuild "env PYTHONHOME=${pkgs.python3} ${fakeQemu}/bin/qemu-system-x86_64 ${guest}"}"""
    ).strip()
    cc1 = machine.succeed(
        """${asBuild "${fakeQemu}/bin/cc1 -c 'while :; do sleep 1; done'"}"""
    ).strip()

    with subtest("every thread of the VM goes ahead, one started later too"):
        machine.wait_until_succeeds(f"[ $(ls /proc/{vm}/task | wc -l) -ge 5 ]", timeout=20)
        machine.wait_until_succeeds(
            f"[ \"$(ps -L -o ni=,cls= -p {vm} | awk '{{print $1, $2}}' | sort -u)\" = '-10 TS' ]",
            timeout=10,
        )
        t.assertIn("Brought qemu-system-x86", machine.succeed("journalctl -u vm-first.service -o cat"))

    with subtest("a compiler beside it stays as the daemon set it"):
        t.assertEqual(scheduling(cc1), "5 B")
  '';
}
