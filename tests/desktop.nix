# Starts the graphical role's desktop in a virtual machine and uses it the
# way Joel does: a notification appears, the volume and brightness bars come
# and go as those change, the launcher finds and opens kitty, the lock screen
# refuses a wrong password and takes the right one, polkit's prompt lets the
# account start a system service, and GTK programs and monospaced text are in
# the theme. niri says which surfaces it drew, photographs show their
# colours, to the test and to a person looking, and the portal and fontconfig
# answer for the settings. Quickshell has to start without a single QML
# error.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";

  # Text views and file lists draw on the theme's dark background, which
  # nothing else on the screen uses, so a window that hasn't drawn can't pass
  # for one that has.
  text = pkgs.writeText "text" "A text view.";
in
pkgs.testers.runNixOSTest {
  name = "desktop";
  # Reads the colours of the windows it photographs.
  extraPythonPackages = python: [ python.pillow ];
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

      # The VM has neither a backlight nor a sound card, so it gets one of each
      # that drives nothing, which the kernel and PipeWire report as they do
      # the laptop's.
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-backlight { })
      ];
      boot.kernelModules = [ "stand-in-backlight" ];
      services.pipewire.extraConfig.pipewire."90-stand-in-speakers"."context.objects" = [
        {
          factory = "adapter";
          args = {
            "factory.name" = "support.null-audio-sink";
            "node.name" = "stand-in-speakers";
            "media.class" = "Audio/Sink";
            "audio.position" = [
              "FL"
              "FR"
            ];
          };
        }
      ];

      virtualisation = {
        memorySize = 4096;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
      };

      users.users.${account} = {
        isNormalUser = true;
        uid = 1000;
        # Polkit asks a member of wheel for their own password.
        extraGroups = [ "wheel" ];
        initialPassword = secret;
      };
      home-manager.users.${account}.home.stateVersion = "26.05";
      environment.systemPackages = [ pkgs.libnotify ];

      # Starting a system service is something only an admin may do, so systemd
      # asks polkit, and polkit asks the account through its agent.
      systemd.services.polkit-allowed = {
        serviceConfig.Type = "oneshot";
        script = "touch /run/polkit-allowed";
      };

      # niri refuses to drive a display through a software renderer, and the
      # VM has no GPU, so it runs nested in Cage, which renders in software,
      # in place of greetd. Everything inside niri is the same, but niri can't
      # run as a session there, so the test hands its variables to systemd.
      services.greetd.enable = lib.mkForce false;
      services.cage = {
        enable = true;
        user = account;
        program = "${pkgs.niri}/bin/niri";
        environment.WLR_RENDERER = "pixman";
      };
      # Stands in for niri's own unit, which starts the graphical session, and
      # with it the desktop portal.
      systemd.user.services.compositor = {
        bindsTo = [ "graphical-session.target" ];
        before = [ "graphical-session.target" ];
        wants = [ "graphical-session-pre.target" ];
        after = [ "graphical-session-pre.target" ];
        serviceConfig.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
      };
    };

  testScript =
    { nodes, ... }:
    let
      inherit (nodes.machine.home-manager.users.${account}.jitsusama.theme) colors font;
    in
    ''
      import json
      import os
      import shlex
      import time

      from PIL import Image

      # A command as the account, inside its niri session: qs finds the shell by
      # the display it's on.
      def in_session(command):
          environment = (
              "export XDG_RUNTIME_DIR=/run/user/1000; "
              "export $(systemctl --user show-environment | grep -E '^(WAYLAND_DISPLAY|NIRI_SOCKET)='); "
          )
          return "su - ${account} -c " + shlex.quote(environment + command)

      def as_account(command):
          return machine.succeed(in_session(command))

      def wait_for_surface(namespace):
          machine.wait_until_succeeds(in_session(f"niri msg layers | grep -q {namespace}"), timeout=30)

      def window_titled(title):
          windows = json.loads(as_account("niri msg --json windows"))
          return next(window for window in windows if window["title"] == title)

      # The commonest colour inside a window, a little in from its edges.
      # niri says where a window is only once it floats, so it floats first.
      def commonest_colour_in(title, picture):
          machine.wait_until_succeeds(
              in_session(f"niri msg --json windows | grep -qF '\"title\":\"{title}\"'"), timeout=30
          )
          as_account(f"niri msg action move-window-to-floating --id {window_titled(title)['id']}")
          time.sleep(1)
          machine.screenshot(picture)
          window = window_titled(title)
          print(window)
          layout = window["layout"]
          left = int(layout["tile_pos_in_workspace_view"][0] + layout["window_offset_in_tile"][0])
          top = int(layout["tile_pos_in_workspace_view"][1] + layout["window_offset_in_tile"][1])
          width, height = layout["window_size"]
          image = Image.open(os.path.join(machine.out_dir, picture + ".png")).convert("RGB")
          inside = image.crop((left + 20, top + 20, left + width - 20, top + height - 20))
          # Pillow gives no counts when there are more colours than asked for,
          # and there can't be more than there are pixels.
          counts = inside.getcolors(inside.width * inside.height)
          assert counts is not None
          _, colour = max(counts)
          as_account(f"niri msg action close-window --id {window['id']}")
          return "#%02x%02x%02x" % colour

      machine.wait_for_unit("multi-user.target")
      machine.wait_until_succeeds("ls /run/user/1000/niri.*.sock", timeout=60)
      as_account(
          "NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg action spawn --"
          " systemctl --user import-environment WAYLAND_DISPLAY NIRI_SOCKET"
      )
      machine.wait_until_succeeds(in_session("systemctl --user show-environment | grep -q WAYLAND_DISPLAY"))
      # niri sets these as a session, then starts it.
      as_account("systemctl --user set-environment XDG_CURRENT_DESKTOP=niri XDG_SESSION_TYPE=wayland")
      as_account("systemctl --user start compositor")

      with subtest("Quickshell starts without a QML error"):
          as_account("systemctl --user start quickshell")
          machine.wait_until_succeeds(in_session("qs ipc show | grep -q launcher"), timeout=60)
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          print(log)
          assert "ERROR" not in log and "Error:" not in log, "Quickshell reported an error"

      with subtest("a notification appears"):
          as_account("notify-send 'Build finished' 'optimus is ready'")
          wait_for_surface("notifications")
          machine.sleep(1)
          machine.screenshot("notification")

      with subtest("the volume shows when it changes, then goes"):
          as_account("wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.4")
          wait_for_surface("volume")
          # niri lists a surface before it has drawn its first frame.
          time.sleep(0.5)
          machine.screenshot("volume")
          machine.wait_until_fails(in_session("niri msg layers | grep -q volume"), timeout=10)

      # niri runs the brightness keys' command inside Joel's session, which is
      # the one logind lets change a backlight, so the test has niri run it too.
      with subtest("the brightness shows when the keys' command changes it, then goes"):
          as_account("niri msg action spawn -- brightnessctl --class=backlight set 80%")
          machine.wait_until_succeeds("grep -qx 80 /sys/class/backlight/stand-in/brightness", timeout=10)
          wait_for_surface("brightness")
          time.sleep(0.5)
          machine.screenshot("brightness")
          machine.wait_until_fails(in_session("niri msg layers | grep -q brightness"), timeout=10)

      with subtest("the launcher finds and opens kitty"):
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          machine.send_chars("kitty")
          machine.sleep(1)
          machine.screenshot("launcher")
          machine.send_key("ret")
          machine.wait_until_succeeds(in_session("niri msg windows | grep -i kitty"), timeout=30)

      with subtest("the lock screen takes only the right password"):
          as_account("qs ipc call lock lock")
          machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q true"), timeout=30)
          machine.sleep(1)
          machine.send_chars("wrong-password\n")
          machine.sleep(5)
          as_account("qs ipc call lock isLocked | grep -q true")
          machine.screenshot("locked")
          machine.send_chars("${secret}\n")
          machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q false"), timeout=30)

      with subtest("polkit's prompt lets the account start a system service"):
          as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
          wait_for_surface("polkit")
          machine.sleep(1)
          machine.screenshot("polkit")
          machine.send_chars("${secret}\n")
          machine.wait_for_file("/run/polkit-allowed", timeout=30)

      # Chrome and Slack ask the desktop portal, and websites hear it from them.
      with subtest("the desktop says it prefers dark"):
          reply = as_account(
              "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
              " org.freedesktop.portal.Settings ReadOne ss org.freedesktop.appearance color-scheme"
          )
          t.assertIn("u 1", reply)

      with subtest("monospace is the theme's font"):
          family = as_account("${pkgs.fontconfig.bin}/bin/fc-match -f '%{family}' monospace")
          t.assertIn("${font.family}", family)

      with subtest("GTK programs draw in the theme's colours"):
          zenity = "${pkgs.zenity}/bin/zenity --text-info --title=libadwaita --filename=${text}"
          # Here niri runs in Cage's system service, and the portal refuses the
          # programs it starts ("Unable to open /proc/<pid>/root"), so zenity
          # runs under the account's user manager, where niri's own unit puts
          # them on the laptop.
          as_account(f"systemd-run --user --collect {zenity}")
          t.assertEqual(commonest_colour_in("libadwaita", "libadwaita"), "${colors.dark_background}".lower())
          # The file chooser Chrome and Slack open, which the GTK portal draws
          # with GTK 3, asked for the way they ask.
          as_account(
              "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
              " org.freedesktop.portal.FileChooser OpenFile 'ssa{sv}' \"\" Open 0"
          )
          t.assertEqual(commonest_colour_in("Open", "file-chooser"), "${colors.dark_background}".lower())

      machine.screenshot("desktop")
    '';
}
