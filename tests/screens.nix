# Boots the graphical role in a virtual machine with niri straight on its
# screen, not nested in Cage as the desktop test runs it, so the screen can
# really go and come back, as at an undock. The shell's surfaces must be
# there again after: the launcher, a notice that was up, the margin's marks
# and dictation listening. The script is screens-vm-test.py.
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
    { lib, ... }:
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
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
        # virtio-fs can't suspend, and refuses a sleep for the whole
        # machine, so the store is a disk image and nothing is shared, ready
        # for a sleep and wake this test is to gain. It's writable over the
        # image, since Home Manager writes the account's profile.
        useNixStoreImage = true;
        mountHostNixStore = false;
        writableStore = true;
        sharedDirectories = lib.mkForce { };
      };

      # greetd signs the account in by itself, into niri's own session.
      jitsusama.login.account = account;
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
