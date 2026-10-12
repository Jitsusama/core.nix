# Starts the graphical role's desktop in a virtual machine and uses it the
# way Joel does: the wallpaper stands behind the workspaces, the overview
# shows the time and status, a notification appears, the volume and
# brightness bars come and go as those change, the launcher finds and opens
# kitty and remembers it was picked, a terminal opens below the focused
# window, the launcher switches to a window by its title, the lock screen
# refuses a wrong password and takes the right one, polkit's prompt lets the
# account start a system service, what's playing shows in the margin and a
# media key shows what it is, a call shows in the margin while it lasts, the
# clipboard takes and gives text, and GTK programs and monospaced text are
# in the theme. niri says which surfaces it drew, photographs show their
# colours, to the test and to a person looking, and
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
        # Two stand-in Wi-Fi radios: hostapd makes the first an access point
        # and NetworkManager joins it from the second, as the laptop's would.
        "mac80211_hwsim"
      ];
      services.hostapd = {
        enable = true;
        radios.wlan0 = {
          band = "2g";
          channel = 6;
          countryCode = "CA";
          networks.wlan0 = {
            ssid = "Lantern";
            authentication = {
              mode = "wpa2-sha1";
              wpaPassword = "glowing in the dark";
            };
          };
        };
      };
      networking.networkmanager.unmanaged = [ "wlan0" ];
      # The access point hands out addresses, as a real network does, since
      # NetworkManager counts a network joined only once it has one.
      networking.interfaces.wlan0.ipv4.addresses = [
        {
          address = "10.42.0.1";
          prefixLength = 24;
        }
      ];
      services.dnsmasq = {
        enable = true;
        resolveLocalQueries = false;
        settings = {
          port = 0;
          interface = "wlan0";
          bind-dynamic = true;
          dhcp-range = "10.42.0.10,10.42.0.50,1h";
        };
      };
      networking.firewall.interfaces.wlan0.allowedUDPPorts = [ 67 ];
      # A VM turns off wpa_supplicant, which NetworkManager joins networks
      # through, since it usually has no radio; this one has two.
      networking.wireless.enable = lib.mkOverride 0 true;
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
        # A second output, for the launcher's Sound to move to, ranked below
        # the speakers so they're the one in use from the start.
        {
          factory = "adapter";
          args = {
            "factory.name" = "support.null-audio-sink";
            "node.name" = "stand-in-headphones";
            "media.class" = "Audio/Sink";
            "priority.session" = 1;
            "audio.position" = [
              "FL"
              "FR"
            ];
          };
        }
      ];
      # A microphone of the class a real one has, which Quickshell needs to see
      # its audio: a loopback whose far side is a source, as Pipewire's guide
      # makes one, since a null sink's ports face the wrong way for that class.
      services.pipewire.extraConfig.pipewire."91-stand-in-microphone"."context.modules" = [
        {
          name = "libpipewire-module-loopback";
          args = {
            "capture.props" = {
              "node.name" = "stand-in-microphone-input";
              # Two channels, as a laptop's microphones are: Quickshell reads no
              # volume from a loopback whose map and volumes differ in size.
              "audio.position" = [
                "FL"
                "FR"
              ];
              "node.passive" = true;
            };
            "playback.props" = {
              "node.name" = "stand-in-microphone";
              "media.class" = "Audio/Source";
              "audio.position" = [
                "FL"
                "FR"
              ];
            };
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
        # Polkit asks a member of wheel for their own password. It lets the
        # signed-in account change networks as an active local session, which
        # the shell nested in Cage isn't seen as, so NetworkManager's group
        # gives the account the same.
        extraGroups = [
          "wheel"
          "networkmanager"
        ];
        initialPassword = secret;
      };
      home-manager.users.${account}.home.stateVersion = "26.05";
      environment.systemPackages = [
        pkgs.libnotify
        # A player that tells the desktop what it plays, as Spotify does.
        (pkgs.mpv.override { scripts = [ pkgs.mpvScripts.mpris ]; })
      ];

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

      # A silent track with a title, an artist and a cover, for a player to
      # play through no sound card at all.
      track = pkgs.runCommandLocal "track.mp3" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
        ffmpeg -loglevel error -f lavfi -i color=c=0x${pkgs.lib.removePrefix "#" roles.program}:s=300x300 \
          -frames:v 1 cover.png
        ffmpeg -loglevel error -f lavfi -i anullsrc=r=44100:cl=stereo -i cover.png -t 60 \
          -map 0 -map 1 -c:a libmp3lame -c:v png -disposition:v attached_pic \
          -metadata title="Bless The Child" -metadata artist=Nightwish $out
      '';
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

      # What the launcher shows, from its showing call: whether it holds the
      # keys, what's typed and held, and the row Return takes. Under load the
      # launcher lags the keys sent to it, so each step waits on this rather
      # than on a guess at how long it takes.
      def showing():
          return json.loads(as_account("qs ipc call launcher showing"))

      def launcher_until(test, timeout=30):
          retry(lambda _: test(showing()), timeout_seconds=timeout)

      # niri lists the layer a frame before it gives it the keys, and a letter
      # typed in that frame goes to the window underneath.
      def launcher_ready():
          wait_for_surface("launcher")
          launcher_until(lambda shown: shown["open"] and shown["keys"])

      # Types into the launcher, and waits until it holds every letter with the
      # row Return takes settled, since an answer still being looked up closes
      # the launcher without copying. A row that comes from something found
      # later, such as a window or a copy, can be named to wait for as well.
      def type_in(text, selecting=""):
          machine.send_chars(text)

          def holds(shown):
              row = shown["selected"]
              name = row["name"].lower() if row else ""
              settled = row is None or row["settled"]
              return shown["typed"].endswith(text) and settled and selecting.lower() in name

          launcher_until(holds)

      # A key that steps into the selected row, which the launcher then holds:
      # the right arrow into a thing's verbs, or Return into what a verb asks.
      def step_in(key="right"):
          before = len(showing()["held"])
          machine.send_key(key)
          launcher_until(lambda shown: len(shown["held"]) > before)

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

          # It fades in once it's read, which takes longer on a slower machine.
          def painted(last):
              _, colours, _ = colours_and_light("workspace")
              if last:
                  t.assertGreater(
                      colours, 1000, "the workspace shows a flat colour, not the picture"
                  )
              return colours > 1000

          retry(painted, timeout_seconds=30)

      with subtest("the overview shows the time and status, and only the overview"):
          machine.fail(in_session("niri msg layers | grep -qw status"))
          as_account("niri msg action open-overview")
          wait_for_surface("status")

          def shown(last):
              drawn = foreground_on_the_left("overview")
              if last:
                  t.assertGreater(drawn, 100, "the overview shows no status")
              return drawn > 100

          retry(shown, timeout_seconds=30)
          as_account("niri msg action close-overview")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw status"), timeout=10)

      with subtest("a notification appears"):
          as_account("notify-send 'Build finished' 'the laptop is ready'")
          wait_for_surface("notifications")
          machine.sleep(1)
          machine.screenshot("notification")

      # Quickshell hears a node change only once it has bound the node's audio,
      # a moment after WirePlumber names it the default, so a change is made
      # again, each time a different one, until its meter answers.
      def answered(namespace, change):
          def tried(_):
              as_account(change)
              for _ in range(4):
                  time.sleep(0.5)
                  shown = in_session(f"niri msg layers | grep -q {namespace}")
                  if machine.execute(shown)[0] == 0:
                      return True
              return False

          retry(tried, timeout_seconds=30)

      with subtest("the volume shows when it changes, then goes"):
          # WirePlumber picks the defaults a moment after Pipewire lists the nodes.
          defaults = "wpctl inspect @DEFAULT_AUDIO_SINK@ && wpctl inspect @DEFAULT_AUDIO_SOURCE@"
          machine.wait_until_succeeds(in_session(defaults), timeout=30)
          answered("volume", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 10%-")
          # niri lists a surface before it has drawn its first frame.
          time.sleep(0.5)
          machine.screenshot("volume")
          machine.wait_until_fails(in_session("niri msg layers | grep -q volume"), timeout=10)

      # The microphone's key mutes it through the same command, which niri runs.
      with subtest("the microphone shows when it mutes, then goes"):
          toggle = "niri msg action spawn -- wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"
          answered("microphone", toggle)
          time.sleep(0.5)
          machine.screenshot("microphone")
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
          as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")

      # niri runs the brightness keys' command inside Joel's session, which is
      # the one logind lets change a backlight, so the test has niri run it too.
      with subtest("the brightness shows when the keys' command changes it, then goes"):
          as_account("niri msg action spawn -- brightnessctl --class=backlight set 80%")
          machine.wait_until_succeeds("grep -qx 80 /sys/class/backlight/stand-in/brightness", timeout=10)
          wait_for_surface("brightness")
          time.sleep(0.5)
          machine.screenshot("brightness")
          machine.wait_until_fails(in_session("niri msg layers | grep -q brightness"), timeout=10)

      with subtest("the charge shows when power goes and comes back, then goes"):
          # The kernel's stand-in has two supplies, mains and USB, and UPower
          # counts the machine on battery only with both off.
          def power(state):
              parameters = "/sys/module/test_power/parameters"
              for supply in ["ac", "usb"]:
                  machine.succeed(f"echo {state} > {parameters}/{supply}_online")

          for state in ["off", "on"]:
              power(state)
              wait_for_surface("power")
              time.sleep(0.5)
              machine.screenshot(f"power-{state}")
              machine.wait_until_fails(in_session("niri msg layers | grep -q power"), timeout=10)

      # A photograph of the media keys' chords, whose marks lie past U+FFFF,
      # and Escape leaving.
      with subtest("the launcher shows the volume's chords, and Escape closes it"):
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("volume")
          machine.screenshot("launcher-volume")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

      with subtest("the launcher finds and opens kitty, and learns that kitty was meant"):
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("kitty", selecting="kitty")
          machine.screenshot("launcher")
          machine.send_key("ret")
          # The first window opened can take as long as the first GTK one.
          machine.wait_until_succeeds(in_session("niri msg windows | grep -i kitty"), timeout=120)
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
          launcher_ready()
          type_in("tuesday", selecting="Notes for Tuesday")
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
              launcher_ready()
              type_in("tuesday", selecting="Notes for Tuesday")
              step_in()
              type_in(verb)

          into_notes("copy")
          machine.screenshot("launcher-verbs")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertEqual(as_account("wl-paste --no-newline"), "Notes for Tuesday")

          into_notes("move")
          step_in("ret")
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
              launcher_ready()
              type_in(typed)

          def copied(typed, expected):
              answer(typed)
              machine.send_key("ret")
              closed = in_session("niri msg layers | grep -qw launcher")
              machine.wait_until_fails(closed, timeout=10)
              t.assertEqual(as_account("wl-paste --no-newline"), expected)

          copied("12*7", "84")
          # The vendor comes from the list built from hwdata, so this proves it's there.
          copied("3c:22:fb:12:34:56", "Apple, Inc.")
          copied("5 km in mi", "3.10686")
          answer("350 f in c")
          machine.screenshot("launcher-units")
          machine.send_key("esc")
          # The zones come from the list built from tzdata, so this proves it's
          # there. The minute may turn while it's asked, so either will do.
          before = machine.succeed("date -u +%H:%M").strip()
          answer("time in utc")
          machine.screenshot("launcher-time")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          after = machine.succeed("date -u +%H:%M").strip()
          t.assertIn(as_account("wl-paste --no-newline"), [before, after])
          # A theme's colour is found by its role, as the theme exports it.
          theme = json.loads(as_account("cat ~/.config/jitsusama/theme.json"))
          copied("colour accent", theme["roles"]["accent"].lower())
          answer("#f53")
          machine.screenshot("launcher-colour")
          machine.send_key("esc")
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
          step_in()
          type_in(" cat")
          machine.screenshot("launcher-emoji-all")
          machine.send_key("ret")
          closed = in_session("niri msg layers | grep -qw launcher")
          machine.wait_until_fails(closed, timeout=10)
          # The grinning cat with smiling eyes, first among the smiling cats.
          t.assertEqual(as_account("wl-paste --no-newline"), "\U0001F638")
          # Super+Ctrl+Space opens the emoji straight away, so "kitty" is a cat
          # there, where the main search would open the terminal.
          machine.send_key("meta_l-ctrl-spc")
          launcher_ready()
          type_in("kitty")
          machine.send_key("ret")
          machine.wait_until_fails(closed, timeout=10)
          t.assertEqual(as_account("wl-paste --no-newline"), "\U0001F431")

      with subtest("Super+Shift+/ shows every layer's shortcuts, and runs the desktop's"):
          windows = len(json.loads(as_account("niri msg --json windows")))
          machine.send_key("meta_l-shift-slash")
          launcher_ready()
          type_in("paste")
          machine.screenshot("launcher-shortcuts")
          for _ in "paste":
              machine.send_key("backspace")
          type_in("open a terminal")
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
          launcher_ready()
          machine.screenshot("launcher-copies-all")
          type_in("tues", selecting="Notes for Tuesday")
          machine.screenshot("launcher-copies")
          machine.send_key("ret")

          def copied_again(_):
              return as_account("wl-paste --no-newline") == "Notes for Tuesday"

          retry(copied_again, timeout_seconds=10)

      with subtest("the launcher's home shows each collection, and opens it"):
          windows = len(json.loads(as_account("niri msg --json windows")))
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          machine.screenshot("launcher-home")
          # The last row at home is the last collection, the shortcuts; a page
          # down stops there, so two reach it however long home grows.
          machine.send_key("pgdn")
          machine.send_key("pgdn")
          # Opening a collection keeps the launcher open, where a program
          # picked instead would close it.
          step_in("ret")
          type_in("open a terminal")
          machine.send_key("ret")

          def another(_):
              return len(json.loads(as_account("niri msg --json windows"))) > windows

          retry(another, timeout_seconds=15)

      with subtest("the launcher's Sound moves sound to another output, and back"):
          for output in ["stand-in-headphones", "stand-in-speakers"]:
              as_account("qs ipc call launcher collection sound")
              launcher_ready()
              machine.screenshot(f"launcher-sound-{output}")
              kind = output.removeprefix("stand-in-")
              type_in(kind, selecting=kind)
              machine.send_key("ret")
              used = f"wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q 'node.name = \"{output}\"'"
              machine.wait_until_succeeds(in_session(used), timeout=10)
              machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)

      with subtest("the launcher sets the power profile"):
          for typed, profile in [("power saver", "power-saver"), ("balanced power", "balanced")]:
              as_account("qs ipc call launcher toggle")
              launcher_ready()
              type_in(typed)
              machine.send_key("ret")
              machine.wait_until_succeeds(f"powerprofilesctl get | grep -qx {profile}", timeout=10)
              machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)

      # The right arrow on one of a collection's members offers its own
      # actions, as on anything at the top.
      with subtest("the launcher's Displays scales a display by keys, and back"):

          def scale():
              outputs = json.loads(as_account("niri msg --json outputs"))
              return next(iter(outputs.values()))["logical"]["scale"]

          for wanted in [1.5, scale()]:
              as_account("qs ipc call launcher collection displays")
              launcher_ready()
              step_in()
              machine.screenshot("launcher-displays")
              type_in(f"scale {round(wanted * 100)}")
              machine.send_key("ret")
              retry(lambda _, wanted=wanted: scale() == wanted, timeout_seconds=15)
              machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)

      with subtest("a Wi-Fi network is joined from the launcher, a new one asking its password"):
          machine.wait_for_unit("hostapd.service")
          machine.wait_until_succeeds(
              "nmcli -t -f SSID device wifi list ifname wlan1 | grep -qx Lantern", timeout=60
          )
          as_account("qs ipc call launcher collection wifi")
          launcher_ready()
          machine.screenshot("launcher-wifi")
          type_in("lant", selecting="Lantern")
          # Never joined, it asks for its password, which a wrong one doesn't
          # satisfy; the launcher stays to say so.
          step_in("ret")
          type_in("not the password")
          machine.screenshot("launcher-wifi-password")
          machine.send_key("ret")
          machine.wait_until_succeeds(
              "journalctl -u NetworkManager"
              " | grep -q \"Activation: failed for connection 'Lantern'\"",
              timeout=60,
          )
          time.sleep(1)
          machine.screenshot("launcher-wifi-refused")
          machine.succeed(in_session("niri msg layers | grep -q launcher"))
          type_in("glowing in the dark")
          machine.send_key("ret")
          # Joined means an address too, which is when NetworkManager says so.
          machine.wait_until_succeeds(
              "nmcli -g GENERAL.CONNECTION,GENERAL.STATE device show wlan1"
              " | tr '\\n' ' ' | grep -q '^Lantern 100 '",
              timeout=60,
          )
          # Once it's joined, the launcher's done.
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # Turning Wi-Fi off is an action like any.
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("wifi off")
          machine.send_key("ret")
          machine.wait_until_succeeds("nmcli radio wifi | grep -qx disabled", timeout=15)
          machine.succeed("nmcli radio wifi on")

      with subtest("a notice that came and went is found again in the launcher"):
          # One its sender says not to keep isn't, though it's the newest match.
          as_account("notify-send --transient 'Build finished twice' 'not kept'")
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          # Its row at home counts the one missed earlier, since it left by itself.
          machine.screenshot("launcher-home-missed")
          type_in("build fin", selecting="the laptop is ready")
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
          launcher_ready()
          machine.screenshot("launcher-notices-all")
          machine.send_key("esc")

      with subtest("a notice's action is done by the keys, and one not acted on stays unread"):
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          as_account("notify-send 'Lunch' 'at noon'")
          # notify-send waits for the answer, so it runs on its own.
          as_account(
              "notify-send --action=play='Play it' 'Recording saved' 'x.mp4'"
              " > /tmp/answer 2>&1 < /dev/null &"
          )
          wait_for_surface("notifications")
          time.sleep(0.5)
          machine.screenshot("notice-with-action")
          # Qt only warns when a layout never settles, which left the
          # card's words piled at its top.
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          assert "polish() loop" not in log, "a notice's layout never settles"
          # Super+N opens on what it offers, so Return does the first.
          machine.send_key("meta_l-n")
          launcher_ready()
          launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Play it")
          machine.screenshot("launcher-notice-actions")
          machine.send_key("ret")
          machine.wait_until_succeeds("grep -qx play /tmp/answer", timeout=15)
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # The history is kept on disk, the one acted on read and the one
          # that only arrived still unread.
          kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
          machine.wait_until_succeeds(
              in_session(kept + " | grep -q '\"summary\":\"Recording saved\"[^}]*\"read\":true'"),
              timeout=10,
          )
          as_account(kept + " | grep -q '\"summary\":\"Lunch\"[^}]*\"read\":false'")

      with subtest("the launcher keeps the machine awake, lid shut too, then lets it sleep"):
          held = "systemd-inhibit --list --no-pager | grep Caffeinate"
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("keep awake")
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
          power("off")
          machine.wait_until_fails(f"{held} | grep -q handle-lid-switch", timeout=10)
          machine.succeed(f"{held} | grep -q sleep:idle")
          power("on")
          machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          machine.screenshot("launcher-home-awake")
          type_in("let it sleep")
          machine.send_key("ret")
          machine.wait_until_fails(held, timeout=10)

      # The margin is there only while something's on, and every part of it
      # is reached by the keys: Super+O brings out the drawer with what wants
      # Joel selected, the arrows move, Return does the action.
      margin = "niri msg layers | grep -q margin"
      notices = "niri msg layers | grep -q notifications"

      with subtest("Super+O shows Keep Awake in the margin, and Return lets it sleep"):
          # Keep Awake was just let go, and the margin leaves as it fades.
          machine.wait_until_fails(in_session(margin), timeout=10)
          as_account("systemctl --user start caffeinate.service")
          wait_for_surface("margin")
          time.sleep(0.5)
          machine.screenshot("margin-awake")
          machine.send_key("meta_l-o")
          # The panel takes the keys, and a click anywhere else puts it away.
          wait_for_surface("outside")
          time.sleep(0.5)
          machine.screenshot("margin-drawer")
          machine.send_key("ret")
          machine.wait_until_fails(held, timeout=10)
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("do not disturb holds a notice, shows it held, and ends from the margin"):
          machine.wait_until_fails(in_session(notices), timeout=30)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("do not disturb")
          machine.send_key("ret")
          wait_for_surface("margin")
          as_account("notify-send 'Held back' 'until it ends'")
          time.sleep(2)
          machine.fail(in_session(notices))
          # Its panel offers ending it, an hour more, then what's held.
          machine.send_key("meta_l-o")
          wait_for_surface("outside")
          time.sleep(0.5)
          machine.screenshot("margin-quiet")
          machine.send_key("down")
          machine.send_key("down")
          machine.send_key("ret")
          launcher_ready()
          machine.screenshot("launcher-held")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          machine.send_key("meta_l-o")
          wait_for_surface("outside")
          time.sleep(0.5)
          machine.send_key("ret")
          machine.wait_until_fails(in_session(margin), timeout=10)
          as_account("test ! -s $XDG_RUNTIME_DIR/quiet")

      with subtest("a missed notice lights its pull, and Return shows the notices"):
          as_account("notify-send --expire-time=1000 'Peter' 'are you there?'")
          wait_for_surface("notifications")
          # On screen it isn't waiting yet; once it goes by itself, it is.
          machine.wait_until_fails(in_session(notices), timeout=15)
          wait_for_surface("margin")
          time.sleep(0.5)
          machine.screenshot("margin-waiting")
          machine.send_key("meta_l-o")
          wait_for_surface("outside")
          time.sleep(0.5)
          machine.send_key("ret")
          launcher_ready()
          machine.screenshot("launcher-from-margin")
          machine.send_key("esc")
          # Opening them counted them seen, so nothing's waiting.
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("the microphone muted shows in the margin, and Return unmutes it"):
          as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
          wait_for_surface("margin")
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
          machine.send_key("meta_l-o")
          wait_for_surface("outside")
          time.sleep(0.5)
          machine.screenshot("margin-muted")
          machine.send_key("ret")
          muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
          machine.wait_until_fails(in_session(muted), timeout=10)
          machine.wait_until_fails(in_session(margin), timeout=10)

      # Any player that tells the desktop what it plays shows in the margin
      # while it plays, and the media keys' command shows what it is.
      with subtest("what's playing shows in the margin, and a media key shows what it is"):
          machine.wait_until_fails(in_session(margin), timeout=10)
          as_account("systemd-run --user --unit=player mpv --ao=null --no-video --loop ${track}")
          wait_for_surface("margin")
          time.sleep(0.5)
          machine.screenshot("margin-playing")
          # The play key pauses it, and the card says what was paused.
          as_account("qs ipc call music toggle")
          wait_for_surface("notifications")
          time.sleep(0.5)
          machine.screenshot("music-card")
          # Paused, nothing's playing, so the margin goes; the card goes by itself.
          machine.wait_until_fails(in_session(margin), timeout=10)
          machine.wait_until_fails(in_session(notices), timeout=15)
          as_account("systemctl --user stop player")

      with subtest("a Tuple call shows in the margin while it lasts"):
          # Only a call writes these lines, so they're written here, with
          # the daemon stopped, since it rewrites its log as it starts.
          as_account("systemctl --user stop tuple")
          written = "~/.local/share/tuple/0/log.txt"

          def said(message):
              line = f"$(date -u +%H:%M:%S.000)|1|1|info|{message}"
              as_account(f"mkdir -p $(dirname {written}) && echo \"{line}\" >> {written}")

          machine.wait_until_fails(in_session(margin), timeout=10)
          said("call connected")
          wait_for_surface("margin")
          time.sleep(0.5)
          machine.screenshot("margin-call")
          said("Caller.cpp:985|invalidating call")
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("caps lock shows in the margin while it's on"):
          capitals = "/run/keyboard-lights/capslock"
          machine.send_key("caps_lock")
          machine.wait_until_succeeds(f"grep -qx 1 {capitals}", timeout=10)
          wait_for_surface("margin")
          machine.screenshot("margin-capitals")
          machine.send_key("caps_lock")
          machine.wait_until_succeeds(f"grep -qx 0 {capitals}", timeout=10)
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("caps lock still shows after its watcher restarts"):
          # Quickshell keeps running, so it must still hear the file.
          machine.succeed("systemctl restart keyboard-lights")
          machine.wait_for_unit("keyboard-lights.service")
          machine.send_key("caps_lock")
          machine.wait_until_succeeds(f"grep -qx 1 {capitals}", timeout=10)
          wait_for_surface("margin")
          machine.send_key("caps_lock")
          machine.wait_until_fails(in_session(margin), timeout=10)

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

      # With no agent BlueZ refuses every pairing, even one asked for here.
      # BlueZ starts when a radio appears, which the VM has none of, so it's
      # started as a radio would, once the kernel's Bluetooth is loaded. It
      # comes last, since BlueZ starting midway disturbs the session's audio
      # for the subtests after it.
      with subtest("a Bluetooth agent answers for paired devices"):
          machine.succeed("modprobe bluetooth && systemctl start bluetooth.service")
          machine.wait_until_succeeds(
              "journalctl -u bluetooth-agent | grep -q 'Default agent requested'", timeout=30
          )
          machine.succeed("systemctl is-active bluetooth-agent")

      machine.screenshot("desktop")
    '';
}
