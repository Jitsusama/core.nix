# Boots the graphical role from an encrypted disk in a virtual machine and
# answers the disk's passphrase on the keyboard, as Joel types his PIN: the
# theme's screen asks for it, shows a dot for each key, says try again after
# a wrong one in the alert frame, then lets the boot through. A slow job is
# named while the splash is up, a shutdown's last stage is said in the
# theme's words, and a restart's screen lasts to its end. The test reads
# the screen with OCR, so each step waits on what's drawn rather than on a
# timer, and photographs each state for a person to judge.
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
      alert = theme.roles.alert;
      inherit (nodes.machine.console) font;
      kmscon = nodes.machine.services.kmscon.config;
    in
    ''
      import datetime
      import filecmp
      import os
      import re
      import shlex
      import subprocess
      import tempfile
      import time
      from pathlib import Path
      from test_driver.machine.ocr import perform_ocr_variants_on_screenshot

      # The longest a wait may take to read words off the screen. A wait
      # ends when they show, so this is only a ceiling: on a host busy
      # with other machines and builds, one reading took 30 seconds, and a
      # wait of 30 then fails before it can read twice.
      reading = 120

      # A photograph of the screen, kept for a person to judge.
      def photographed(name):
          machine.screenshot(name)
          return os.path.join(machine.out_dir, f"{name}.png")

      # Photographs the screen about once a second until the console, from
      # mark on, says words. Reading a screen can take longer under load
      # than a shutdown's screen stays up, so the photographs are read only
      # afterwards, by showing.
      def photographs_until(words, mark, within):
          shots = []
          began = time.monotonic()
          while words not in machine.get_console_log()[mark:]:
              t.assertLess(time.monotonic() - began, within, f"the console never said {words}")
              # As the driver photographs for its own reading: raw, since
              # its OCR can't read PNG.
              shot = os.path.join(burst, f"{len(os.listdir(burst))}.ppm")
              machine.send_monitor_command(f"screendump {shot}")
              shots.append(shot)
              time.sleep(1)
          return shots

      # The newest of the photographs that shows words, kept under name for
      # a person to judge.
      def showing(shots, words, name):
          for shot in reversed(shots):
              if any(re.search(words, text) for text in perform_ocr_variants_on_screenshot(Path(shot))):
                  kept = os.path.join(machine.out_dir, f"{name}.png")
                  with open(kept, "wb") as png:
                      subprocess.run(["pnmtopng", shot], stdout=png, check=True)
                  return kept
          raise AssertionError(f"none of {len(shots)} photographs showed {words}")

      burst = tempfile.mkdtemp()

      # How many of a photograph's pixels are the theme's alert colour,
      # which only the frame of a refused secret is drawn in. The muted
      # hint inside it is too faint on the dark panel for OCR to read.
      def alerting(path):
          return int(subprocess.check_output([
              "${pkgs.imagemagick}/bin/magick", path, "-fuzz", "4%",
              "-fill", "black", "+opaque", "${alert}",
              "-fill", "white", "-opaque", "${alert}",
              "-format", "%[fx:round(mean*w*h)]", "info:",
          ], text=True))

      # The frame is two panels' widths and two heights of hairline, so a
      # few hundred pixels at the least.
      framed = 400

      # Waits for the screen to differ from a photograph, as it does once
      # Plymouth has drawn what a key changed.
      def changed_from(before, name, timeout=30):
          def differs(_):
              return not filecmp.cmp(before, photographed(name), shallow=False)
          retry(differs, timeout)

      # Whether a console's glyphs are the theme's font, bitmap for
      # bitmap, as many as its header counts at its size.
      def wears_the_face(console):
          machine.succeed(
              f"setfont -C {console} -O /tmp/held.psf",
              "word() { od -An -t u4 -j $(( $2 * 4 )) -N 4 $1 | tr -d ' '; }; "
              "cmp -n $(( $(word ${font} 4) * $(word ${font} 5) )) "
              "<(tail -c +33 /tmp/held.psf) <(tail -c +33 ${font})",
          )

      machine.wait_for_unit("multi-user.target")
      machine.succeed(
          "echo -n ${passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
          "echo -n ${passphrase} | cryptsetup luksOpen -q /dev/vdb root",
          "mkfs.ext4 -q /dev/mapper/root",
          "${encrypted}/bin/switch-to-configuration boot",
          "sync",
      )
      machine.crash()
      machine.start()

      with subtest("the theme asks for the disk's passphrase"):
          machine.wait_for_text("disk is locked", timeout=reading)
          asked = photographed("asks")
          t.assertLess(alerting(asked), framed // 10, "the first ask is framed in alert")

      with subtest("each key shows as a dot"):
          machine.send_chars("48")
          changed_from(asked, "typing")
          for _ in range(2):
              machine.send_key("backspace")

      with subtest("a wrong one is refused in the alert frame"):
          machine.send_chars("0000\n")
          retry(lambda _: alerting(photographed("wrong")) >= framed, 60)

      with subtest("the right one opens the disk and the boot goes on"):
          machine.send_chars("${passphrase}\n")
          machine.wait_for_unit("multi-user.target")
          t.assertIn("/dev/mapper/root on / ", machine.succeed("mount"))

      with subtest("a console signed in on draws the theme's face, with room round it"):
          # kmscon draws it, so the kernel's text of it is empty, and what
          # it drew is read from the screen and from what it says. The
          # switch is made directly: a chord sent as the compositor takes
          # the first console is lost.
          machine.succeed("chvt 3")
          machine.wait_for_unit("kmsconvt@tty3.service")
          machine.wait_for_text("login:", timeout=reading)
          said = machine.succeed("journalctl -o cat -u kmsconvt@tty3.service")
          t.assertRegex(said, r"font engine \[pango\]")
          used = re.search(r"Using \[(.*)\] size (\d+) -> cell size (\d+)x(\d+)", said)
          assert used, said
          # A face goes by more than one name, and Pango says any of them.
          names = machine.succeed(
              "${pkgs.fontconfig.bin}/bin/fc-match -f '%{family}' '${theme.font.mono.family}'"
          ).split(",")
          t.assertIn("${theme.font.mono.family}", names)
          t.assertTrue(any(used[1].startswith(f"{name} ") for name in names), used[1])
          t.assertEqual(int(used[2]), ${toString kmscon.font-size})
          width, height = int(used[3]), int(used[4])
          grid = re.findall(r"Resized terminal to (\d+)x(\d+)", said)
          t.assertEqual(
              grid[-1],
              (str((1280 - 2 * ${toString kmscon.padding-x}) // width),
               str((800 - 2 * ${toString kmscon.padding-y}) // height)),
          )
          # The padding is clear on all four sides: nothing but the
          # background in it, though the prompt sits at the left and top.
          shot = photographed("console")
          for crop in [
              "${toString kmscon.padding-x}x800+0+0",
              "${toString kmscon.padding-x}x800+${toString (1280 - kmscon.padding-x)}+0",
              "1280x${toString kmscon.padding-y}+0+0",
              "1280x${toString kmscon.padding-y}+0+${toString (800 - kmscon.padding-y)}",
          ]:
              inked = subprocess.check_output([
                  "${pkgs.imagemagick}/bin/magick", shot, "-crop", crop, "+repage",
                  "-fuzz", "4%", "-fill", "black", "-opaque", "${theme.terminal.background}",
                  "-fill", "white", "+opaque", "black",
                  "-format", "%[fx:round(mean*w*h)]", "info:",
              ], text=True)
              t.assertEqual(int(inked), 0, f"the padding at {crop} has ink")

      with subtest("a console greets by name, and signs in to a shell told it's kmscon"):
          # The encrypted root has no Nix database, so Home Manager can't
          # activate there, and the account has no shell files: an empty
          # one stands in, so zsh asks nothing of a new user.
          machine.succeed("install -o ${account} -m 600 /dev/null /home/${account}/.zshrc")
          machine.wait_for_text("Hello", timeout=reading)
          machine.send_chars("${account}\n")
          machine.wait_for_text("Password", timeout=reading)
          machine.send_chars("${password}\n")
          machine.wait_until_succeeds("pgrep -u ${account} -x zsh", timeout=30)
          # The prompt draws stand-ins only on a console that calls itself
          # linux, whose font can't hold its marks; kmscon calls itself by
          # its own name, which the machine's terminfo describes.
          told = machine.succeed(
              "tr '\\0' '\\n' < /proc/$(pgrep -u ${account} -x zsh | head -n 1)/environ"
          ).splitlines()
          t.assertIn("TERM=kmscon", told)
          machine.succeed(
              "${pkgs.ncurses}/bin/infocmp -A /run/current-system/sw/share/terminfo kmscon"
          )
          # Signing out clears the console and greets again, ready for the
          # next, and the restarts below stop no more than they did before.
          machine.send_chars("exit\n")
          machine.wait_until_fails("pgrep -u ${account} -x zsh", timeout=30)

          def greeted_afresh(_):
              shown = machine.get_screen_text()
              return "Hello" in shown and "Password" not in shown

          retry(greeted_afresh, reading)

      with subtest("the keys the greeting gives go back to the desktop"):
          machine.send_key("ctrl-alt-f1")
          machine.wait_until_succeeds("grep -qx tty1 /sys/class/tty/tty0/active", timeout=10)

      with subtest("a console kmscon can't draw still asks, in the theme's face"):
          # A new console takes the font of the one it's switched from until
          # its login prompt sets its own, so it's switched to from the
          # first, which loses the face first. While the greeter starts or
          # stops, the first console can refuse to be opened for a moment.
          machine.succeed("chvt 1")
          machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=30)
          # Drop-ins apply in their names' order, so this one comes after
          # the one that sets kmscon's command.
          machine.succeed(
              "mkdir -p /run/systemd/system/kmsconvt@tty4.service.d",
              "printf '[Service]\\nExecStart=\\nExecStart=/bin/false\\n'"
              " > /run/systemd/system/kmsconvt@tty4.service.d/zz-fails.conf",
              "systemctl daemon-reload",
              "chvt 4",
          )
          machine.wait_for_unit("getty@tty4.service")
          machine.fail("systemctl is-active kmsconvt@tty4.service")
          machine.wait_until_tty_matches("4", "login: ")
          machine.wait_until_tty_matches("4", "Hello, ${account}")
          wears_the_face("/dev/tty4")
          machine.succeed("chvt 3")

      with subtest("started on a running system with no splash, the watch ends at once"):
          # As a switch to a new configuration starts it.
          machine.succeed("systemctl start plymouth-waits.service")
          machine.wait_until_fails("systemctl is-active plymouth-waits.service", timeout=10)
          ended = machine.succeed(
              "systemctl show -p Result -p ExecMainStatus --value plymouth-waits.service"
          ).split()
          t.assertEqual(ended, ["success", "0"])

      with subtest("a slow job is named while it runs, and goes when it ends"):
          # The splash back up over the first console, as at boot, with
          # nothing graphical holding it and logind kept from handing the
          # console on, and the watch started as at boot.
          machine.succeed(
              "systemctl mask --runtime --now systemd-logind.service",
              "systemctl stop greetd.service 'getty@*' 'autovt@*' 'kmsconvt@*'",
              "chvt 1",
              "systemctl restart plymouth-start.service",
              "systemctl start plymouth-waits.service",
              "systemd-run --no-block --collect --unit=slow -p Type=oneshot"
              " -p Description='Slow Hello' sleep 60",
          )
          machine.wait_for_text("waiting on Slow Hello", timeout=60)
          photographed("waiting")
          machine.succeed("systemctl stop slow.service")
          retry(lambda _: "Slow" not in machine.get_screen_text(), reading)
          # logind stays masked and the greeter stopped, as the emergency
          # shell wants them next.
          machine.succeed(
              "systemctl stop plymouth-waits.service",
              "systemd-run --wait --collect plymouth quit",
          )

      with subtest("the emergency shell wears the face, and says what failed"):
          # As after a failed boot: a unit has failed, nothing graphical
          # holds a console and no login prompt runs, logind masked so none
          # starts again and sets the face itself, and the splash is up over
          # consoles that have lost the face. The shell's own steps before it
          # asks, Plymouth's quitting and then this module's, run in order on
          # the console in front as the shell would run them. sulogin itself
          # isn't started: it asks on every console, the test's serial line
          # among them, and the test stops hearing the machine. The splash is
          # started by systemd, as at boot: started from the test's own
          # shell, it broke the driver's reading of that shell.
          # The account's activation fails on this new root, which has no
          # Nix profile for it, so failures from before are cleared, and the
          # summary lists only the one made here.
          machine.succeed("systemctl reset-failed")
          machine.execute(
              "systemd-run --unit=breaks --description='Breaks on purpose' "
              "sh -c 'echo the disk was not there; exit 1'"
          )
          machine.wait_until_succeeds("systemctl is-failed breaks.service", timeout=30)
          machine.succeed(
              "systemctl mask --runtime --now systemd-logind.service",
              "systemctl stop greetd.service 'getty@*' 'autovt@*' 'kmsconvt@*'",
          )
          # A boot has one console, the first, which the splash draws on.
          machine.succeed("chvt 1")
          machine.wait_until_succeeds("setfont -C /dev/tty1 default8x16", timeout=30)
          machine.succeed(
              "printf '\\033[H\\033[2J' > /dev/tty1",
              "systemctl restart plymouth-start.service",
          )
          steps = re.findall(
              r"argv\[\]=([^;]*) ;", machine.succeed("systemctl show -P ExecStartPre emergency.service")
          )
          machine.succeed(
              "systemd-run --wait --collect -p StandardInput=tty-force -p StandardOutput=tty"
              " -p TTYPath=/dev/tty0 /bin/sh -c " + shlex.quote("; ".join(steps))
          )
          front = machine.succeed("fgconsole").strip()
          wears_the_face(f"/dev/tty{front}")
          said = machine.get_tty_text(front)
          for words in ["breaks.service  Breaks on purpose", "the disk was not there", "Ctrl+Alt+Delete"]:
              t.assertIn(words, said)
          photographed("emergency")

      with subtest("a restart names a slow stop and keeps its screen to the end"):
          # A service that takes its time to stop, and a program left out of
          # every unit's stop that outlasts the last stage's kill and, as
          # whatever bounds a shutdown would, says so a second later, once
          # a splash that didn't survive the kill would have gone.
          machine.succeed(
              "systemd-run --unit=slowstop -p Description='Slow Goodbye'"
              " -p ExecStop='${pkgs.coreutils}/bin/sleep 12' sleep infinity",
              "systemd-run --unit=leftover -p KillMode=none sh -c \""
              "trap '${pkgs.coreutils}/bin/sleep 1;"
              " ${pkgs.plymouth}/bin/plymouth update --status=leftovers;"
              " exec ${pkgs.coreutils}/bin/sleep 25' TERM;"
              " ${pkgs.coreutils}/bin/sleep 25 & wait\"",
          )
          mark = len(machine.get_console_log())
          machine.reboot()
          # Every wait is bounded, so a restart that stalls fails here in
          # minutes rather than holding its VM until the hour is up.
          stopping = photographs_until("Sending SIGTERM to remaining processes", mark, 120)
          last = photographs_until("Sending SIGKILL to remaining processes", mark, 60)
          showing(stopping, "waiting on Slow Goodbye", "stopping-slowly")
          showing(last, "waiting on the last programs", "last-stage")
          # The test's machine ends at a restart rather than starting again.
          machine.wait_for_shutdown(timeout=datetime.timedelta(minutes=3))
          machine.start()
          machine.wait_for_text("disk is locked", timeout=180)
          machine.send_chars("${passphrase}\n")
          machine.wait_for_unit("multi-user.target")

      # The next boot is one whose initrd fails, set up while the system is
      # whole. The encrypted disk is a new root with no profile of its own,
      # so it's given one, as nix-env would.
      machine.succeed(
          "mkdir -p /nix/var/nix/profiles",
          "ln -sfn ${stops} /nix/var/nix/profiles/system-1-link",
          "ln -sfn system-1-link /nix/var/nix/profiles/system",
          "${stops}/bin/switch-to-configuration boot",
          "sync",
      )

      with subtest("a failed initrd says what stopped it"):
          # Its shell can't be read through the test's own, which starts
          # later, so the screen is read.
          machine.crash()
          machine.start()
          for words in ["breaks.service", "the disk was not there", "Ctrl.Alt.Delete"]:
              machine.wait_for_text(words, timeout=reading)
          photographed("initrd-stops")
    '';
}
