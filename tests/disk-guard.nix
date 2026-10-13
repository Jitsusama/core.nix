# Runs the disk guard in a virtual machine, the machine's daemon and an
# account's, and drives the disks through the bands. Both start on the configs
# their modules render; then the test gives them bands it can reach by filling
# a 4 GiB Btrfs /home, compressed as a workstation's is. At pressure an idle
# target goes while a fresh one and a cache stay. At critical the cache and
# the fresh target go, but a real cargo build, waiting in its build script
# with cargo's locks held, keeps its own until it ends. The account takes
# nothing at all without the machine's guard vouching for the processes it
# can't read. A target on a disk too full to make a trash in is deleted in
# place. With its store under pressure, the machine's guard expires all but
# the newest five of a profile's generations and collects what that and
# anything else left dead.
{ self, pkgs }:
let

  # The account's guard, watching only what the test fills.
  userConfig = pkgs.writeText "disk-guard-user-test.json" (
    builtins.toJSON {
      version = 1;
      scope = "user";
      status_file = true;
      notify = false;
      bands = {
        pressure = "40%";
        urgent = "25%";
        critical = "10%";
        target = "50%";
      };
      sources = {
        src = {
          kind = "cargo";
          roots = [
            "/home/joel/src"
            "/srv/small"
          ];
        };
        thing = {
          kind = "cache";
          path = "/home/joel/.cache/thing";
          rank = 10;
        };
      };
    }
  );

  # A build script that waits for a file, so cargo holds its locks and runs
  # from the target until the test lets it end.
  waiting = pkgs.writeText "build.rs" ''
    fn main() {
        while !std::path::Path::new("/home/joel/release").exists() {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
  '';
in
pkgs.testers.runNixOSTest {
  name = "disk-guard";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.disk-guard
    ];

    virtualisation = {
      memorySize = 2048;
      cores = 2;
      # What Nix adds to the store lands on a disk, as on a machine.
      writableStoreUseTmpfs = false;
      emptyDiskImages = [
        {
          size = 4096;
          driveConfig.deviceExtraOpts.serial = "home";
        }
        {
          size = 256;
          driveConfig.deviceExtraOpts.serial = "small";
        }
      ];
      fileSystems = {
        "/home" = {
          device = "/dev/disk/by-id/virtio-home";
          fsType = "btrfs";
          autoFormat = true;
          options = [
            "compress=zstd:1"
            "noatime"
          ];
        };
        "/srv/small" = {
          device = "/dev/disk/by-id/virtio-small";
          fsType = "ext4";
          autoFormat = true;
        };
      };
    };

    environment.systemPackages = [
      pkgs.cargo
      pkgs.jq
      pkgs.rustc
      pkgs.stdenv.cc
    ];

    users.users.joel = {
      isNormalUser = true;
      uid = 1000;
    };
    # Activation makes the home before the disk mounts over /home, so it's
    # made again on the disk, ahead of home-manager.
    systemd.tmpfiles.rules = [ "d /home/joel 0700 joel users -" ];
    home-manager.users.joel = {
      imports = [ self.homeModules.disk-guard ];
      home.stateVersion = "26.05";
    };
  };

  testScript =
    { nodes, ... }:
    let
      # The machine's guard as its module renders it, with its store under
      # pressure from the start and nothing else to look after.
      systemConfig = pkgs.writeText "disk-guard-system-test.json" (
        builtins.toJSON (
          nodes.machine.jitsusama.disk-guard.settings
          // {
            version = 1;
            scope = "system";
            sources = { };
            bands = {
              pressure = "100%";
              urgent = "0%";
              critical = "0%";
              target = "100%";
            };
          }
        )
      );
      dropIn = config: ''
        [Service]
        ExecStart=
        ExecStart=/run/current-system/sw/bin/disk-guard ${config.scope} --config ${config.file}
      '';
      systemDropIn = pkgs.writeText "system.conf" (dropIn {
        scope = "system";
        file = systemConfig;
      });
      userDropIn = pkgs.writeText "user.conf" (dropIn {
        scope = "user";
        file = userConfig;
      });
    in
    ''
      import shlex

      def as_joel(command, succeed=True):
          environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
          run = machine.succeed if succeed else machine.fail
          return run("su - joel -c " + shlex.quote(environment + command))

      def wait_for_band(path, band, timeout=300):
          machine.wait_until_succeeds(
              "jq -e '.filesystems[] | select(.path == \"" + path + "\") | .band == \"" + band + "\"'"
              " /run/user/1000/disk-guard/status.json",
              timeout=timeout,
          )

      fills = 0

      # Fills /home until its free space is this share of it, with files no
      # source names. The fill goes in while the account's guard is stopped,
      # so it starts on a fresh rate rather than forecasting a worse band
      # from a jump only a test makes. Btrfs counts unallocated space as free
      # that one allocation can't wholly take, so it goes in pieces, and its
      # free space only shows a piece once the transaction commits.
      def fill_to(share):
          global fills
          as_joel("systemctl --user stop disk-guard")
          while True:
              free, total = (
                  int(word) for word in as_joel("df -B1 --output=avail,size /home | tail -1").split()
              )
              wanted = int(free - share * total)
              fills += 1
              piece = f"fallocate -l {min(wanted, 256 << 20)} ~/fill-{fills} && sync -f ~"
              if wanted <= 0 or as_joel(piece + " && echo ok || true").strip() != "ok":
                  break
          as_joel("systemctl --user start disk-guard")

      # A built target, with a file of its own so its size is felt.
      def built(name, megabytes):
          as_joel(f"cargo new -q --lib --vcs none ~/src/{name}")
          as_joel(f"cd ~/src/{name} && cargo build -q --offline")
          as_joel(f"head -c {megabytes}M /dev/urandom > ~/src/{name}/target/debug/bulk")

      def gone(path):
          machine.wait_until_succeeds(f"! test -e {path}", timeout=300)

      def there(path):
          machine.succeed(f"test -e {path}")

      machine.wait_for_unit("multi-user.target")
      machine.succeed("loginctl enable-linger joel")
      machine.wait_for_unit("user@1000.service")

      with subtest("both guards run on the configs their modules render"):
          machine.wait_for_unit("disk-guard.service")
          machine.wait_for_unit("disk-guard.service", "joel")
          t.assertEqual(machine.succeed("stat -c %a /run/disk-guard/socket").strip(), "666")
          machine.wait_until_succeeds("jq -e '.filesystems | length > 0' /run/disk-guard/status.json")
          machine.wait_until_succeeds(
              "jq -e '.filesystems | length > 0' /run/user/1000/disk-guard/status.json"
          )

      with subtest("the account's guard takes the test's bands"):
          machine.succeed("chown joel:users /srv/small")
          as_joel("mkdir -p ~/src ~/.cache/thing ~/.config/systemd/user/disk-guard.service.d")
          as_joel("cp ${userDropIn} ~/.config/systemd/user/disk-guard.service.d/test.conf")
          as_joel("head -c 300M /dev/urandom > ~/.cache/thing/blob")
          built("idle", 700)
          built("fresh", 300)
          # Built three days ago, and nothing since.
          as_joel("find ~/src/idle -exec touch -h -d '3 days ago' {} +")
          as_joel("systemctl --user daemon-reload && systemctl --user restart disk-guard")
          machine.wait_until_succeeds(
              "jq -e '.filesystems | length == 2' /run/user/1000/disk-guard/status.json"
          )
          wait_for_band("/home", "healthy")

      with subtest("at pressure an idle target goes, and a fresh one and a cache stay"):
          # The idle target alone leaves the pass short of its target, so only
          # the fresh one's window keeps it.
          fill_to(0.26)
          gone("/home/joel/src/idle/target/debug/bulk")
          wait_for_band("/home", "healthy")
          there("/home/joel/src/fresh/target/debug/bulk")
          there("/home/joel/.cache/thing/blob")
          there("/home/joel/src/idle/Cargo.toml")

      with subtest("at critical a build holding its locks keeps its target until it ends"):
          as_joel("cargo new -q --lib --vcs none ~/src/busy && cp ${waiting} ~/src/busy/build.rs")
          as_joel("mkdir -p ~/src/busy/target/debug")
          as_joel("head -c 200M /dev/urandom > ~/src/busy/target/debug/bulk")
          as_joel("cd ~/src/busy && setsid -f cargo build -q --offline > ~/busy.log 2>&1")
          machine.wait_until_succeeds("pgrep -u joel -f build-script-build")
          fill_to(0.05)
          gone("/home/joel/.cache/thing/blob")
          gone("/home/joel/src/fresh/target/debug/bulk")
          there("/home/joel/src/busy/target/debug/bulk")
          machine.succeed("pgrep -u joel -f build-script-build")
          as_joel("touch ~/release")
          machine.wait_until_fails("pgrep -u joel -x cargo", timeout=120)
          # Built a moment ago, it waits for critical.
          fill_to(0.05)
          gone("/home/joel/src/busy/target/debug/bulk")

      with subtest("once the fill goes the disk is healthy again"):
          as_joel("rm ~/fill-*")
          wait_for_band("/home", "healthy")

      with subtest("a target on a disk too full for a trash is deleted in place"):
          as_joel("cargo new -q --lib --vcs none /srv/small/tiny")
          as_joel("cd /srv/small/tiny && cargo build -q --offline")
          as_joel("head -c 40M /dev/urandom > /srv/small/tiny/target/debug/bulk")
          # Held open, so it's kept until the disk is full.
          as_joel("setsid -f sleep infinity < /srv/small/tiny/target/debug/bulk > /dev/null 2>&1")
          # ext4 gives back what delayed allocation over-reserved once written
          # out, so the fill goes on until a write after a sync finds nothing.
          as_joel(
              "for piece in 1M 4k 4k 4k; do"
              " dd if=/dev/zero of=/srv/small/fill-$piece-$RANDOM bs=$piece 2>/dev/null;"
              " sync -f /srv/small; done; true"
          )
          as_joel("mkdir /srv/small/room", succeed=False)
          machine.succeed("pkill -u joel -x sleep")
          gone("/srv/small/tiny/target/debug/bulk")
          machine.succeed("ls /srv/small/fill-*")
          as_joel("! ls -a /srv/small/tiny | grep -q disk-guard-trash")

      with subtest("with its store pressed the machine expires generations and collects"):
          paths = []
          for n in range(1, 9):
              machine.succeed(f"mkdir -p /root/gen-{n} && echo {n} > /root/gen-{n}/n")
              paths.append(machine.succeed(f"nix-store --add /root/gen-{n}").strip())
              machine.succeed(f"nix-env -p /nix/var/nix/profiles/test --set {paths[-1]}")
          machine.succeed("echo junk > /root/junk")
          junk = machine.succeed("nix-store --add /root/junk").strip()
          machine.succeed(
              "mkdir -p /run/systemd/system/disk-guard.service.d",
              "cp ${systemDropIn} /run/systemd/system/disk-guard.service.d/test.conf",
              "systemctl daemon-reload",
              "systemctl restart disk-guard",
          )
          gone(junk)
          for n in range(1, 4):
              gone(f"/nix/var/nix/profiles/test-{n}-link")
              gone(paths[n - 1])
          for n in range(4, 9):
              there(f"/nix/var/nix/profiles/test-{n}-link")
              there(paths[n - 1])
          t.assertEqual(machine.succeed("readlink /nix/var/nix/profiles/test").strip(), "test-8-link")
    '';
}
