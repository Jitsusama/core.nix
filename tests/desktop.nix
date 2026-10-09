# Starts the graphical role's desktop in a virtual machine and uses it the way
# Joel does: a notification appears, the launcher finds and opens kitty, the
# lock screen refuses a wrong password and takes the right one, and polkit's
# prompt lets the account start a system service. Each part is checked by asking niri for
# the surface it drew, and photographed for a person to look at. Quickshell
# has to start without a single QML error.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";
in
pkgs.testers.runNixOSTest {
  name = "desktop";
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [ self.nixosModules.graphical ];

      virtualisation = {
        memorySize = 4096;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
      };

      users.users.${account} = {
        isNormalUser = true;
        uid = 1000;
        # Polkit asks a member of wheel for their own password.
        extraGroups = [ "wheel" ];
        initialPassword = secret;
      };
      home-manager.users.${account}.home.stateVersion = "26.05";
      environment.systemPackages = [ pkgs.libnotify ];

      # Starting a system service is something only an admin may do, so systemd
      # asks polkit, and polkit asks the account through its agent.
      systemd.services.polkit-allowed = {
        serviceConfig.Type = "oneshot";
        script = "touch /run/polkit-allowed";
      };

      # niri refuses to drive a display through a software renderer, and the
      # VM has no GPU, so it runs nested in Cage, which renders in software,
      # in place of greetd. Everything inside niri is the same, but niri can't
      # run as a session there, so the test hands its variables to systemd.
      services.greetd.enable = lib.mkForce false;
      services.cage = {
        enable = true;
        user = account;
        program = "${pkgs.niri}/bin/niri";
        environment.WLR_RENDERER = "pixman";
      };
    };

  testScript = ''
    import shlex

    # A command as the account, inside its niri session: qs finds the shell by
    # the display it's on.
    def in_session(command):
        environment = (
            "export XDG_RUNTIME_DIR=/run/user/1000; "
            "export $(systemctl --user show-environment | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)='); "
        )
        return "su - ${account} -c " + shlex.quote(environment + command)

    def as_account(command):
        return machine.succeed(in_session(command))

    def wait_for_surface(namespace):
        machine.wait_until_succeeds(in_session(f"niri msg layers | grep -q {namespace}"), timeout=30)

    machine.wait_for_unit("multi-user.target")
    machine.wait_until_succeeds("ls /run/user/1000/niri.*.sock", timeout=60)
    as_account(
        "NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg action spawn --"
        " systemctl --user import-environment WAYLAND_DISPLAY NIRI_SOCKET"
    )
    machine.wait_until_succeeds(in_session("systemctl --user show-environment | grep -q WAYLAND_DISPLAY"))

    with subtest("Quickshell starts without a QML error"):
        as_account("systemctl --user start quickshell")
        machine.wait_until_succeeds(in_session("qs ipc show | grep -q launcher"), timeout=60)
        log = as_account("journalctl --user -u quickshell --no-pager -o cat")
        print(log)
        assert "ERROR" not in log and "Error:" not in log, "Quickshell reported an error"

    with subtest("a notification appears"):
        as_account("notify-send 'Build finished' 'optimus is ready'")
        wait_for_surface("notifications")
        machine.sleep(1)
        machine.screenshot("notification")

    with subtest("the launcher finds and opens kitty"):
        as_account("qs ipc call launcher toggle")
        wait_for_surface("launcher")
        machine.send_chars("kitty")
        machine.sleep(1)
        machine.screenshot("launcher")
        machine.send_key("ret")
        machine.wait_until_succeeds(in_session("niri msg windows | grep -i kitty"), timeout=30)

    with subtest("the lock screen takes only the right password"):
        as_account("qs ipc call lock lock")
        machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q true"), timeout=30)
        machine.sleep(1)
        machine.send_chars("wrong-password\n")
        machine.sleep(5)
        as_account("qs ipc call lock isLocked | grep -q true")
        machine.screenshot("locked")
        machine.send_chars("${secret}\n")
        machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q false"), timeout=30)

    with subtest("polkit's prompt lets the account start a system service"):
        as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
        wait_for_surface("polkit")
        machine.sleep(1)
        machine.screenshot("polkit")
        machine.send_chars("${secret}\n")
        machine.wait_for_file("/run/polkit-allowed", timeout=30)

    machine.screenshot("desktop")
  '';
}
