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
# anything else left dead. The tests are disk-guard-vm-test.py, which pytest
# runs inside the driver (tests/lib/vm_pytest.py); one runs alone with
# `run-subtests vm-disk-guard-keeps-the-disk <words>`.
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
      # Paced as a workstation's is, but resting a second rather than a
      # commit's thirty, so the test waits on what it proves.
      pacing.rest = "1s";
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
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
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

  # The driver runs on the host, so what the tests need from Nix, the
  # drop-ins that give each guard the test's config and the build script
  # that waits, reaches them in one file, their `values` fixture.
  tests = ./disk-guard-vm-test.py;
  values =
    { nodes, ... }:
    let
      # The machine's guard as its module renders it, with its store under
      # pressure from the start and only Nix's build directories beside it,
      # which it removes itself under the unit's own sandbox.
      systemConfig = pkgs.writeText "disk-guard-system-test.json" (
        builtins.toJSON (
          nodes.machine.jitsusama.disk-guard.settings
          // {
            version = 1;
            scope = "system";
            sources = {
              inherit (nodes.machine.jitsusama.disk-guard.settings.sources) nix-builds;
            };
            bands = {
              pressure = "100%";
              urgent = "0%";
              critical = "0%";
              target = "100%";
            };
            pacing.rest = "1s";
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
      values = pkgs.writeText "disk-guard-values.json" (
        builtins.toJSON {
          systemDropIn = "${systemDropIn}";
          userDropIn = "${userDropIn}";
          waiting = "${waiting}";
        }
      );
    in
    values;
}
