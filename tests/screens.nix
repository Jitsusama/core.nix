# Boots the graphical role in a virtual machine with niri straight on its
# screen, not nested in Cage as the desktop test runs it, so the screen can
# really go and come back, as at an undock. The shell's surfaces must be
# there again after: the launcher, a notice that was up, the margin's marks
# and dictation listening. Then the console is switched away and back, and
# the machine sleeps as a laptop does and wakes at a key, and they must all
# be there again after each. The script is screens-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";

  # What the script needs of the above, which it reads at its top.
  values = pkgs.writeText "screens-values.json" (builtins.toJSON { inherit account; });

  # Stands in for dictation's service, which would load Whisper, as the
  # desktop test's does (desktop/stand-in-dictation.py): it listens from
  # the first press until the next.
  standInDictation = pkgs.writers.writePython3 "stand-in-dictation" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-dictation.py);
in
pkgs.testers.runNixOSTest {
  name = "screens";
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      # The machine's store as a read-only image, packed as NixOS packs one
      # for useNixStoreImage, but built once, as a derivation of its own,
      # rather than afresh at every start. Packing it wrote about 17 GiB
      # each run; now that's written once for each closure, and a run
      # writes a few MiB. The registration the machine loads at boot goes
      # in with it.
      registration =
        (import "${pkgs.path}/nixos/lib/store-registration-info.nix" {
          hostPkgs = pkgs;
          rootPaths = config.virtualisation.additionalPaths;
        }).regInfo;
      storeImage = pkgs.runCommand "screens-store.img" { } (
        import "${pkgs.path}/nixos/lib/erofs-store-image.nix" {
          hostPkgs = pkgs;
          storePaths = "${
            pkgs.closureInfo {
              rootPaths = [
                config.system.build.toplevel
                registration
              ];
            }
          }/store-paths";
          label = "nix-store";
          destination = "$out";
        }
      );
    in
    {
      imports = [ self.nixosModules.graphical ];

      # The VM has no GPU, only llvmpipe, which niri as it ships won't draw
      # with. The patch goes under the machine's own, which niri's module
      # adds to this package.
      nixpkgs.overlays = [
        (_: prev: {
          niri = prev.niri.overrideAttrs (old: {
            patches = (old.patches or [ ]) ++ [ ./niri-software-renderer.patch ];
          });
        })
      ];

      virtualisation = {
        memorySize = 4096;
        cores = 4;
        # QEMU's plain VGA card, which bochs-drm drives. virtio-gpu's driver
        # can't suspend yet (Linux 6.18): after a wake its queue never
        # drains, and niri waits on it for good. This card has no render
        # node, so the patch lets niri draw through the card's own node.
        qemu.options = [ "-vga none -device VGA,xres=1280,yres=800,edid=on" ];
        # virtio-fs can't suspend, and refuses a sleep for the whole
        # machine, so the store is the image above and nothing is shared.
        # The host's store stays on in name, so NixOS still registers the
        # store and lays a writable layer over it, where Home Manager writes
        # the account's profile; the image stands where the share was.
        mountHostNixStore = true;
        writableStore = true;
        sharedDirectories = lib.mkForce { };
        qemu.drives = [
          {
            name = "nix-store";
            file = "${storeImage}";
            driveExtraOpts = {
              format = "raw";
              readonly = "on";
            };
          }
        ];
        fileSystems."/nix/.ro-store" = {
          device = "/dev/disk/by-label/nix-store";
          fsType = "erofs";
          neededForBoot = true;
          options = [ "ro" ];
        };
      };

      # greetd signs the account in by itself, into niri's own session.
      jitsusama.login.account = account;
      # Laptops of this age sleep by s2idle alone, so the machine does too.
      boot.kernelParams = [ "mem_sleep_default=s2idle" ];
      users.users.${account} = {
        isNormalUser = true;
        uid = 1000;
        initialPassword = "correct-horse";
      };
      home-manager.users.${account} = {
        home.stateVersion = "26.05";
        jitsusama.quickshell.reports = true;
        systemd.user.services.dictation.Service.ExecStart = lib.mkForce "${standInDictation}";
      };
      # As in the greeter test: debug lines copied to the serial console make
      # journald fall behind, and PID 1 drops lines it won't take.
      services.journald.settings.Journal.MaxLevelConsole = lib.mkForce "info";
    };

  # The script reads the values from the file put in place of its marker.
  testScript = builtins.replaceStrings [ "@values@" ] [ "${values}" ] (
    builtins.readFile ./screens-vm-test.py
  );
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
