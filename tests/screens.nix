# Boots the graphical role in a virtual machine with niri straight on its
# screen, not nested in Cage as the desktop test runs it, so the screen can
# really go and come back, as at an undock. niri must first have held each
# screen's first frame for the wallpaper, and a niri from before that hold
# must still take the account's settings, as it reloads them at a switch
# while it runs. The shell's surfaces must be
# there again after: the launcher, a notice that was up, the margin's marks
# and dictation listening. Then the console is switched away and back, and
# the machine sleeps as a laptop does and wakes at a key, and they must all
# be there again after each; after the wake, the shell must still hear what
# it's told over the bus, dictation's socket and a unit's state. Last, as a
# dock's two screens, one goes and comes back: the margin and dictation's
# bar must stay on the other and the margin come back on both. The script
# is screens-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";

  # What the tests need of the above, as their values fixture.
  values = pkgs.writeText "screens-values.json" (builtins.toJSON { inherit account; });

  # Stands in for dictation's service, which would load Whisper, as the
  # desktop test's does (desktop/stand-in-dictation.py): it listens from
  # the first press until the next.
  standInDictation = pkgs.writers.writePython3 "stand-in-dictation" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-dictation.py);
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
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
      # The machine's store as two read-only images, built once for each
      # closure as derivations of their own, rather than afresh at every
      # start as useNixStoreImage does. The base holds what the machine's
      # packages, fonts, graphics and kernel need, which an edit to the
      # shell or a module's settings leaves alone; the rest holds what's
      # left, the system, its units and the account's files among it. So
      # a Quickshell edit makes only the rest again: about 330 MiB of the
      # 8.8 GiB closure, where one image made all of it every time. Both
      # are packed as NixOS packs one (nixos/lib/erofs-store-image.nix),
      # uncompressed, but in the tar's own order. Sorting by path,
      # mkfs.erofs's default for a tar, or compressing first copies every
      # file aside: packing the rest wrote 474 MiB for a 174 MiB image with
      # lz4, and 309 MiB for a 290 MiB one without. The images are bigger,
      # but the drive is spared what a build writes and throws away. The
      # registration the machine loads at boot goes in the rest.
      registration =
        (import "${pkgs.path}/nixos/lib/store-registration-info.nix" {
          hostPkgs = pkgs;
          rootPaths = config.virtualisation.additionalPaths;
        }).regInfo;
      closure = pkgs.closureInfo {
        rootPaths = [
          config.system.build.toplevel
          registration
        ];
      };
      base = pkgs.closureInfo {
        rootPaths = [
          config.system.path
          config.home-manager.users.${account}.home.path
          config.system.modulesTree
          config.boot.kernelPackages.kernel
          config.hardware.graphics.package
        ]
        ++ config.hardware.graphics.extraPackages
        ++ config.fonts.packages;
      };
      # Packs the store paths a file names into an image with a label and
      # a UUID of its own.
      storeImage =
        name: label: uuid: paths:
        pkgs.runCommand name
          {
            nativeBuildInputs = [
              pkgs.gnutar
              pkgs.erofs-utils
            ];
          }
          ''
            tar --create --absolute-names --verbatim-files-from \
              --transform 'flags=rSh;s|/nix/store/||' \
              --files-from ${paths} \
              | mkfs.erofs --quiet --sort=none --force-uid=0 --force-gid=0 -L ${label} \
                -U ${uuid} -T 0 --hard-dereference --tar=f "$out"
          '';
      baseImage =
        storeImage "screens-store-base.img" "nix-store-base" "eb176051-bd15-49b7-9e6b-462e0b467019"
          "${base}/store-paths";
      # What the closure holds that the base doesn't.
      rest = pkgs.runCommand "screens-store-rest" { } ''
        sort ${base}/store-paths > base
        sort ${closure}/store-paths | comm -23 - base > "$out"
      '';
      restImage =
        storeImage "screens-store-rest.img" "nix-store-rest" "5d1b8a2e-6c3f-4f0a-9b7e-2a4c8e1f3d60"
          rest;
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

      # The machine's niri as it was before the hold on its first frame came
      # in, as a niri started before a switch still is, for the script to
      # read the account's settings with.
      environment.etc."niri-before-the-hold".source = config.programs.niri.package.overrideAttrs (old: {
        patches = lib.filter (patch: baseNameOf (toString patch) != "hold-first-frame.patch") old.patches;
      });

      virtualisation = {
        memorySize = 4096;
        cores = 4;
        # QEMU's plain VGA card, which bochs-drm drives. virtio-gpu's driver
        # can't suspend yet (Linux 6.18): after a wake its queue never
        # drains, and niri waits on it for good. This card has no render
        # node, so the patch lets niri draw through the card's own node.
        # A second card is a second screen, of another size, as a dock's is,
        # and niri draws on both. Each is named, so the script can ask QEMU
        # for the picture on either.
        qemu.options = [
          "-vga none -device VGA,id=first,xres=1280,yres=800,edid=on"
          "-device secondary-vga,id=second,xres=1024,yres=768,edid=on"
        ];
        # virtio-fs can't suspend, and refuses a sleep for the whole
        # machine, so the store is the images above and nothing is shared.
        # The host's store stays on in name, so NixOS still registers the
        # store and lays a writable layer over it, where Home Manager writes
        # the account's profile; the images stand where the share was, the
        # rest over the base, which share no path. The rest is mounted where
        # the share was, since the registration is in it, and the kernel's
        # command line names it there.
        mountHostNixStore = true;
        writableStore = true;
        sharedDirectories = lib.mkForce { };
        qemu.drives = [
          {
            name = "nix-store-base";
            file = "${baseImage}";
            driveExtraOpts = {
              format = "raw";
              readonly = "on";
            };
          }
          {
            name = "nix-store-rest";
            file = "${restImage}";
            driveExtraOpts = {
              format = "raw";
              readonly = "on";
            };
          }
        ];
        fileSystems = {
          "/nix/.ro-store" = {
            device = "/dev/disk/by-label/nix-store-rest";
            fsType = "erofs";
            neededForBoot = true;
            options = [ "ro" ];
          };
          "/nix/.ro-store-base" = {
            device = "/dev/disk/by-label/nix-store-base";
            fsType = "erofs";
            neededForBoot = true;
            options = [ "ro" ];
          };
          "/nix/store".overlay.lowerdir = lib.mkForce [
            "/nix/.ro-store"
            "/nix/.ro-store-base"
          ];
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

  tests = ./screens-vm-test.py;
  inherit values;
}
