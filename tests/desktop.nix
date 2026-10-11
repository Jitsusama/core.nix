# Starts the graphical role's desktop in a virtual machine and uses it the
# way Joel does: the wallpaper stands behind the workspaces, the overview
# shows the time and status, a notification appears, the volume and
# brightness bars come and go as those change, the launcher finds and opens
# kitty and remembers it was picked, a terminal opens below the focused
# window, the launcher switches to a window by its title, the lock screen
# refuses a wrong password and takes the right one, polkit's prompt lets the
# account start a system service, the clipboard takes and gives text, and
# GTK programs and monospaced text are in the theme. niri says which surfaces it drew,
# photographs show their colours, to the test and to a person looking, and
# the portal and fontconfig answer for the settings. Quickshell has to start
# without a single QML error.
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
      imports = [
        self.nixosModules.graphical
        # The laptop role's power profiles, whose units start only at boot.
        self.nixosModules.power-profiles-daemon
      ];

      # The VM has neither a backlight nor a sound card, so it gets one of each
      # that drives nothing, which the kernel and PipeWire report as they do
      # the laptop's.
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-backlight { })
      ];
      boot.kernelModules = [
        "stand-in-backlight"
        # The kernel's own stand-in battery, which UPower reports as a laptop's.
        "test_power"
      ];
      # The overview's status reads the battery from UPower and the network from
      # NetworkManager, which the laptop role brings.
      services.upower.enable = true;
      networking.networkmanager.enable = true;
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
      inherit (nodes.machine.home-manager.users.${account}.jitsusama.theme) roles font;
    in
    ''
      import json
      import os
      import shlex
      import time

      from PIL import Image, ImageStat

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

      # Where a floating window draws on the screen: left, top, width, height.
      def drawn_at(window):
          layout = window["layout"]
          left = int(layout["tile_pos_in_workspace_view"][0] + layout["window_offset_in_tile"][0])
          top = int(layout["tile_pos_in_workspace_view"][1] + layout["window_offset_in_tile"][1])
          width, height = layout["window_size"]
          return left, top, width, height

      # The commonest colour inside a window, a little in from its edges.
      # niri says where a window is only once it floats, so it floats first.
      # The first GTK window has taken over 30 seconds to open on a CI runner
      # building the kernel beside the test, so this waits longer than the rest.
      def commonest_colour_in(title, picture):
          machine.wait_until_succeeds(
              in_session(f"niri msg --json windows | grep -qF '\"title\":\"{title}\"'"), timeout=120
          )
          as_account(f"niri msg action move-window-to-floating --id {window_titled(title)['id']}")
          time.sleep(1)
          machine.screenshot(picture)
          window = window_titled(title)
          print(window)
          left, top, width, height = drawn_at(window)
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

      # A loop in the order units start breaks nothing at a switch, when the
      # targets are already reached, so only a boot shows it.
      with subtest("the machine boots without an ordering cycle"):
          machine.fail("journalctl -b -o cat | grep -F 'ordering cycle'")

      with subtest("the power profile follows the charger from boot"):
          machine.wait_until_succeeds(
              "journalctl -b -o cat -u power-profile-follows-the-charger | grep -F 'power profile:'", timeout=30
          )

      with subtest("Quickshell starts without a QML error"):
          as_account("systemctl --user start quickshell")
          machine.wait_until_succeeds(in_session("qs ipc show | grep -q launcher"), timeout=60)
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          print(log)
          assert "ERROR" not in log and "Error:" not in log, "Quickshell reported an error"

      # How much of the theme's foreground the left quarter of the screen draws,
      # which the overview leaves free of workspaces.
      def foreground_on_the_left(picture):
          machine.screenshot(picture)
          image = Image.open(os.path.join(machine.out_dir, picture + ".png")).convert("RGB")
          left = image.crop((0, 0, image.width // 4, image.height))
          counts = left.getcolors(left.width * left.height)
          assert counts is not None
          foreground = tuple(int("${roles.text}"[i : i + 2], 16) for i in (1, 3, 5))
          return sum(count for count, colour in counts if colour == foreground)

      # The screen, and how many colours a part of it holds, which a flat
      # colour has one of, and how bright that part is, from 0 to 255. The part
      # is a box of fractions of the screen, left, top, right and bottom.
      def colours_and_light(picture, part=(0, 0, 1, 1)):
          machine.screenshot(picture)
          image = Image.open(os.path.join(machine.out_dir, picture + ".png")).convert("RGB")
          w, h = image.size
          box = image.crop((int(part[0] * w), int(part[1] * h), int(part[2] * w), int(part[3] * h)))
          counts = box.getcolors(box.width * box.height)
          assert counts is not None
          return image, len(counts), ImageStat.Stat(box.convert("L")).mean[0]

      with subtest("the wallpaper stands behind the workspaces"):
          wait_for_surface("wallpaper")
          # It fades in once it's read.
          time.sleep(1)
          _, colours, _ = colours_and_light("workspace")
          t.assertGreater(colours, 1000, "the workspace shows a flat colour, not the picture")

      with subtest("the overview shows the time and status, and only the overview"):
          machine.fail(in_session("niri msg layers | grep -qw status"))
          as_account("niri msg action open-overview")
          wait_for_surface("status")
          time.sleep(1)
          drawn = foreground_on_the_left("overview")
          as_account("niri msg action close-overview")
          t.assertGreater(drawn, 100)
          machine.wait_until_fails(in_session("niri msg layers | grep -qw status"), timeout=10)

      with subtest("a notification appears"):
          as_account("notify-send 'Build finished' 'the laptop is ready'")
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

      # A photograph of the media keys' chords, whose marks lie past U+FFFF,
      # and Escape leaving.
      with subtest("the launcher shows the volume's chords, and Escape closes it"):
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.send_chars("volume")
          machine.sleep(1)
          machine.screenshot("launcher-volume")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

      with subtest("the launcher finds and opens kitty, and learns that kitty was meant"):
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          # niri lists the layer a frame before it holds the keyboard, and a
          # letter typed in that frame goes to the window underneath.
          time.sleep(0.5)
          machine.send_chars("kitty")
          machine.sleep(1)
          machine.screenshot("launcher")
          machine.send_key("ret")
          machine.wait_until_succeeds(in_session("niri msg windows | grep -i kitty"), timeout=30)
          # What was picked for what was typed is kept for next time.
          picked = "'\"kitty\":{\"program:kitty\"'"
          machine.wait_until_succeeds(
              in_session(f"grep -rqsF {picked} ~/.local/state/quickshell"), timeout=10
          )

      # The border is all that marks the focused window, and niri leaves
      # borders off unless its config turns them on.
      with subtest("the focused window is framed in the accent"):
          machine.wait_until_succeeds(in_session("niri msg focused-window | grep -qi kitty"), timeout=30)
          window = json.loads(as_account("niri msg --json focused-window"))
          as_account(f"niri msg action move-window-to-floating --id {window['id']}")
          time.sleep(1)
          machine.screenshot("focused")
          left, top, _, height = drawn_at(json.loads(as_account("niri msg --json focused-window")))
          image = Image.open(os.path.join(machine.out_dir, "focused.png")).convert("RGB")
          beside = "#%02x%02x%02x" % image.getpixel((left - 1, top + height // 2))
          as_account(f"niri msg action move-window-to-tiling --id {window['id']}")
          t.assertEqual(beside, "${roles.accent}".lower())

      # niri opens a window in a column of its own, and the shell folds it
      # in just below the focused one.
      with subtest("a terminal opens below the focused window"):
          anchor = json.loads(as_account("niri msg --json focused-window"))
          as_account("qs ipc call grid below kitty")
          kitties = "niri msg --json windows | grep -o '\"app_id\":\"kitty\"' | wc -l"
          machine.wait_until_succeeds(in_session(f"{kitties} | grep -qx 2"), timeout=30)

          def placed(_):
              windows = json.loads(as_account("niri msg --json windows"))
              at = {w["id"]: w["layout"]["pos_in_scrolling_layout"] for w in windows}
              new = next(w["id"] for w in windows if w["id"] != anchor["id"])
              here, there = at[anchor["id"]], at[new]
              return there[0] == here[0] and there[1] == here[1] + 1

          retry(placed, timeout_seconds=15)

      # The launcher finds a window by its title, and Return goes to it.
      with subtest("the launcher switches to a window by its title"):
          as_account("niri msg action spawn -- kitty --title 'Notes for Tuesday'")
          machine.wait_until_succeeds(
              in_session("niri msg windows | grep -q 'Notes for Tuesday'"), timeout=30
          )
          as_account("niri msg action focus-window-previous")
          machine.wait_until_fails(
              in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"), timeout=10
          )
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.send_chars("tuesday")
          machine.sleep(1)
          machine.screenshot("launcher-window")
          machine.send_key("ret")
          machine.wait_until_succeeds(
              in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"), timeout=10
          )

      # The right arrow steps into what can be done to a window, and a verb
      # that needs a target asks for it in the same field.
      with subtest("the launcher moves a window and copies its title"):
          notes = json.loads(as_account("niri msg --json focused-window"))

          def into_notes(verb):
              as_account("qs ipc call launcher toggle")
              wait_for_surface("launcher")
              time.sleep(0.5)
              machine.send_chars("tuesday")
              time.sleep(0.5)
              machine.send_key("right")
              time.sleep(0.5)
              machine.send_chars(verb)
              time.sleep(0.5)

          into_notes("copy")
          machine.screenshot("launcher-verbs")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertEqual(as_account("wl-paste --no-newline"), "Notes for Tuesday")

          into_notes("move")
          machine.send_key("ret")
          time.sleep(0.5)
          machine.screenshot("launcher-target")
          machine.send_key("ret")

          def moved(_):
              windows = json.loads(as_account("niri msg --json windows"))
              now = next(w for w in windows if w["id"] == notes["id"])
              return now["workspace_id"] != notes["workspace_id"]

          retry(moved, timeout_seconds=10)

      with subtest("the launcher answers what's asked, and copies the answer"):

          def answer(typed):
              as_account("qs ipc call launcher toggle")
              wait_for_surface("launcher")
              time.sleep(0.5)
              machine.send_chars(typed)
              time.sleep(1)

          def copied(typed, expected):
              answer(typed)
              machine.send_key("ret")
              closed = in_session("niri msg layers | grep -qw launcher")
              machine.wait_until_fails(closed, timeout=10)
              t.assertEqual(as_account("wl-paste --no-newline"), expected)

          copied("12*7", "84")
          # The vendor comes from the list built from hwdata, so this proves it's there.
          copied("3c:22:fb:12:34:56", "Apple, Inc.")
          answer("10.0.0.0/22")
          machine.screenshot("launcher-answer")
          machine.send_key("esc")
          # With no network, what has to ask one says it couldn't, and nothing breaks.
          for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "ip"]:
              answer(typed)
              machine.send_key("esc")
          time.sleep(11)
          answer("weather tokyo")
          machine.screenshot("launcher-offline")
          machine.send_key("esc")
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          assert "ERROR" not in log and "Error:" not in log, "an answer raised an error"

      with subtest("the launcher finds emoji as one row, and steps into the rest"):
          copied("tada", "\U0001F389")
          answer("smiling")
          machine.screenshot("launcher-emoji")
          machine.send_key("right")
          time.sleep(0.5)
          machine.send_chars(" cat")
          time.sleep(0.5)
          machine.screenshot("launcher-emoji-all")
          machine.send_key("ret")
          closed = in_session("niri msg layers | grep -qw launcher")
          machine.wait_until_fails(closed, timeout=10)
          # The grinning cat with smiling eyes, first among the smiling cats.
          t.assertEqual(as_account("wl-paste --no-newline"), "\U0001F638")

      with subtest("Super+Shift+/ shows every layer's shortcuts, and runs the desktop's"):
          windows = len(json.loads(as_account("niri msg --json windows")))
          machine.send_key("meta_l-shift-slash")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.send_chars("paste")
          time.sleep(0.5)
          machine.screenshot("launcher-shortcuts")
          for _ in "paste":
              machine.send_key("backspace")
          machine.send_chars("open a terminal")
          time.sleep(0.5)
          machine.send_key("ret")

          def opened(_):
              return len(json.loads(as_account("niri msg --json windows"))) > windows

          retry(opened, timeout_seconds=15)

      with subtest("Super+V finds what was copied earlier, and copies it again"):
          keeping = as_account("systemctl --user is-active clipboard-history")
          t.assertEqual(keeping.strip(), "active")
          as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")
          time.sleep(0.5)
          as_account("printf 'the second copy\\nover two lines' | wl-copy > /dev/null 2>&1")
          time.sleep(0.5)
          machine.send_key("meta_l-v")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.screenshot("launcher-copies-all")
          machine.send_chars("tues")
          time.sleep(0.5)
          machine.screenshot("launcher-copies")
          machine.send_key("ret")

          def copied_again(_):
              return as_account("wl-paste --no-newline") == "Notes for Tuesday"

          retry(copied_again, timeout_seconds=10)

      with subtest("the launcher's home shows each collection, and opens it"):
          windows = len(json.loads(as_account("niri msg --json windows")))
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.screenshot("launcher-home")
          # The last row at home is the last collection, the shortcuts.
          machine.send_key("pgdn")
          machine.send_key("ret")
          time.sleep(0.5)
          # Opening a collection keeps the launcher open, where a program
          # picked instead would close it.
          wait_for_surface("launcher")
          machine.send_chars("open a terminal")
          time.sleep(0.5)
          machine.send_key("ret")

          def another(_):
              return len(json.loads(as_account("niri msg --json windows"))) > windows

          retry(another, timeout_seconds=15)

      with subtest("a notice that came and went is found again in the launcher"):
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          # Its row at home counts the one missed earlier, since it left by itself.
          machine.screenshot("launcher-home-missed")
          machine.send_chars("build fin")
          time.sleep(0.5)
          machine.screenshot("launcher-notices")
          # Nothing sent it that can be gone to, so Return copies what it said.
          machine.send_key("ret")

          def told_again(_):
              said = as_account("wl-paste --no-newline")
              return said == "Build finished\nthe laptop is ready"

          retry(told_again, timeout_seconds=10)
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # Super+N opens all of them, which counts them seen.
          machine.send_key("meta_l-n")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.screenshot("launcher-notices-all")
          machine.send_key("esc")

      with subtest("the launcher keeps the machine awake, lid shut too, then lets it sleep"):
          held = "systemd-inhibit --list --no-pager | grep Caffeinate"
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.send_chars("keep awake")
          time.sleep(0.5)
          machine.screenshot("launcher-keep-awake")
          machine.send_key("ret")
          machine.wait_until_succeeds(f"{held} | grep -q sleep:idle", timeout=10)
          machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
          # The overview's status says so while it holds, and so does home.
          as_account("niri msg action open-overview")
          wait_for_surface("status")
          time.sleep(1)
          machine.screenshot("overview-awake")
          as_account("niri msg action close-overview")
          # Without power the lid is let go, and taken again when it's back.
          # The kernel's stand-in has two supplies, mains and USB.
          def power(state):
              parameters = "/sys/module/test_power/parameters"
              for supply in ["ac", "usb"]:
                  machine.succeed(f"echo {state} > {parameters}/{supply}_online")

          power("off")
          machine.wait_until_fails(f"{held} | grep -q handle-lid-switch", timeout=10)
          machine.succeed(f"{held} | grep -q sleep:idle")
          power("on")
          machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
          as_account("qs ipc call launcher toggle")
          wait_for_surface("launcher")
          time.sleep(0.5)
          machine.screenshot("launcher-home-awake")
          machine.send_chars("let it sleep")
          time.sleep(0.5)
          machine.send_key("ret")
          machine.wait_until_fails(held, timeout=10)

      with subtest("the lock screen takes only the right password"):
          as_account("qs ipc call lock lock")
          machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q true"), timeout=30)
          machine.sleep(1)
          machine.send_chars("wrong-password\n")
          machine.sleep(5)
          as_account("qs ipc call lock isLocked | grep -q true")
          _, colours, _ = colours_and_light("locked")
          t.assertGreater(colours, 1000, "the lock screen shows a flat colour, not its picture")
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

      with subtest("programs in a terminal reach the clipboard"):
          # wl-copy stays behind to serve what it copied, so it lets go of the
          # output the test reads.
          as_account("echo -n 'from a terminal' | wl-copy > /dev/null 2>&1")
          t.assertEqual(as_account("wl-paste --no-newline"), "from a terminal")

      with subtest("monospace is the theme's font"):
          family = as_account("${pkgs.fontconfig.bin}/bin/fc-match -f '%{family}' monospace")
          t.assertIn("${font.mono.family}", family)

      with subtest("GTK programs draw in the theme's colours"):
          zenity = "${pkgs.zenity}/bin/zenity --text-info --title=libadwaita --filename=${text}"
          # Here niri runs in Cage's system service, and the portal refuses the
          # programs it starts ("Unable to open /proc/<pid>/root"), so zenity
          # runs under the account's user manager, where niri's own unit puts
          # them on the laptop.
          as_account(f"systemd-run --user --collect {zenity}")
          t.assertEqual(commonest_colour_in("libadwaita", "libadwaita"), "${roles.sunken}".lower())
          # The file chooser Chrome and Slack open, which the GTK portal draws
          # with GTK 3, asked for the way they ask.
          as_account(
              "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
              " org.freedesktop.portal.FileChooser OpenFile 'ssa{sv}' \"\" Open 0"
          )
          t.assertEqual(commonest_colour_in("Open", "file-chooser"), "${roles.sunken}".lower())

      machine.screenshot("desktop")
    '';
}
