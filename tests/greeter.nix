# Boots the graphical role in a virtual machine that doesn't sign in by
# itself, so greetd asks through the greeter: it shows, refuses a wrong
# password, takes the right one and starts the account's session, then
# shows again once that session ends, as after logging out. When the
# greeter falls over, tuigreet asks instead. It shows while home-manager
# still sets up the account, whose service manager waits for that. The
# greeter says what it does to the journal as greeter
# (modules/quickshell/Report.qml), which the test waits on rather than
# guessing how long anything takes. The script is greeter-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";

  # What the tests need of the above, as their values fixture.
  values = pkgs.writeText "greeter-values.json" (builtins.toJSON { inherit account secret; });

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
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "greeter";
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    {
      config,
      lib,
      pkgs,
      ...
    }:
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
        # the VM has no GPU, so the greeter's niri runs nested in Cage. It's
        # the machine's niri, with its patches, since the config it reads
        # holds options only they add; nixpkgs's niri falls back to its own
        # defaults and never starts the greeter.
        compositor = [
          (lib.getExe' pkgs.coreutils "env")
          "WLR_RENDERER=pixman"
          (lib.getExe pkgs.cage)
          "--"
          (lib.getExe config.programs.niri.package)
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

  tests = ./greeter-vm-test.py;
  inherit values;
}
