# Boots the graphical role in a virtual machine that doesn't sign in by
# itself, so greetd asks through the greeter: it shows, refuses a wrong
# password, takes the right one and starts the account's session, then
# shows again once that session ends, as after logging out. When the
# greeter falls over, tuigreet asks instead. It shows while home-manager
# still sets up the account, whose service manager waits for that. The
# greeter says what it does to the journal as greeter
# (modules/quickshell/Report.qml), which the test waits on rather than
# guessing how long anything takes.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";

  # Stands in for niri's session, which the VM can't run on its screen: it
  # says who it started as, and the class logind gave its session, then
  # ends, which is a logout.
  session = pkgs.writeShellScript "signed-in" ''
    class=$(${pkgs.systemd}/bin/loginctl show-session "$XDG_SESSION_ID" --property=Class --value)
    echo "$(${pkgs.coreutils}/bin/id -un) $class" \
      > /tmp/signed-in.part
    mv /tmp/signed-in.part /tmp/signed-in
  '';
in
pkgs.testers.runNixOSTest {
  name = "greeter";
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [ self.nixosModules.graphical ];

      virtualisation = {
        memorySize = 3072;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
      };

      jitsusama.login = {
        inherit account;
        automatic = false;
        session = [ "${session}" ];
        # niri refuses to drive a display through a software renderer, and
        # the VM has no GPU, so the greeter's niri runs nested in Cage.
        compositor = [
          (lib.getExe' pkgs.coreutils "env")
          "WLR_RENDERER=pixman"
          (lib.getExe pkgs.cage)
          "--"
          (lib.getExe pkgs.niri)
        ];
      };

      users.users.${account} = {
        isNormalUser = true;
        description = "Joel Gerber";
        initialPassword = secret;
        # Its service manager starts at boot, as a lingering account's does.
        linger = true;
      };
      # NixOS's test machines copy every journal line, debug included, to the
      # serial console, which is slow enough that journald falls behind a
      # burst of debug lines, and PID 1 drops any line journald won't take
      # within 10 ms. The mount burst below lost its last line that way.
      services.journald.settings.Journal.MaxLevelConsole = lib.mkForce "info";
      # Where the account's setup waits to be let go.
      systemd.tmpfiles.rules = [ "d /run/home-hold 0700 ${account} users -" ];
      home-manager.users.${account} = hm: {
        home.stateVersion = "26.05";
        # The setup waits at boot, its files linked, until the test writes
        # to this pipe, so what waits for the setup shows.
        home.activation.heldAtBoot = hm.lib.hm.dag.entryAfter [ "linkGeneration" ] ''
          ${pkgs.coreutils}/bin/mkfifo /run/home-hold/release
          read -r _ < /run/home-hold/release
        '';
        jitsusama.quickshell.reports = true;
        # No key repeats, as in the desktop test: a key let go late on a busy
        # host would type twice.
        xdg.configFile."niri/keyboard.kdl".text = lib.mkForce ''
          input {
              keyboard {
                  repeat-rate 0
              }
          }
        '';
      };
    };

  testScript = ''
    import shlex

    # A point in the greeter's journal, so only what's said after it counts.
    def mark():
        said = machine.succeed(
            "journalctl --identifier=greeter --lines=1 --show-cursor --output=cat"
        )
        return said.rsplit("-- cursor: ", 1)[1].strip()

    # Waits until the greeter says exactly this after the point, following
    # the journal as it's written, and prints what it did say if it never
    # does.
    def heard(what, since, timeout=60):
        point = f"--after-cursor={shlex.quote(since)}" if since else "--boot"
        journal = f"journalctl --identifier=greeter --output=cat --no-pager {point}"
        wanted = shlex.quote(f"report[{what}]")
        follow = shlex.quote(f"{journal} --follow | grep -m1 -aqF {wanted}")
        status, _ = machine.execute(
            f"timeout {timeout} sh -c {follow}",
            timeout=timeout + 30,
        )
        if status != 0:
            print(machine.succeed(journal))
            machine.screenshot("unheard")
        assert status == 0, f"the greeter never said {what!r}"

    with subtest("the greeter shows while the account is set up"):
        machine.wait_for_file("/run/home-hold/release", timeout=120)
        machine.wait_until_succeeds("systemctl is-active --quiet greetd.service", timeout=120)
        setup = machine.succeed(
            "systemctl show --property=ActiveState --value home-manager-${account}.service"
        ).strip()
        assert setup == "activating", f"the account's setup was {setup!r}"

    with subtest("the account's service manager waits for its setup"):
        manager = "user@$(id -u ${account}).service"
        state = machine.succeed(f"systemctl show --property=ActiveState --value {manager}").strip()
        assert state == "inactive", f"the manager was {state!r} before the setup finished"
        machine.succeed("echo go > /run/home-hold/release")
        machine.wait_until_succeeds(f"systemctl is-active --quiet {manager}", timeout=60)
        finished = int(
            machine.succeed(
                "systemctl show --property=ActiveEnterTimestampMonotonic --value"
                " home-manager-${account}.service"
            )
        )
        started = int(
            machine.succeed(
                f"systemctl show --property=InactiveExitTimestampMonotonic --value {manager}"
            )
        )
        assert started >= finished, "the manager started before the setup finished"

    with subtest("the greeter asks who's there"):
        heard("keys greeter", "", timeout=120)
        machine.screenshot("greeter")

    with subtest("the greeter refuses a wrong password"):
        since = mark()
        machine.send_chars("wrong-horse\n")
        heard("refused greeter", since)
        machine.succeed("test ! -e /tmp/signed-in")
        # Refused once, saying "try again" rather than that it couldn't check.
        status, said = machine.execute(
            f"journalctl --identifier=greeter --output=cat --after-cursor={shlex.quote(since)}"
            " | grep -c 'report.refused greeter'"
        )
        assert said.strip() == "1", f"the greeter refused {said.strip()} times"
        machine.screenshot("greeter-refused")

    with subtest("the greeter signs in with the right one"):
        since = mark()
        machine.send_chars("${secret}\n")
        heard("signs in ${account}", since)
        machine.wait_for_file("/tmp/signed-in", timeout=60)
        signed_in = machine.succeed("cat /tmp/signed-in").strip()
        assert signed_in == "${account} user", f"the session said {signed_in!r}"

    with subtest("the greeter asks again after a logout"):
        since = mark()
        heard("keys greeter", since)
        machine.succeed("! pgrep -u greeter -x tuigreet")

    with subtest("tuigreet asks when the greeter falls over"):
        # The brackets keep the test's own shell from matching.
        machine.succeed("pkill -u greeter -f '[q]uickshell[^ ]* -p '")
        machine.wait_until_succeeds("pgrep -u greeter -x tuigreet", timeout=60)
        machine.succeed("! pgrep -u greeter -f '[b]in/niri -c'")

    # systemd starts no mount while it's over its limit on changes to the
    # mount table, which held the mounts the greeter waits for at boot. Each
    # systemd-mount waits for its mount, so each is a change of its own, and
    # systemd says at debug level when the limit is reached. Debug goes only
    # to the journal, since the serial console slows systemd enough to spread
    # the burst out, and journald keeps it off the console too, so it keeps
    # up and the last mount's line in the journal means all that came before
    # it is there too.
    with subtest("a burst of mounts doesn't wait on systemd's limit"):
        target = machine.succeed("systemd-analyze log-target").strip()
        machine.succeed("systemd-analyze log-target journal")
        machine.succeed("systemd-analyze log-level debug")
        machine.succeed(
            "for n in $(seq 12); do"
            " timeout 30 systemd-mount --quiet --type=tmpfs tmpfs /mnt/burst$n || exit 1;"
            " done"
        )
        machine.succeed("systemd-analyze log-level info")
        machine.succeed(f"systemd-analyze log-target {shlex.quote(target)}")
        machine.wait_until_succeeds(
            "journalctl --boot _PID=1 --output=cat | grep -qxF 'Mounted /mnt/burst12.'",
            timeout=30,
        )
        machine.fail(
            "journalctl --boot _PID=1 --output=cat"
            " | grep -F 'mount-monitor-dispatch) entered rate limit state'"
        )
  '';
}
