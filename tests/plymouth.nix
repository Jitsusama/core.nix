# Boots the graphical role from an encrypted disk in a virtual machine and
# answers the disk's passphrase on the keyboard, as Joel types his PIN: the
# theme's screen asks for it, shows a dot for each key, says try again after
# a wrong one in the alert frame, then lets the boot through. A slow job is
# named while the splash is up, a shutdown's last stage is said in the
# theme's words, and a restart's screen lasts to its end. The test reads
# the screen with OCR, so each step waits on what's drawn rather than on a
# timer, and photographs each state for a person to judge. The script is
# plymouth-vm-test.py.
{ self, pkgs }:
let
  account = "jitsusama";
  passphrase = "4815";
  password = "a console's password";
in
pkgs.testers.runNixOSTest {
  name = "plymouth";
  enableOCR = true;
  # The base role sets nixpkgs' own settings.
  node.pkgsReadOnly = false;

  nodes.machine =
    { lib, pkgs, ... }:
    {
      imports = [ self.nixosModules.graphical ];
      environment.systemPackages = [ pkgs.cryptsetup ];

      virtualisation = {
        memorySize = 3072;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
        emptyDiskImages = [ 512 ];
        useBootLoader = true;
        useEFIBoot = true;
        # Booting off the encrypted disk needs the store from the host.
        mountHostNixStore = true;
      };
      boot.loader.systemd-boot.enable = true;
      boot.initrd.systemd.enable = true;
      # The GPU's driver in the initrd, as a machine's hardware module puts
      # it, so Plymouth has a screen to draw on before the disk opens.
      boot.initrd.kernelModules = [ "virtio_gpu" ];
      # The test reads the machine through a serial console, and Plymouth
      # keeps to text where there's one unless told it needn't.
      boot.kernelParams = [ "plymouth.ignore-serial-consoles" ];

      jitsusama.login.account = account;
      users.users.${account} = {
        isNormalUser = true;
        initialPassword = password;
      };
      home-manager.users.${account}.home.stateVersion = "26.05";

      specialisation.encrypted.configuration = {
        boot.initrd.luks.devices = lib.mkVMOverride {
          root.device = "/dev/vdb";
        };
        virtualisation.rootDevice = "/dev/mapper/root";
      };
      # An initrd that can't finish, as one whose disk never comes can't.
      specialisation.stops.configuration = {
        # The test framework panics a test machine whose initrd fails, to
        # fail fast; a real machine waits in the shell.
        boot.initrd.systemd.services.panic-on-fail.enable = false;
        boot.initrd.systemd.services.breaks = {
          description = "Breaks on purpose";
          wantedBy = [ "initrd.target" ];
          before = [ "initrd.target" ];
          unitConfig = {
            OnFailure = "emergency.target";
            # As initrd.target fails over.
            OnFailureJobMode = "replace-irreversibly";
          };
          serviceConfig = {
            Type = "oneshot";
            ExecStart = "/bin/bash -c 'echo the disk was not there; exit 1'";
          };
        };
      };
    };

  testScript =
    { nodes, ... }:
    let
      encrypted = nodes.machine.specialisation.encrypted.configuration.system.build.toplevel;
      stops = nodes.machine.specialisation.stops.configuration.system.build.toplevel;
      theme = nodes.machine.home-manager.users.${account}.jitsusama.theme;
      inherit (nodes.machine.console) font;
      kmscon = nodes.machine.services.kmscon.config;
      # What the script needs from Nix, which it reads from this file at its
      # top. ImageMagick runs beside the driver, and the other programs are
      # named by their paths in the store, as the script always named them,
      # so none of them needs the machine's PATH.
      values = pkgs.writeText "plymouth-values.json" (
        builtins.toJSON {
          inherit account passphrase password;
          encrypted = "${encrypted}";
          stops = "${stops}";
          alert = theme.roles.alert;
          inherit (theme.terminal) background;
          monoFamily = theme.font.mono.family;
          font = "${font}";
          fontSize = kmscon.font-size;
          paddingX = kmscon.padding-x;
          paddingY = kmscon.padding-y;
          magick = "${pkgs.imagemagick}/bin/magick";
          fcMatch = "${pkgs.fontconfig.bin}/bin/fc-match";
          infocmp = "${pkgs.ncurses}/bin/infocmp";
          sleep = "${pkgs.coreutils}/bin/sleep";
          plymouth = "${pkgs.plymouth}/bin/plymouth";
        }
      );
      script = builtins.readFile ./plymouth-vm-test.py;
    in
    builtins.replaceStrings [ "@values@" ] [ "${values}" ] script;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}
