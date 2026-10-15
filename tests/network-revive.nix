# Runs network-revive in a virtual machine, on an account whose services need
# the network, the way a laptop's do. The machine's only network is a dummy
# link networkd keeps, down at first, as a network that comes late is. Each
# stand-in service starts only once that link reaches beyond the machine, as
# a program that looks a name up as it starts does, and is left in a state
# the network let it down into: one failed for good, one waiting an hour to
# restart, one stopped on purpose, one whose check never passes, and a
# one-shot job with no check, as a backup is. Once the link comes up, each
# that should starts, at once and with one line saying so; the one stopped
# stays stopped, and the one never ready stays down, with a line saying
# why. One that runs reloads when the servers resolved asks change: once
# for a burst however long, and not at all for a burst that ends where it
# began. It keeps following networkd through a restart, and when it starts
# itself, it starts what it finds down. A start asked just as systemd
# executes itself again, as a switch has it do, is asked again once it's
# back.
{ self, pkgs }:
let
  inherit (pkgs) lib;

  # Starts only while the uplink reaches 10.0.0.53, then runs until stopped.
  # QEMU's own network is on another link networkd keeps up throughout,
  # with a default route, so the route has to go by the uplink. Its --check
  # says whether it could start; --never that it never could. --reexec
  # checks as --check does, but the first time it passes it has the
  # account's systemd execute itself again, without waiting for it to go, so
  # the start that follows reaches systemd as it leaves.
  standIn = pkgs.writeShellScript "stand-in" ''
    export PATH=${
      lib.makeBinPath [
        pkgs.iproute2
        pkgs.coreutils
        pkgs.gnugrep
        pkgs.systemd
      ]
    }
    reaches() {
      ip -o route get 10.0.0.53 2> /dev/null | grep -q " dev uplink "
    }
    case "''${1-}" in
      --check) reaches ;;
      --never) echo "never ready" >&2; exit 1 ;;
      --reexec)
        reaches || exit 1
        if [ ! -e "$HOME/reexecuted" ]; then
          touch "$HOME/reexecuted"
          busctl --user call --expect-reply=no org.freedesktop.systemd1 \
            /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager Reexecute
        fi
        ;;
      --once) reaches || { echo "10.0.0.53 is out of reach" >&2; exit 1; } ;;
      *)
        reaches || { echo "10.0.0.53 is out of reach" >&2; exit 1; }
        exec sleep infinity
        ;;
    esac
  '';

  # Writes the servers resolved asks on the link at each reload, so the
  # test can tell one reload from another.
  reloaded = pkgs.writeShellScript "reloaded" ''
    ${pkgs.systemd}/bin/resolvectl dns uplink | ${pkgs.gnused}/bin/sed "s/^.*: //" >> "$HOME/reloads"
  '';

  service = settings: {
    Service = {
      ExecStart = "${standIn}";
    }
    // settings;
  };
in
pkgs.testers.runNixOSTest {
  name = "network-revive";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.network-revive
    ];

    virtualisation.vlans = [ ];
    systemd.network = {
      # Nothing waits for a network the test brings up itself.
      wait-online.enable = false;
      netdevs."10-uplink".netdevConfig = {
        Name = "uplink";
        Kind = "dummy";
      };
      networks."10-uplink" = {
        matchConfig.Name = "uplink";
        address = [ "10.0.0.2/24" ];
        dns = [ "10.0.0.53" ];
        linkConfig.ActivationPolicy = "manual";
      };
    };

    users.users.joel = {
      isNormalUser = true;
      uid = 1000;
      linger = true;
    };
    home-manager.users.joel = {
      home.stateVersion = "26.05";
      # What it finds each time it looks, so the test can tell when it has.
      jitsusama.network-revive.debug = true;
      jitsusama.network-revive.units = {
        "needs-network.service".check = [ "--check" ];
        "waits-long.service".check = [ "--check" ];
        "stopped.service".check = [ "--check" ];
        "never-ready.service".check = [ "--never" ];
        "follows-names.service" = {
          check = [ "--check" ];
          reload = true;
        };
        "backs-up.service" = { };
        "meets-reexec.service".check = [ "--reexec" ];
      };
      systemd.user.services = {
        # As a proxy's: it tries five times a second apart, then fails.
        needs-network = service {
          Restart = "always";
          RestartSec = 1;
        };
        waits-long = service {
          Restart = "always";
          RestartSec = "1h";
        };
        stopped = service { };
        never-ready = service {
          Restart = "always";
          RestartSec = 1;
        };
        follows-names = service { ExecReload = "${reloaded}"; };
        backs-up.Service = {
          Type = "oneshot";
          ExecStart = "${standIn} --once";
        };
        meets-reexec = service { };
      };
    };
  };

  testScript = ''
    import re
    import shlex

    def as_joel(command):
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    def state(unit):
        return as_joel(f"systemctl --user show -P ActiveState,SubState,Result {unit}").split()

    def wait_for_state(unit, *expected):
        machine.wait_until_succeeds(
            "su - joel -c "
            + shlex.quote(
                "export XDG_RUNTIME_DIR=/run/user/1000; "
                f"test \"$(systemctl --user show -P ActiveState,SubState {unit} | paste -sd ' ')\""
                f" = {shlex.quote(' '.join(expected))}"
            ),
            timeout=120,
        )

    def said():
        return as_joel("journalctl --user -u network-revive -o cat")

    def times_said(pattern):
        return len([line for line in said().splitlines() if re.fullmatch(pattern, line)])

    # Waits for the daemon to say a line matching pattern more than it had.
    def wait_for_more(pattern, than):
        journal = "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 journalctl --user -u network-revive -o cat'"
        machine.wait_until_succeeds(
            f"test $({journal} | grep -Ecx {shlex.quote(pattern)}) -gt {than}", timeout=60
        )

    def reloads():
        return as_joel("cat ~/reloads 2>/dev/null || true").splitlines()

    def wait_for_reloads(count):
        machine.wait_until_succeeds(f"test $(wc -l < /home/joel/reloads) -ge {count}", timeout=60)
        return reloads()

    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("network-revive.service", "joel")

    with subtest("without the network, each service is let down as the network leaves it"):
        as_joel("systemctl --user start --no-block needs-network waits-long never-ready backs-up")
        wait_for_state("needs-network", "failed", "failed")
        t.assertEqual(state("needs-network")[2], "start-limit-hit")
        wait_for_state("waits-long", "activating", "auto-restart")
        wait_for_state("never-ready", "failed", "failed")
        wait_for_state("backs-up", "failed", "failed")

    with subtest("once the network comes, what it let down starts at once"):
        machine.succeed("networkctl up uplink")
        wait_for_state("needs-network", "active", "running")
        wait_for_state("waits-long", "active", "running")
        wait_for_state("backs-up", "inactive", "dead")
        t.assertEqual(state("backs-up")[2], "success")
        lines = said()
        t.assertEqual(
            lines.count("started needs-network.service again, which had failed, as the network changed"),
            1,
            lines,
        )
        t.assertIn("started waits-long.service now, not at its next try, as the network changed", lines)

    with subtest("a service whose check never passes stays down, and the journal says why"):
        machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 journalctl --user -u network-revive -o cat'"
            " | grep -qx 'left never-ready.service down: its check still failed 31 s"
            " after the network changed: never ready'",
            timeout=120,
        )
        t.assertEqual(state("never-ready")[:2], ["failed", "failed"])

    with subtest("a running service reloads once for each change, and a stopped one stays stopped"):
        as_joel("systemctl --user start follows-names stopped")
        as_joel("systemctl --user stop stopped")
        machine.succeed("resolvectl dns uplink 10.0.0.54")
        t.assertEqual(wait_for_reloads(1), ["10.0.0.54"])
        # Ending where it began, a burst changes nothing.
        unchanged = times_said("the machine reaches what it did")
        machine.succeed("resolvectl dns uplink 10.0.0.55 && resolvectl dns uplink 10.0.0.54")
        wait_for_more("the machine reaches what it did", unchanged)
        # Ending elsewhere, however long, it's one change.
        machine.succeed(
            "for server in 56 57 58 59 60; do resolvectl dns uplink 10.0.0.$server; done"
        )
        machine.wait_until_succeeds("grep -q 10.0.0.60 /home/joel/reloads", timeout=60)
        t.assertEqual(reloads(), ["10.0.0.54", "10.0.0.60"])
        t.assertEqual(state("stopped")[:2], ["inactive", "dead"])
        t.assertEqual(state("needs-network")[:2], ["active", "running"])

    with subtest("it follows networkd through a restart"):
        looked = times_said("the machine reaches .*")
        machine.succeed("systemctl restart systemd-networkd")
        machine.wait_until_succeeds("networkctl status uplink | grep -q routable", timeout=60)
        wait_for_more("the machine reaches .*", looked)
        machine.succeed("resolvectl dns uplink 10.0.0.61")
        machine.wait_until_succeeds("grep -q 10.0.0.61 /home/joel/reloads", timeout=60)

    with subtest("it hears resolved come back asking other servers"):
        # Nothing changes for networkd to hand over, so resolved coming
        # back is all there is to hear.
        machine.succeed(
            "mkdir -p /run/systemd/resolved.conf.d"
            " && printf '[Resolve]\\nDNS=10.0.0.99\\n' > /run/systemd/resolved.conf.d/other.conf"
            " && systemctl restart systemd-resolved"
        )
        wait_for_more("the machine reaches .*10[.]0[.]0[.]99%0.*", 0)

    with subtest("starting, it starts what it finds down"):
        as_joel("systemctl --user stop network-revive")
        machine.succeed("networkctl down uplink")
        as_joel("systemctl --user restart --no-block needs-network")
        wait_for_state("needs-network", "failed", "failed")
        machine.succeed("networkctl up uplink")
        machine.wait_until_succeeds("ip -o route get 10.0.0.53 | grep -q ' dev uplink '", timeout=60)
        as_joel("systemctl --user start network-revive")
        wait_for_state("needs-network", "active", "running")
        t.assertIn("started needs-network.service again, which had failed\n", said() + "\n")

    with subtest("a start asked as systemd executes itself again is asked again"):
        # Heard going, so its coming back is a change of its own rather than
        # the end of a burst that changed nothing.
        looked = times_said("the machine reaches .*")
        machine.succeed("networkctl down uplink")
        wait_for_more("the machine reaches .*", looked)
        as_joel("systemctl --user start --no-block meets-reexec")
        wait_for_state("meets-reexec", "failed", "failed")
        machine.succeed("networkctl up uplink")
        wait_for_state("meets-reexec", "active", "running")
        machine.succeed("test -e /home/joel/reexecuted")
        machine.succeed("journalctl _UID=1000 _COMM=systemd -o cat | grep -qx Reexecuting.")
        t.assertIn(
            "started meets-reexec.service again, which had failed, as the network changed", said()
        )

    with subtest("systemd refused nothing it was asked for"):
        t.assertEqual(times_said("couldn't .*"), 0, said())
  '';
}
