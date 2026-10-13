# Starts the graphical role's desktop in a virtual machine and uses it the
# way Joel does: the wallpaper stands behind the workspaces, the overview
# shows the time and status, a notification appears, the volume and
# brightness bars come and go as those change, the launcher finds and opens
# kitty and remembers it was picked, a terminal opens below the focused
# window, the launcher switches to a window by its title, the lock screen
# refuses a wrong password and takes the right one, polkit's prompt lets the
# account start a system service, what's playing shows in the margin and a
# media key shows what it is, a call shows in the margin while it lasts,
# people coming and going in it are said and Joel can leave from the last
# one's notice, a ringing call wants him and one not answered is kept as
# missed, a copied call's link can be joined until something else is
# copied, the clipboard takes and gives text, and GTK programs and
# monospaced text are in the theme. niri says which surfaces it drew,
# photographs show their colours, to the test and to a person looking, and
# the portal and fontconfig answer for the settings. Quickshell has to
# start without a single QML error.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";

  # Text views and file lists draw on the theme's dark background, which
  # nothing else on the screen uses, so a window that hasn't drawn can't pass
  # for one that has.
  text = pkgs.writeText "text" "A text view.";

  # What the shell has reported (modules/quickshell/Report.qml): waits until
  # it reports something starting with a prefix after a point in its
  # journal, or since the boot with no point given, then prints every
  # report since that point, each as "said", a tab and what was said, and
  # last the point it read to, as "cursor" and a tab. journalctl follows
  # the journal as it's written, so nothing is asked over and over. The
  # journal keeps Quickshell's lines as bytes, since it colours their
  # level, so they're read as plain text: JSON would give them as numbers.
  reports = pkgs.writeShellApplication {
    name = "reports";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.gnugrep
      pkgs.gnused
      pkgs.systemd
    ];
    text = ''
      since=$1 seconds=$2 prefix=$3
      journal=(journalctl --user --unit=quickshell --no-pager --output=cat --lines=all)
      if [ -n "$since" ]; then journal+=("--after-cursor=$since"); else journal+=(--boot); fi
      exec 3< <(exec "''${journal[@]}" --follow)
      follower=$!
      heard=0
      timeout "$seconds" grep -m1 -aqF "report[$prefix" <&3 || heard=$?
      kill "$follower" 2> /dev/null || true
      [ "$heard" = 0 ] || exit "$heard"
      "''${journal[@]}" --show-cursor \
        | sed -n -e 's/^-- cursor: /cursor\t/p' -e 's/.*report\[\(.*\)\]$/said\t\1/p'
    '';
  };
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
        # The workstation role's, which sets how the account's manager watches
        # pressure.
        self.nixosModules.memory
      ];

      # The VM has neither a backlight nor a sound card, so it gets one of each
      # that drives nothing, which the kernel and PipeWire report as they do
      # the laptop's.
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-backlight { })
        config.boot.kernelPackages.v4l2loopback
      ];
      boot.kernelModules = [
        "stand-in-backlight"
        # A stand-in camera, a loopback as the laptop's is, fed a test
        # picture as the laptop's relay feeds it the sensor's.
        "v4l2loopback"
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
      boot.extraModprobeConfig = ''
        options v4l2loopback video_nr=9 exclusive_caps=1 card_label=Camera
      '';
      systemd.services.stand-in-camera = {
        wantedBy = [ "multi-user.target" ];
        after = [ "systemd-modules-load.service" ];
        serviceConfig.ExecStart = lib.concatStringsSep " " [
          "${lib.getExe pkgs.ffmpeg-headless} -loglevel error -re"
          "-f lavfi -i testsrc2=size=640x480:rate=15 -pix_fmt yuyv422 -f v4l2 /dev/video9"
        ];
      };
      jitsusama.camera.device = "/dev/video9";

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
            # A null sink has no priority of its own, which ranks it below
            # any that has one.
            "priority.session" = 2;
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
      # It hears what the speakers play, so it gives something to record, as a
      # real one always does: unlinked, it would give nothing, and a recording
      # with sound waits for its first sound before it writes anything.
      services.pipewire.extraConfig.pipewire."91-stand-in-microphone"."context.modules" = [
        {
          name = "libpipewire-module-loopback";
          args = {
            "capture.props" = {
              "node.name" = "stand-in-microphone-input";
              "stream.capture.sink" = true;
              "target.object" = "stand-in-speakers";
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
          # The camera's, given to a seat's active session on the laptop.
          "video"
        ];
        initialPassword = secret;
      };
      home-manager.users.${account} = {
        home.stateVersion = "26.05";
        jitsusama.quickshell.reports = true;
        # What Quickshell hears from Pipewire about nodes and defaults, so a
        # change it missed can be told from one it was never sent.
        systemd.user.services.quickshell.Service.Environment = [
          "QT_LOGGING_RULES=${
            lib.concatMapStringsSep ";" (part: "quickshell.service.pipewire.${part}.info=true") [
              "node"
              "defaults"
            ]
          }"
        ];
      };
      environment.systemPackages = [
        reports
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
      import contextlib
      import json
      import os
      import re
      import shlex
      import tempfile
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

      # The shell reports what it does to its journal (Report.qml), so the test
      # waits on what happened, never on a guess at how long it takes. A point
      # in the journal is marked before the step that should bring a report,
      # and only what's said after it counts, so a report made before the wait
      # begins still ends it.
      def mark():
          said = as_account(
              "journalctl --user --unit=quickshell --lines=1 --show-cursor --output=cat"
          )
          return said.rsplit("-- cursor: ", 1)[1].strip()

      # Waits until something said after a point, or since the boot from "",
      # starts with the prefix and passes the test on the rest.
      # What it did say is printed when it never says what's wanted.
      def reported(prefix, test, since, timeout=30):
          deadline = time.monotonic() + timeout
          start = since

          def never():
              point = f"--after-cursor={shlex.quote(start)}" if start else "--boot"
              print(
                  as_account(f"journalctl --user --unit=quickshell --no-pager --output=cat {point}")
              )
              return f"the shell never reported {prefix!r} as wanted"

          while True:
              left = int(deadline - time.monotonic())
              assert left > 0, never()
              status, said = machine.execute(
                  in_session(f"reports {shlex.quote(since)} {left} {shlex.quote(prefix)}"),
                  timeout=left + 30,
              )
              assert status == 0, never()
              for line in said.splitlines():
                  kind, what = line.split("\t", 1)
                  if kind == "cursor":
                      since = what
                  elif what.startswith(prefix) and test(what[len(prefix) :]):
                      return

      # Waits, after what's done inside it, for the shell to say exactly this.
      @contextlib.contextmanager
      def told(what, timeout=30):
          since = mark()
          yield
          reported(what, lambda rest: rest == "", since, timeout)

      # A photograph of the screen once it holds still, for a person to look
      # at: niri composes a frame a moment after the shell hands it over, and
      # surfaces move as they come, so two pictures in a row have to match.
      # One that never holds still is kept as it is.
      def photograph(name, timeout=10):
          deadline = time.monotonic() + timeout
          with tempfile.TemporaryDirectory() as folder:
              shot = os.path.join(folder, "shot.ppm")
              last = None
              while time.monotonic() < deadline:
                  machine.send_monitor_command(f"screendump {shot}")
                  with open(shot, "rb") as taken:
                      now = taken.read()
                  if now == last:
                      break
                  last = now
          machine.screenshot(name)

      # What the launcher shows, from its showing call: whether it holds the
      # keys, what's typed and held, and the row Return takes. Under load the
      # launcher lags the keys sent to it, so each step waits on this.
      def showing():
          return json.loads(as_account("qs ipc call launcher showing"))

      # The launcher reports its state each time it changes, so a wait reads
      # it once and then hears every change.
      def launcher_until(test, timeout=30):
          since = mark()
          if not test(showing()):
              reported("showing ", lambda said: test(json.loads(said)), since, timeout)

      # niri lists the layer a frame before it gives it the keys, and a letter
      # typed in that frame goes to the window underneath.
      def launcher_ready():
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

      # What a reading of the screen settles on: read until it gives the
      # colour wanted, since niri moves a window to where it says it is over
      # a few frames, and the last reading if it never does. A reading niri
      # can't give yet, a window not yet floating, reads as nothing.
      def settles_on(wanted, read, timeout=15):
          readings = [None]

          def settled(_):
              try:
                  readings.append(read())
              except (KeyError, TypeError):
                  readings.append(None)
              return readings[-1] == wanted

          with contextlib.suppress(Exception):
              retry(settled, timeout_seconds=timeout)
          return readings[-1]

      # The commonest colour inside a window, a little in from its edges,
      # once it's the one wanted. niri says where a window is only once it
      # floats, so it floats first. The first GTK window has taken over 30
      # seconds to open on a CI runner building the kernel beside the test,
      # so this waits longer than the rest.
      def commonest_colour_in(title, picture, wanted):
          machine.wait_until_succeeds(
              in_session(f"niri msg --json windows | grep -qF '\"title\":\"{title}\"'"), timeout=120
          )
          as_account(f"niri msg action move-window-to-floating --id {window_titled(title)['id']}")

          def commonest():
              photograph(picture)
              left, top, width, height = drawn_at(window_titled(title))
              image = Image.open(os.path.join(machine.out_dir, picture + ".png")).convert("RGB")
              inside = image.crop((left + 20, top + 20, left + width - 20, top + height - 20))
              # Pillow gives no counts when there are more colours than asked
              # for, and there can't be more than there are pixels.
              counts = inside.getcolors(inside.width * inside.height)
              assert counts is not None
              _, colour = max(counts)
              return "#%02x%02x%02x" % colour

          colour = settles_on(wanted, commonest)
          as_account(f"niri msg action close-window --id {window_titled(title)['id']}")
          return colour

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

      # A user manager handed its whole service's pressure to watch can't set
      # its own threshold on it, and says so each time it starts or reloads.
      with subtest("every user manager watches its own pressure"):
          as_account("systemctl --user daemon-reload")
          machine.fail("journalctl -b -o cat | grep -F 'pressure threshold'")

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
          with told("drawn notifications"):
              as_account("notify-send 'Build finished' 'the laptop is ready'")
          photograph("notification")

      # Quickshell hears a node change only once it has bound the node's audio,
      # a moment after WirePlumber names it the default, which the meter says,
      # so one change is made after that and its meter answers it.
      def meter_shows(namespace, kind, change):
          said = as_account(f"wpctl inspect @DEFAULT_AUDIO_{kind}@")
          found = re.search(r'node\.name = "([^"]+)"', said)
          assert found is not None, f"no default {kind.lower()} has a name"
          node = found.group(1)
          reported(f"hears {namespace} {node}", lambda rest: rest == "", "", timeout=60)
          with told(f"drawn {namespace}"):
              as_account(change)
          photograph(namespace)

      with subtest("the volume shows when it changes, then goes"):
          # WirePlumber picks the defaults a moment after Pipewire lists the nodes.
          defaults = "wpctl inspect @DEFAULT_AUDIO_SINK@ && wpctl inspect @DEFAULT_AUDIO_SOURCE@"
          machine.wait_until_succeeds(in_session(defaults), timeout=30)
          meter_shows("volume", "SINK", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 10%-")
          machine.succeed(
              in_session("wpctl inspect @DEFAULT_AUDIO_SINK@ | grep -q stand-in-speakers")
          )
          machine.wait_until_fails(in_session("niri msg layers | grep -q volume"), timeout=10)

      # The microphone's key mutes it through the same command, which niri runs.
      with subtest("the microphone shows when it mutes, then goes"):
          toggle = "niri msg action spawn -- wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"
          meter_shows("microphone", "SOURCE", toggle)
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
          # Unmuted again, which the margin hears, so it's empty for what's next.
          since = mark()
          as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
          print(as_account("wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
          reported("holds margin", lambda rest: "muted" not in rest.split(), since)

      # niri runs the brightness keys' command inside Joel's session, which is
      # the one logind lets change a backlight, so the test has niri run it too.
      with subtest("the brightness shows when the keys' command changes it, then goes"):
          with told("drawn brightness"):
              as_account("niri msg action spawn -- brightnessctl --class=backlight set 80%")
          machine.succeed("grep -qx 80 /sys/class/backlight/stand-in/brightness")
          photograph("brightness")
          machine.wait_until_fails(in_session("niri msg layers | grep -q brightness"), timeout=10)

      with subtest("the charge shows when power goes and comes back, then goes"):
          # The kernel's stand-in has two supplies, mains and USB, and UPower
          # counts the machine on battery only with both off.
          def power(state):
              parameters = "/sys/module/test_power/parameters"
              for supply in ["ac", "usb"]:
                  machine.succeed(f"echo {state} > {parameters}/{supply}_online")

          for state in ["off", "on"]:
              with told("drawn power"):
                  power(state)
              photograph(f"power-{state}")
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

          def beside():
              photograph("focused")
              left, top, _, height = drawn_at(json.loads(as_account("niri msg --json focused-window")))
              image = Image.open(os.path.join(machine.out_dir, "focused.png")).convert("RGB")
              return "#%02x%02x%02x" % image.getpixel((left - 1, top + height // 2))

          framed = settles_on("${roles.accent}".lower(), beside)
          as_account(f"niri msg action move-window-to-tiling --id {window['id']}")
          t.assertEqual(framed, "${roles.accent}".lower())

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
          # With no network, what has to ask one says it couldn't, and nothing
          # breaks. Each waits for its answer to settle, failed; asked again
          # while that failure stands, it says so at once. When a failure is
          # asked again is the launcher's answers check's to say.
          for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "ip"]:
              answer(typed)
              machine.send_key("esc")
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
          # Each copy is waited on until the shell has heard it, so they're
          # kept in the order made.
          with told("heard copy Notes for Tuesday"):
              as_account("echo -n 'Notes for Tuesday' | wl-copy > /dev/null 2>&1")
          with told("heard copy the second copy"):
              as_account("printf 'the second copy\\nover two lines' | wl-copy > /dev/null 2>&1")
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

      with subtest("a screenshot just taken is at the launcher's home, and opens in GIMP"):
          pictures = "/home/${account}/Pictures/Screenshots"
          # Made at login, since niri makes it only with its first screenshot
          # and the launcher watches it from the start.
          machine.succeed(f"test -d {pictures}")
          as_account("niri msg action screenshot-screen")
          machine.wait_until_succeeds(f"ls {pictures}/*.png", timeout=15)
          picture = machine.succeed(f"ls {pictures}/*.png").strip()
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          # Just below the window in front, which home always puts first.
          machine.send_key("down")
          launcher_until(
              lambda shown: (shown["selected"] or {}).get("name") == "Edit the screenshot in GIMP"
          )
          machine.send_key("ret")
          machine.wait_until_succeeds(f"pgrep -u ${account} -f '[g]imp.*{picture}'", timeout=30)
          # GIMP goes before it draws, so no window of its own takes the keys
          # meant for what follows, and the picture too, since the region
          # screenshot's subtest waits for one of its own. GIMP takes any other
          # signal as a crash, and writes its backtrace to the log.
          machine.succeed("pkill -KILL -u ${account} -f '[g]imp'")
          machine.wait_until_fails("pgrep -u ${account} -f '[g]imp'", timeout=30)
          machine.succeed(f"rm {pictures}/*.png")

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
          with told("refused wifi Lantern", timeout=60):
              machine.send_key("ret")
          # NetworkManager's own word may land in its journal just after.
          machine.wait_until_succeeds(
              "journalctl -u NetworkManager"
              " | grep -q \"Activation: failed for connection 'Lantern'\"",
              timeout=10,
          )
          photograph("launcher-wifi-refused")
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
          with told("shows card Lunch"):
              as_account("notify-send 'Lunch' 'at noon'")
          # notify-send waits for the answer, so it runs on its own. The
          # photograph waits for its card's frames to settle, which is when
          # Qt would have warned of a layout that never does.
          with told("shows card Recording saved"):
              as_account(
                  "notify-send --action=play='Play it' 'Recording saved' 'x.mp4'"
                  " > /tmp/answer 2>&1 < /dev/null &"
              )
          photograph("notice-with-action")
          # Qt only warns when a layout never settles, which left the
          # card's words piled at its top.
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          assert "polish() loop" not in log, "a notice's layout never settles"
          # Super+N opens on what it offers, so Return does the first. Its
          # card stops counting down while the launcher holds it, so what it
          # offers still answers however long the keys take.
          with told("waits card Recording saved"):
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

      # A notice offering to play, and where its action's glyph is drawn, as
      # a fraction of the screen. The glyph is the only thing in the action
      # colour. notify-send waits for the answer, so it runs on its own, and
      # the notice stays a minute, so only a click closes it sooner.
      def offer_to_play(picture):
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          with told(f"shows card {picture}"):
              as_account(
                  f"notify-send --expire-time=60000 --action=play='Play it' '{picture}' 'y.mp4'"
                  " > /tmp/answer 2>&1 < /dev/null &"
              )
          action = [int("${roles.action}"[i : i + 2], 16) for i in (1, 3, 5)]
          drawn = []
          size = [1, 1]

          # The card's first frames may not hold the glyph yet.
          def glyph_drawn(last):
              photograph(picture)
              image = Image.open(os.path.join(machine.out_dir, picture + ".png")).convert("RGB")
              pixels = image.tobytes()
              size[:] = [image.width, image.height]
              drawn[:] = [
                  (x, y)
                  for y in range(image.height)
                  for x in range(image.width)
                  if sum(abs(pixels[3 * (y * image.width + x) + c] - action[c]) for c in range(3))
                  < 40
              ]
              if last:
                  assert drawn, "the action's glyph isn't drawn"
              return bool(drawn)

          retry(glyph_drawn, timeout_seconds=15)
          xs, ys = [x for x, _ in drawn], [y for _, y in drawn]
          assert max(xs) - min(xs) < 30 and max(ys) - min(ys) < 30, f"more than one glyph: {drawn}"
          return sum(xs) / len(xs) / size[0], sum(ys) / len(ys) / size[1]

      # A click there, by QEMU's tablet, which spans the screen from 0 to
      # 32767. The driver says a command's arguments are strings, so the
      # events go through JSON to reach it as they are.
      def click(x, y):
          def tablet(*events):
              assert machine.qmp_client is not None
              machine.qmp_client.send("input-send-event", json.loads(json.dumps({"events": events})))

          tablet(
              {"type": "abs", "data": {"axis": "x", "value": int(x * 32767)}},
              {"type": "abs", "data": {"axis": "y", "value": int(y * 32767)}},
          )
          # QEMU hands the guest each batch in the order sent, so the press
          # lands where the pointer went.
          tablet({"type": "btn", "data": {"down": True, "button": "left"}})
          tablet({"type": "btn", "data": {"down": False, "button": "left"}})

      # notify-send ends once its notice closes, saying the action if one
      # was done and nothing if the notice was only put away.
      def answered():
          machine.wait_until_fails("pgrep -f '[n]otify-send .*--action=play'", timeout=15)
          return machine.succeed("cat /tmp/answer").strip()

      with subtest("a click on a notice's action does it, and anywhere else puts it away"):
          since = mark()
          x, y = offer_to_play("notice-to-click")
          click(x, y)
          assert answered() == "play", "a click on the action put the notice away instead"
          # A little above the action is the notice's message.
          x, y = offer_to_play("notice-to-put-away")
          click(x, y - 0.04)
          assert answered() == "", "a click on the message did the action"
          # Neither was missed, so the margin never once said one waits: a
          # notice arriving or leaving mustn't flash it.
          said = as_account(
              "journalctl --user --unit=quickshell --no-pager --output=cat"
              f" --after-cursor={shlex.quote(since)}"
          )
          t.assertNotIn("holds margin waiting", said, "the margin flashed a waiting notice")

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
          with told("drawn status"):
              as_account("niri msg action open-overview")
          photograph("overview-awake")
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
          with told("drawn margin"):
              as_account("systemctl --user start caffeinate.service")
          photograph("margin-awake")
          # The panel takes the keys, and a click anywhere else puts it away.
          with told("keys margin"):
              machine.send_key("meta_l-o")
          wait_for_surface("outside")
          photograph("margin-drawer")
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
          # The shell decides as the notice arrives, so once it says it held
          # one, there's no card to come.
          with told("held Held back"):
              as_account("notify-send 'Held back' 'until it ends'")
          machine.fail(in_session(notices))
          # Its panel offers ending it, an hour more, then what's held.
          with told("keys margin"):
              machine.send_key("meta_l-o")
          photograph("margin-quiet")
          machine.send_key("down")
          machine.send_key("down")
          machine.send_key("ret")
          launcher_ready()
          machine.screenshot("launcher-held")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          with told("keys margin"):
              machine.send_key("meta_l-o")
          machine.send_key("ret")
          machine.wait_until_fails(in_session(margin), timeout=10)
          as_account("test ! -s $XDG_RUNTIME_DIR/quiet")

      with subtest("a missed notice lights its pull, and Return shows the notices"):
          # On screen it isn't waiting yet; once it goes by itself, it is.
          with told("drawn margin"):
              with told("shows card Peter"):
                  as_account("notify-send --expire-time=1000 'Peter' 'are you there?'")
          machine.wait_until_fails(in_session(notices), timeout=15)
          photograph("margin-waiting")
          with told("keys margin"):
              machine.send_key("meta_l-o")
          machine.send_key("ret")
          launcher_ready()
          machine.screenshot("launcher-from-margin")
          machine.send_key("esc")
          # Opening them counted them seen, so nothing's waiting.
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("the microphone muted shows in the margin, and Return unmutes it"):
          try:
              with told("drawn margin"):
                  as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
          except AssertionError:
              # Which source Pipewire calls the default, and whether it's muted.
              print(as_account("wpctl status; wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
              raise
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
          with told("keys margin"):
              machine.send_key("meta_l-o")
          photograph("margin-muted")
          machine.send_key("ret")
          muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
          machine.wait_until_fails(in_session(muted), timeout=10)
          machine.wait_until_fails(in_session(margin), timeout=10)

      # Any player that tells the desktop what it plays shows in the margin
      # while it plays, and the media keys' command shows what it is.
      with subtest("what's playing shows in the margin, and a media key shows what it is"):
          machine.wait_until_fails(in_session(margin), timeout=10)
          with told("drawn margin"):
              as_account(
                  "systemd-run --user --unit=player mpv --ao=null --no-video --loop ${track}"
              )
          photograph("margin-playing")
          # The play key pauses it, and the card says what was paused.
          with told("drawn notifications"):
              as_account("qs ipc call music toggle")
          photograph("music-card")
          # Paused, nothing's playing, so the margin goes; the card goes by itself.
          machine.wait_until_fails(in_session(margin), timeout=10)
          machine.wait_until_fails(in_session(notices), timeout=15)
          as_account("systemctl --user stop player")

      with subtest("a Tuple call shows in the margin while it lasts"):
          # Only a call writes these lines, so they're written here, with
          # the daemon stopped, since it rewrites its log as it starts.
          as_account("systemctl --user stop tuple")
          written = "~/.local/share/tuple/0/log.txt"

          # A line as the daemon writes it, as if `ago` seconds since.
          def said(message, ago=0):
              line = f"$(date -u -d '-{ago} sec' +%H:%M:%S.000)|1|1|info|{message}"
              as_account(f"mkdir -p $(dirname {written}) && echo \"{line}\" >> {written}")

          machine.wait_until_fails(in_session(margin), timeout=10)
          with told("drawn margin"):
              said("call connected")
          photograph("margin-call")
          said("Caller.cpp:985|invalidating call")
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("people coming and going in a Tuple call are said, and Joel can leave"):
          # Whoever comes is "a guest", not a number, which tuple ls would
          # be asked about, and with the daemon stopped it starts one.
          offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action={0}'"
          # Those there as Joel joins aren't news, so the call connected a
          # while before the guest comes.
          with told("drawn margin"):
              said("call connected", ago=10)
          with told("shows card Someone joined the call"):
              said("peer joined call: a guest")
          photograph("notice-joined")
          with told("shows card Someone left the call"):
              said("peer left call: a guest")
          machine.succeed(offered.format("leave"))
          photograph("notice-alone")
          # Leaving runs tuple end, which starts the stopped daemon, whose
          # new log ends the call.
          machine.send_key("meta_l-n")
          launcher_ready()
          launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Leave the call")
          machine.send_key("ret")
          machine.wait_until_fails(in_session(margin), timeout=15)
          machine.wait_until_fails(offered.format("leave"), timeout=10)
          as_account("tuple off")

      with subtest("a Tuple call ringing wants Joel, and one not answered is kept as missed"):
          offered = "pgrep -f '[n]otify-send --app-name=Tuple .*--action=show'"
          machine.wait_until_fails(in_session(margin), timeout=10)
          with told("shows card Someone is calling"):
              said("received incoming call: a caller")
          wait_for_surface("margin")
          machine.succeed(offered)
          photograph("margin-ringing")
          # The caller gives up: the ring's notice goes, and a missed call stays.
          said("Caller.cpp:985|invalidating call")
          machine.wait_until_fails(offered, timeout=10)
          kept = "grep -q 'Missed a call from someone' ~/.local/state/quickshell/by-shell/*/notices.json"
          machine.wait_until_succeeds(in_session(kept), timeout=10)
          # Once it goes by itself it waits, and the margin shows it.
          machine.wait_until_fails(in_session(notices), timeout=15)
          wait_for_surface("margin")
          machine.send_key("meta_l-n")
          launcher_ready()
          machine.send_key("esc")
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("a copied call's link can be joined until something else is copied"):
          link = "https://tuple.app/c/from-a-chat"
          as_account(f"echo -n 'pair with me? {link}' | wl-copy > /dev/null 2>&1")
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("join", selecting="Join the Tuple call")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

          # A screenshot is copied as an image alone, with no text to read.
          as_account("printf '\\211PNG\\r\\n' | wl-copy --type image/png > /dev/null 2>&1")
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("join")

          def joins_nothing(shown):
              row = shown["selected"]
              return row is None or "Tuple" not in row["name"]

          launcher_until(joins_nothing)
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

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

      # Print opens the launcher's Capture collection, where a picture of the
      # screen, a window or a region is chosen by the keys, and niri says
      # when it's taken.
      pictures = "/home/${account}/Pictures/Screenshots"

      # Print opens Capture, its rows ready for the keys.
      def capture():
          machine.send_key("sysrq")
          launcher_until(
              lambda shown: shown["keys"]
              and shown["held"] == ["captures"]
              and shown["selected"] is not None
          )

      # Waits for niri to say it took a picture, after what's done inside this.
      @contextlib.contextmanager
      def captured():
          since = mark()
          yield
          reported("captured ", lambda path: path.startswith(f"{pictures}/"), since)

      # A PNG says its width and height in its header, from byte 16.
      def newest_size():
          newest = f"ls -t {pictures}/*.png | head -1"
          size = machine.succeed(f"{newest} | xargs -d '\\n' od -An -tu1 -j16 -N8")
          b = [int(n) for n in size.split()]
          return (b[2] * 256 + b[3], b[6] * 256 + b[7])

      with subtest("Print opens Capture, where Return takes the whole screen"):
          capture()
          screen = showing()["selected"]
          t.assertEqual(screen["name"], "The whole screen")
          t.assertEqual(screen["chord"], "\u21e7 Print", "the whole screen's row shows its chord")
          photograph("capture")
          with told("shows card Screenshot captured"), captured():
              machine.send_key("ret")

      with subtest("Capture takes a window found by its title, on any workspace"):
          as_account("niri msg action spawn -- kitty --title Seashells")
          machine.wait_until_succeeds(
              in_session("niri msg --json windows | grep -qF '\"title\":\"Seashells\"'"), timeout=60
          )
          # The window the keys were in, though niri says none has focus
          # while the launcher has the keys.
          capture()
          type_in("this window", selecting="This window \u00b7 Seashells")
          t.assertEqual(showing()["selected"]["chord"], "Ctrl Print")
          with captured():
              machine.send_key("ret")
          windows = json.loads(as_account("niri msg --json windows"))
          shell = next(w for w in windows if w["title"] == "Seashells")
          t.assertEqual(newest_size(), tuple(shell["layout"]["window_size"]))
          # Moved to a workspace of its own and left there.
          as_account("niri msg action move-window-to-workspace-down")
          as_account("niri msg action focus-workspace-up")
          capture()
          type_in("seashells", selecting="Seashells")
          with captured():
              machine.send_key("ret")
          windows = json.loads(as_account("niri msg --json windows"))
          shell = next(w for w in windows if w["title"] == "Seashells")
          t.assertEqual(newest_size(), tuple(shell["layout"]["window_size"]))
          as_account(f"niri msg action close-window --id {shell['id']}")

      with subtest("A region is picked by the keys, and Escape goes back to Capture"):
          # The picker covers the screen niri gives it, whatever cage leaves.
          logical = json.loads(as_account("niri msg --json focused-output"))["logical"]
          wide, high = logical["width"], logical["height"]
          # It opens on the middle quarter; an arrow moves it a step, and
          # Shift and an arrow size it from its bottom right corner.
          width, height = round(wide / 2), round(high / 2)
          x, y = round((wide - width) / 2), round((high - height) / 2)
          before = int(machine.succeed(f"ls {pictures}/*.png | wc -l"))

          # Return at once takes the box it opened on, which has to be there
          # from its first frame, though the window learns its size later.
          capture()
          type_in("region", selecting="A region")
          with told("keys picker"):
              machine.send_key("ret")
          with told(f"picked {x},{y} {width}x{height}"):
              machine.send_key("ret")
          machine.wait_until_succeeds(
              f"ls {pictures}/*.png | wc -l | grep -qx {before + 1}", timeout=15
          )
          t.assertEqual(newest_size(), (width, height))

          capture()
          type_in("region", selecting="A region")
          with told("keys picker"):
              machine.send_key("ret")
          for key in ("left", "up", "shift-right", "shift-right"):
              machine.send_key(key)
          photograph("picker")
          with told(f"picked {x - 16},{y - 16} {width + 32}x{height}"):
              machine.send_key("ret")
          machine.wait_until_succeeds(
              f"ls {pictures}/*.png | wc -l | grep -qx {before + 2}", timeout=15
          )
          t.assertEqual(newest_size(), (width + 32, height))

          # One cell's letters and another's make the box between them.
          capture()
          type_in("region", selecting="A region")
          with told("keys picker"):
              machine.send_key("ret")
          machine.send_chars("aab")
          photograph("picker-cells")
          rows = round(16 * high / wide)
          with told(f"picked 0,0 {round(2 * wide / 16)}x{round(2 * high / rows)}"):
              machine.send_chars("b\n")

          # Escape clears the letters typed first, then goes back to Capture,
          # taking nothing.
          capture()
          type_in("region", selecting="A region")
          with told("keys picker"):
              machine.send_key("ret")
          machine.send_chars("c")
          machine.send_key("esc")
          with told("gone picker"):
              machine.send_key("esc")
          launcher_until(lambda shown: shown["keys"] and shown["held"] == ["captures"])
          machine.send_key("esc")
          launcher_until(lambda shown: not shown["open"])

      with subtest("the camera takes a picture and a video by the keys"):
          camera = "niri msg --json windows | grep -qF '\"app_id\":\"camera\"'"
          capture()
          type_in("picture from the camera", selecting="camera")
          machine.send_key("ret")
          machine.wait_until_succeeds(in_session(camera), timeout=60)
          # Return before the camera wakes takes the picture once it has.
          with told("shows card Picture copied", timeout=60):
              machine.send_key("ret")
          photograph("camera")
          # Two at once are both kept, under names of their own.
          since = mark()
          machine.send_key("ret")
          machine.send_key("ret")
          pictures_taken = []
          reported(
              "shows card Picture copied",
              lambda rest: pictures_taken.append(rest) is None and len(pictures_taken) == 2,
              since,
              timeout=60,
          )
          # What ffprobe says of each file there, which it reads one at a time.
          def probe(entries, files):
              ffprobe = "${pkgs.ffmpeg-headless}/bin/ffprobe -v error -of csv=p=0"
              each = f'for f in {files}; do {ffprobe} -show_entries {entries} "$f" || exit; done'
              return machine.succeed(each)

          sizes = probe("stream=width,height", "/home/${account}/Pictures/Camera/*.jpg")
          t.assertEqual(sizes.split(), ["640,480"] * 3)

          videos = "/home/${account}/Videos/Camera"
          kept = in_session("cat ~/.local/state/quickshell/by-shell/*/notices.json")

          # The videos there are, and once the newest has grown, recording.
          def recording(count, last):
              failed = re.search(
                  r'"summary":"The video could not be recorded",[^}]*', machine.succeed(kept)
              )
              if failed is not None:
                  # What the recording said, and who had the camera then.
                  print(machine.execute("cat /run/user/1000/camera-recording.log")[1])
                  holders = (
                      "for p in /proc/[0-9]*; do"
                      " ls -l $p/fd 2>/dev/null | grep -q video9 && echo open $p $(cat $p/comm);"
                      " grep -qs video9 $p/maps && echo mapped $p $(cat $p/comm); done; true"
                  )
                  print(machine.succeed(holders))
                  raise AssertionError(failed.group(0))
              grown = (
                  f"[ $(ls {videos}/*.mp4 | wc -l) = {count} ]"
                  f' && [ $(stat -c %s "$(ls -t {videos}/*.mp4 | head -1)") -gt 50000 ]'
              )
              status, _ = machine.execute(grown)
              if last and status != 0:
                  print(machine.execute("ps -eww -o pid,args | grep -aE 'ffmpeg|mpv|sh -c'")[1])
                  print(as_account("pw-top -b -n 2 || true"))
                  print(machine.execute(f"ls -la /run/user/1000 {videos}")[1])
                  print(machine.execute("cat /run/user/1000/camera-recording.log")[1])
                  print(as_account("journalctl --user --no-pager -n 60 -o cat"))
              assert not last or status == 0, f"video {count} never grew"
              return status == 0

          # Tab makes Return record, and Return again stops it.
          machine.send_key("tab")
          machine.send_key("ret")
          retry(lambda last: recording(1, last), timeout_seconds=60)
          with told("shows card Video saved", timeout=60):
              machine.send_key("ret")
          # Escape while recording saves the video, then closes.
          machine.wait_until_succeeds(in_session(camera), timeout=10)
          machine.send_key("ret")
          retry(lambda last: recording(2, last), timeout_seconds=60)
          with told("shows card Video saved", timeout=60):
              machine.send_key("esc")
          machine.wait_until_fails(in_session(camera), timeout=30)
          streams = probe("stream=codec_name", f"{videos}/*.mp4")
          t.assertEqual(streams.split(), ["h264", "aac"] * 2)

          # A picture is kept as others see Joel, not mirrored as the viewfinder
          # shows him: it matches what the camera gives better than its mirror.
          ffmpeg = "${pkgs.ffmpeg-headless}/bin/ffmpeg"
          machine.succeed(f"{ffmpeg} -v error -f v4l2 -i /dev/video9 -frames:v 1 -y /tmp/given.png")
          picture = machine.succeed("ls /home/${account}/Pictures/Camera/*.jpg | head -1").strip()

          def likeness(turn):
              compared = f"[1]{turn}format=yuv420p[given];[0]format=yuv420p[kept];[kept][given]psnr"
              said = machine.succeed(
                  f'{ffmpeg} -i "{picture}" -i /tmp/given.png -lavfi "{compared}" -f null - 2>&1'
              )
              found = re.search(r"average:([0-9.]+)", said)
              assert found is not None, said
              return float(found.group(1))

          t.assertGreater(likeness(""), likeness("hflip,") + 3)

      # Super+Shift+N puts the cards away, and each is still there unread.
      with subtest("Super+Shift+N puts the notices away"):
          with told("shows card Standup"):
              as_account("notify-send --urgency=critical 'Standup' 'in five minutes'")
          with told("gone notifications"):
              machine.send_key("meta_l-shift-n")
          kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
          machine.wait_until_succeeds(
              in_session(kept + " | grep -q '\"summary\":\"Standup\"[^}]*\"read\":false'"),
              timeout=10,
          )

      with subtest("the lock screen takes only the right password"):
          with told("keys lock"):
              machine.send_key("meta_l-l")
          as_account("qs ipc call lock isLocked | grep -q true")
          # PAM answers a wrong password after a delay, which the lock says.
          with told("refused lock"):
              machine.send_chars("wrong-password\n")
          as_account("qs ipc call lock isLocked | grep -q true")

          # Its picture fades in, as the wallpaper's does.
          def pictured(last):
              _, colours, _ = colours_and_light("locked")
              if last:
                  t.assertGreater(
                      colours, 1000, "the lock screen shows a flat colour, not its picture"
                  )
              return colours > 1000

          retry(pictured, timeout_seconds=30)
          machine.send_chars("${secret}\n")
          machine.wait_until_succeeds(in_session("qs ipc call lock isLocked | grep -q false"), timeout=30)

      with subtest("polkit's prompt lets the account start a system service"):
          with told("keys polkit"):
              as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
          photograph("polkit")
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
          sunken = "${roles.sunken}".lower()
          t.assertEqual(commonest_colour_in("libadwaita", "libadwaita", sunken), sunken)
          # The file chooser Chrome and Slack open, which the GTK portal draws
          # with GTK 3, asked for the way they ask.
          as_account(
              "busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
              " org.freedesktop.portal.FileChooser OpenFile 'ssa{sv}' \"\" Open 0"
          )
          t.assertEqual(commonest_colour_in("Open", "file-chooser", sunken), sunken)

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
