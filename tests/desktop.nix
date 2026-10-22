# Starts the graphical role's desktop in a virtual machine and uses it the
# way Joel does: the wallpaper stands behind the workspaces, the overview
# draws nothing of the shell's and the launcher says how things stand, a
# notification appears, the volume and
# brightness bars come and go as those change, the launcher finds and opens
# kitty and remembers it was picked, a terminal opens below the focused
# window, the launcher switches to a window by its title, the lock screen
# refuses a wrong password and takes the right one, polkit's prompt lets the
# account start a system service, what's playing shows in the margin and a
# media key shows what it is, a call shows in the margin while it lasts,
# people coming and going in it are said and Joel can leave from the last
# one's notice, a ringing call wants him and one not answered is kept as
# missed, a copied call's link can be joined until something else is
# copied, the clipboard takes and gives text, a picture opens floating in
# swayimg and a key takes it to GIMP, a document opens in zathura beside
# the rest, and GTK programs and monospaced text are in the theme. niri says which surfaces it drew,
# photographs show their colours, to the test and to a person looking, and
# the portal and fontconfig answer for the settings. Quickshell has to
# start without a single QML error.
{ self, pkgs }:
let
  account = "jitsusama";
  secret = "correct-horse";

  # A certificate authority the machine trusts, and the Office's server's
  # certificate from it, naming radius.office.test.
  certificates = pkgs.runCommand "stand-in-certificates" { nativeBuildInputs = [ pkgs.openssl ]; } ''
    mkdir $out
    cd $out
    key="-newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes"
    openssl req -x509 $key -days 3650 -subj /CN=Stand-in -keyout authority.key -out authority.pem \
      -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign
    openssl req $key -subj /CN=radius.office.test -keyout radius.key -out radius.csr
    printf '%s\n' subjectAltName=DNS:radius.office.test extendedKeyUsage=serverAuth > radius.ext
    openssl x509 -req -in radius.csr -CA authority.pem -CAkey authority.key -CAcreateserial \
      -days 3650 -extfile radius.ext -out radius.pem
  '';

  # Lantern's front desk, as a hotel's: every page is sent there until
  # someone signs in, which its one button does.
  frontDesk = pkgs.writeText "front-desk.py" ''
    from http.server import BaseHTTPRequestHandler, HTTPServer

    PAGE = b"""<!doctype html><title>Lantern</title>
    <form method="post" action="/sign-in"><button autofocus>Sign in</button></form>"""
    signed_in = False

    class Desk(BaseHTTPRequestHandler):
        def answer(self, code, headers={}, body=b""):
            self.send_response(code)
            for name, value in headers.items():
                self.send_header(name, value)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path == "/sign-in":
                return self.answer(200, {"Content-Type": "text/html"}, PAGE)
            if signed_in:
                return self.answer(204)
            self.answer(302, {"Location": "http://front-desk.lantern/sign-in"})

        def do_POST(self):
            global signed_in
            self.rfile.read(int(self.headers.get("Content-Length", 0)))
            signed_in = True
            self.answer(200, {"Content-Type": "text/html"}, b"<p>Welcome to Lantern.</p>")

    HTTPServer(("10.42.0.1", 80), Desk).serve_forever()
  '';

  # Text views and file lists draw on the theme's dark background, which
  # nothing else on the screen uses, so a window that hasn't drawn can't pass
  # for one that has.
  text = pkgs.writeText "text" "A text view.";

  # Asks through SSH_ASKPASS as ssh-tpm-agent and ssh do: the kind, as
  # SSH_ASKPASS_PROMPT, or "secret" for none, then the prompt, with \n for
  # its line breaks. What comes back and how it exits land beside each other
  # in the runtime folder, the exit last.
  asking = pkgs.writeShellScript "asking" ''
    kind=$1
    out=$XDG_RUNTIME_DIR/asked-$kind
    rm -f "$out".*
    if [ "$kind" = secret ]; then unset SSH_ASKPASS_PROMPT; else export SSH_ASKPASS_PROMPT=$kind; fi
    "$SSH_ASKPASS" "$(printf '%b' "$2")" > "$out.answer"
    echo $? > "$out.part"
    mv "$out.part" "$out.exit"
  '';

  # A stand-in touchscreen, as wide as the screen, which swipes with as many
  # fingers as it's told, one way, or taps: each line written to
  # /run/touchscreen is how many and which way (RL is from right to left), or
  # tap, then how far across an up or down swipe goes, as a fraction of the
  # screen. Writing waits until it's read, so a swipe has started once it
  # returns.
  touchscreen =
    pkgs.writers.writePython3 "touchscreen"
      {
        libraries = [ pkgs.python3Packages.evdev ];
        flakeIgnore = [ "E501" ];
      }
      ''
        import os
        import time
        from evdev import AbsInfo, UInput, ecodes as e

        width, height = 1280, 800
        fifo = "/run/touchscreen"


        def axis(most):
            return AbsInfo(0, 0, most, 0, 0, 10)


        device = UInput(
            {
                e.EV_KEY: [e.BTN_TOUCH],
                e.EV_ABS: [
                    (e.ABS_X, axis(width - 1)),
                    (e.ABS_Y, axis(height - 1)),
                    (e.ABS_MT_SLOT, axis(9)),
                    (e.ABS_MT_TRACKING_ID, axis(65535)),
                    (e.ABS_MT_POSITION_X, axis(width - 1)),
                    (e.ABS_MT_POSITION_Y, axis(height - 1)),
                ],
            },
            name="stand-in touchscreen",
            input_props=[e.INPUT_PROP_DIRECT],
        )

        # Across most of the screen the way it's told, from 15% to 85%, as
        # a person swipes: the view follows the fingers, so a short swipe
        # snaps back.
        ways = {"LR": (1, 0), "RL": (-1, 0), "UD": (0, 1), "DU": (0, -1)}


        def tap():
            device.write(e.EV_ABS, e.ABS_MT_SLOT, 0)
            device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, 1)
            device.write(e.EV_ABS, e.ABS_MT_POSITION_X, width // 2)
            device.write(e.EV_ABS, e.ABS_MT_POSITION_Y, height // 2)
            device.write(e.EV_KEY, e.BTN_TOUCH, 1)
            device.syn()
            time.sleep(0.05)
            device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, -1)
            device.write(e.EV_KEY, e.BTN_TOUCH, 0)
            device.syn()


        def swipe(fingers, way, across):
            dx, dy = ways[way]
            steps = 10
            for step in range(steps + 1):
                along = 14 * step - 7 * steps
                for finger in range(fingers):
                    x = int(width * across) + dx * width * along // (20 * steps)
                    x += 40 * finger * abs(dy)
                    y = height // 2 + dy * height * along // (20 * steps)
                    y += 40 * finger * abs(dx)
                    device.write(e.EV_ABS, e.ABS_MT_SLOT, finger)
                    if step == 0:
                        device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, finger + 1)
                    device.write(e.EV_ABS, e.ABS_MT_POSITION_X, x)
                    device.write(e.EV_ABS, e.ABS_MT_POSITION_Y, y)
                if step == 0:
                    device.write(e.EV_KEY, e.BTN_TOUCH, 1)
                device.syn()
                time.sleep(0.02)
            for finger in range(fingers):
                device.write(e.EV_ABS, e.ABS_MT_SLOT, finger)
                device.write(e.EV_ABS, e.ABS_MT_TRACKING_ID, -1)
            device.write(e.EV_KEY, e.BTN_TOUCH, 0)
            device.syn()


        os.mkfifo(fifo)
        while True:
            with open(fifo) as asked:
                for line in asked:
                    fingers, way, across = line.split()
                    if way == "tap":
                        tap()
                    else:
                        swipe(int(fingers), way, float(across))
      '';

  # A picture and a document to open, as a download would be.
  picture = "${pkgs.nixos-icons}/share/icons/hicolor/256x256/apps/nix-snowflake.png";
  page = import ./page.nix {
    inherit (pkgs) lib;
    inherit pkgs;
  };

  # What the shell has reported (modules/quickshell/Report.qml): waits until
  # it reports something starting with a prefix after a point in its
  # journal, or since the boot with no point given, then prints every
  # report since that point, each as "said", a tab and what was said, and
  # last the point it read to, as "cursor" and a tab. It reads the journal
  # again every fifth of a second rather than following it, since
  # journalctl's follower is not always woken for an entry: on a busy
  # machine it missed one in 300 that a fresh read found. The journal
  # keeps Quickshell's lines as bytes, since it colours their level, so
  # they're read as plain text: JSON would give them as numbers.
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
      journal=(journalctl --user --unit=quickshell --no-pager --output=cat)
      journal+=(--lines=all --show-cursor)
      if [ -n "$since" ]; then journal+=("--after-cursor=$since"); else journal+=(--boot); fi
      deadline=$((SECONDS + seconds))
      while :; do
        said=$("''${journal[@]}")
        if grep -aqF "report[$prefix" <<< "$said"; then break; fi
        [ "$SECONDS" -lt "$deadline" ] || exit 1
        sleep 0.2
      done
      sed -n -e 's/^-- cursor: /cursor\t/p' -e 's/.*report\[\(.*\)\]$/said\t\1/p' <<< "$said"
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
        # The laptop role's Wi-Fi, which the shell's network piece and Wi-Fi
        # collection read.
        self.nixosModules.wifid
      ];

      # The VM has no backlight, light sensor or sound card, so it gets one of
      # each that drives nothing, which the kernel and PipeWire report as they
      # do the laptop's.
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-backlight { })
        (config.boot.kernelPackages.callPackage ./stand-in-light { })
        config.boot.kernelPackages.v4l2loopback
      ];
      boot.kernelModules = [
        "stand-in-backlight"
        "stand-in-light"
        # A stand-in camera, a loopback as the laptop's is, fed a test
        # picture as the laptop's relay feeds it the sensor's.
        "v4l2loopback"
        # The kernel's own stand-in battery, which UPower reports as a laptop's.
        "test_power"
        # Three stand-in Wi-Fi radios: hostapd makes the first and the last
        # access points, and wifid joins them from the second, as the
        # laptop's would.
        "mac80211_hwsim"
        # For a stand-in touchscreen.
        "uinput"
      ];
      # The access points' radios move to a namespace of their own before
      # anything joins, so wlan1 is the machine's only card and each join
      # crosses the stand-in air.
      systemd.services.air = {
        after = [ "systemd-modules-load.service" ];
        before = [ "wpa_supplicant.service" ];
        wantedBy = [ "multi-user.target" ];
        path = [
          pkgs.coreutils
          pkgs.iproute2
          pkgs.iw
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
        };
        script = ''
          until [ -e /sys/class/net/wlan2/phy80211/name ]; do sleep 0.1; done
          ip netns add air
          for card in wlan0 wlan2; do
            iw phy "$(cat /sys/class/net/$card/phy80211/name)" set netns name air
          done
          ip -n air link set lo up
          ip -n air address add 10.42.0.1/24 dev wlan0
          ip -n air address add 10.44.0.1/24 dev wlan2
        '';
      };
      systemd.services.lantern = {
        after = [ "air.service" ];
        requires = [ "air.service" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          NetworkNamespacePath = "/run/netns/air";
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${pkgs.writeText "lantern.conf" ''
            interface=wlan0
            driver=nl80211
            hw_mode=g
            channel=6
            country_code=CA
            ieee80211d=1
            ssid=Lantern
            wpa=2
            wpa_key_mgmt=WPA-PSK
            rsn_pairwise=CCMP
            wpa_passphrase=glowing in the dark
          ''}";
        };
      };
      # The Office asks for an account, as a workplace's network does, and
      # its server proves itself with the certificate naming
      # radius.office.test.
      systemd.services.office = {
        after = [ "air.service" ];
        requires = [ "air.service" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          NetworkNamespacePath = "/run/netns/air";
          ExecStart = "${lib.getExe' pkgs.hostapd "hostapd"} ${pkgs.writeText "office.conf" ''
            interface=wlan2
            driver=nl80211
            hw_mode=g
            channel=1
            country_code=CA
            ieee80211d=1
            ssid=Office
            wpa=2
            wpa_key_mgmt=WPA-EAP
            rsn_pairwise=CCMP
            ieee8021x=1
            eap_server=1
            eap_user_file=${pkgs.writeText "office-accounts" ''
              * PEAP,TTLS
              "joel" MSCHAPV2,TTLS-PAP,TTLS-MSCHAPV2 "hunter22" [2]
            ''}
            ca_cert=${certificates}/authority.pem
            server_cert=${certificates}/radius.pem
            private_key=${certificates}/radius.key
          ''}";
        };
      };
      security.pki.certificateFiles = [ "${certificates}/authority.pem" ];
      # The access points hand out addresses, as a real network does, and
      # say every name is Lantern's, as a network wanting a sign-in does.
      systemd.services.lantern-addresses = {
        after = [ "lantern.service" ];
        requires = [ "air.service" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          NetworkNamespacePath = "/run/netns/air";
          ExecStart = lib.concatStringsSep " " [
            "${lib.getExe pkgs.dnsmasq} --keep-in-foreground --dhcp-leasefile=/run/lantern.leases"
            "--interface=wlan0 --bind-dynamic --dhcp-range=10.42.0.10,10.42.0.50,1h"
            "--interface=wlan2 --dhcp-range=10.44.0.10,10.44.0.50,1h"
            "--no-resolv --no-hosts --address=/#/10.42.0.1"
          ];
        };
      };
      systemd.services.lantern-front-desk = {
        after = [ "air.service" ];
        requires = [ "air.service" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          NetworkNamespacePath = "/run/netns/air";
          ExecStart = "${lib.getExe pkgs.python3} ${frontDesk}";
        };
      };
      boot.extraModprobeConfig = ''
        options v4l2loopback video_nr=9 exclusive_caps=1 card_label=Camera
        options mac80211_hwsim radios=3
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

      # The launcher reads the battery from UPower, which the laptop role
      # brings.
      services.upower.enable = true;
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
      # its audio: Pipewire's test tone, a node of its own as a sound card's
      # is. A loopback's far side is a stream, and on about one mute in two
      # hundred Quickshell read back the mute from before, which a node of
      # its own never did in six hundred. The tone gives something to record
      # from the start, as a real microphone does, and a recording with
      # sound waits for its first sound before it writes anything.
      services.pipewire.extraConfig.pipewire."91-stand-in-microphone" = {
        "context.spa-libs".audiotestsrc = "audiotestsrc/libspa-audiotestsrc";
        "context.objects" = [
          {
            factory = "adapter";
            args = {
              "factory.name" = "audiotestsrc";
              "node.name" = "stand-in-microphone";
              "node.description" = "Stand-in microphone";
              "media.class" = "Audio/Source";
              # Two channels, as a laptop's microphones are.
              "audio.position" = [
                "FL"
                "FR"
              ];
            };
          }
        ];
      };

      virtualisation = {
        memorySize = 4096;
        cores = 4;
        qemu.options = [ "-vga none -device virtio-gpu-pci,xres=1280,yres=800" ];
      };

      users.users.${account} = {
        isNormalUser = true;
        uid = 1000;
        # Polkit asks a member of wheel for their own password.
        extraGroups = [
          "wheel"
          # The camera's, given to a seat's active session on the laptop.
          "video"
        ];
        initialPassword = secret;
      };
      home-manager.users.${account} = {
        home.stateVersion = "26.05";
        # A cursor that holds still, so two runs photograph kitty the same
        # rather than catching its fade wherever it happens to be.
        xdg.configFile."kitty/kitty.conf".text = lib.mkAfter "cursor_blink_interval 0";
        jitsusama.quickshell.reports = true;
        # No key repeats. A key the test presses is let go when the host gets
        # round to it, which on a busy host is past the delay a held key
        # repeats after, so a letter typed once arrived twice.
        xdg.configFile."niri/keyboard.kdl".text = lib.mkForce ''
          input {
              keyboard {
                  repeat-rate 0
              }
          }
        '';
        # What Quickshell hears from Pipewire about nodes and defaults, so a
        # change it missed can be told from one it was never sent. A node's
        # mute reaches it only as a note that its parameters changed, which
        # it answers by reading them again, at debug level, so a mute it
        # never heard of can be told from one it read and dropped.
        systemd.user.services.quickshell.Service.Environment = [
          "QT_LOGGING_RULES=${
            lib.concatStringsSep ";" (
              map (part: "quickshell.service.pipewire.${part}.info=true") [
                "node"
                "defaults"
              ]
              ++ [ "quickshell.service.pipewire.node.debug=true" ]
            )
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
        # The machine's niri, with its patches, rather than nixpkgs's.
        program = lib.getExe config.programs.niri.package;
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

      # Whether the screen holds still for half a second, twice the theme's
      # longest movement, before the time is up: niri composes a frame a
      # moment after it's asked to, windows and surfaces move as they come,
      # and a busy host can draw two frames in a row the same mid-motion.
      def held_still(timeout=10, span=0.5):
          deadline = time.monotonic() + timeout
          with tempfile.TemporaryDirectory() as folder:
              shot = os.path.join(folder, "shot.ppm")
              last, since = None, time.monotonic()
              while time.monotonic() < deadline:
                  machine.send_monitor_command(f"screendump {shot}")
                  with open(shot, "rb") as taken:
                      now = taken.read()
                  if now != last:
                      last, since = now, time.monotonic()
                  elif time.monotonic() - since >= span:
                      return True
          return False

      # A photograph of the screen once it holds still, for a person to look
      # at. One that never holds still is kept as it is.
      def photograph(name, timeout=10):
          held_still(timeout)
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
          # The picture to look at, once its fade has finished and the
          # microphone's meter, which can show as the session starts, has
          # gone.
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=30)
          photograph("workspace")

      with subtest("the overview draws nothing of the shell's, and the launcher says how things stand"):
          as_account("niri msg action open-overview")
          machine.wait_until_succeeds(
              in_session("niri msg --json overview-state | grep -q '\"is_open\":true'"), timeout=10
          )
          photograph("overview")
          machine.fail(in_session("niri msg layers | grep -qw status"))
          as_account("niri msg action close-overview")
          # The time and the battery's charge, at least, always.
          charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
          since = mark()
          as_account("qs ipc call launcher toggle")
          reported("stands ", lambda rest: re.match(r"\d\d:\d\d  ", rest) and f"{charge}%" in rest, since)
          launcher_ready()
          photograph("launcher-standing")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

      with subtest("a notification appears"):
          with told("drawn notifications"):
              as_account("notify-send 'Build finished' 'the laptop is ready'")
          photograph("notification")
          # It leaves on its own, before the meters, whose pictures it would
          # be in for as long as the host takes to reach them.
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)

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

      # The VM's light sensor is a stand-in whose light is what the test writes
      # to it, which it puts in the buffer udev turned on. Joel picks a dim
      # screen in a dim room; as the room brightens the screen follows it, and
      # the bar, which is for his picks, stays away.
      with subtest("the screen follows the light, and the bar shows only for picks"):
          named = machine.succeed("grep -lx als /sys/bus/iio/devices/*/name")
          sensor = named.strip().rsplit("/", 1)[0]
          machine.succeed(f"grep -qx 1 {sensor}/buffer/enable")

          def light(lux):
              machine.succeed(f"echo {lux * 1000} > {sensor}/in_illuminance_raw")

          def brightness():
              return int(machine.succeed("cat /sys/class/backlight/stand-in/brightness"))

          def learned(_):
              found = "grep -c '^2 0.100$' ~/.local/state/backlight/learned || true"
              return as_account(found).strip() == "1"

          light(2)
          with told("drawn brightness"):
              as_account("niri msg action spawn -- brightnessctl --class=backlight set 10%")
          retry(learned, timeout_seconds=10)
          machine.wait_until_fails(in_session("niri msg layers | grep -q brightness"), timeout=10)
          since = mark()
          light(30)
          retry(lambda _: brightness() > 15, timeout_seconds=30)
          after = f"--after-cursor={shlex.quote(since)}"
          shell = as_account(f"journalctl --user --unit=quickshell --no-pager --output=cat {after}")
          t.assertNotIn("drawn brightness", shell, "the bar showed for the light")

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
          photograph("launcher-volume")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

      with subtest("the launcher finds a pi session by its quest, and goes to its window"):
          # A window titled as pi titles its own: its address, its quest
          # cut short, its folder.
          agent = "\u03c0 - TESTQA A quest of its own - home"
          as_account(f"niri msg action spawn -- kitty --title '{agent}'")
          machine.wait_until_succeeds(
              in_session("niri msg --json windows | grep -q TESTQA"), timeout=30
          )
          others = [w for w in json.loads(as_account("niri msg --json windows")) if "TESTQA" not in w["title"]]
          as_account(f"niri msg action focus-window --id {others[0]['id']}")
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          # Its collection by what it's called, then the session by its
          # quest, named whole and not as its window's title.
          type_in("agents")
          launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
          machine.send_key("ret")
          launcher_until(
              lambda shown: shown["held"] == ["sessions"]
              and (shown["selected"] or {}).get("name") == "A quest of its own"
          )
          photograph("launcher-agents")
          machine.send_key("ret")
          machine.wait_until_succeeds(
              in_session("niri msg --json focused-window | grep -q TESTQA"), timeout=10
          )
          as_account(f"niri msg action close-window --id {window_titled(agent)['id']}")
          machine.wait_until_fails(in_session("niri msg --json windows | grep -q TESTQA"), timeout=30)

      with subtest("the launcher sets the volume to a number, and says how loud it is"):
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("volume 40")
          launcher_until(lambda shown: (shown["selected"] or {}).get("name", "").endswith(" to 40%"))
          machine.send_key("ret")
          machine.wait_until_succeeds(
              in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.40'"), timeout=10
          )
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("volume")
          launcher_until(lambda shown: (shown["selected"] or {}).get("name", "").startswith("40%, through "))
          photograph("launcher-volume-told")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)

      with subtest("the launcher finds and opens kitty, and learns that kitty was meant"):
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("kitty", selecting="kitty")
          photograph("launcher")
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
          photograph("launcher-window")
          machine.send_key("ret")
          machine.wait_until_succeeds(
              in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"), timeout=10
          )

      # Super+Tab opens the launcher on every other window, the one used last
      # first, so Return goes back to it, and a few letters find any other,
      # among two dozen as easily as among two, with nothing held down. The
      # windows are one kitty's, so the machine holds them all.
      with subtest("Super+Tab goes back to the window before, and finds any by typing"):
          # The last one's title is far wider than the launcher, as a
          # browser tab's can be.
          scratch = [f"Scratch {n:02}" for n in range(1, 25)] + [
              "Scratch 25 and a title that goes on well past the edge of the launcher"
              " and the screen, as the titles of web pages often do"
          ]
          for title in scratch:
              as_account(
                  "niri msg action spawn -- kitty --single-instance"
                  f" --instance-group scratch --title '{title}'"
              )
              machine.wait_until_succeeds(
                  in_session(f"niri msg windows | grep -q '{title}'"), timeout=30
              )
          notes = window_titled("Notes for Tuesday")
          as_account(f"niri msg action focus-window --id {notes['id']}")
          machine.wait_until_succeeds(
              in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"), timeout=10
          )
          closed = in_session("niri msg layers | grep -qw launcher")

          def tab_to(typed, title):
              machine.send_key("meta_l-tab")
              launcher_ready()
              launcher_until(lambda shown: shown["held"] == ["windows"])
              if typed:
                  type_in(typed, selecting=title)
              launcher_until(
                  lambda shown: shown["selected"] and shown["selected"]["name"] == title
              )

          def went(title):
              machine.send_key("ret")
              machine.wait_until_fails(closed, timeout=10)
              machine.wait_until_succeeds(
                  in_session(f"niri msg focused-window | grep -q '{title}'"), timeout=10
              )

          # The window used last is chosen as it opens, and this one isn't
          # there to choose.
          tab_to("", scratch[-1])
          photograph("launcher-windows")
          went("Scratch 25")
          tab_to("", "Notes for Tuesday")
          went("Notes for Tuesday")
          # A few letters find one among them all, and nothing looser.
          tab_to("scr 07", "Scratch 07")
          went("Scratch 07")
          # A program's name finds its windows, the one used last first.
          tab_to("kitty", "Notes for Tuesday")
          machine.send_key("esc")
          machine.wait_until_fails(closed, timeout=10)
          for title in scratch:
              as_account(f"niri msg action close-window --id {window_titled(title)['id']}")
          machine.wait_until_fails(in_session("niri msg windows | grep -q Scratch"), timeout=30)
          as_account(f"niri msg action focus-window --id {notes['id']}")

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
          photograph("launcher-verbs")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertEqual(as_account("wl-paste --no-newline"), "Notes for Tuesday")

          into_notes("move")
          step_in("ret")
          photograph("launcher-target")
          machine.send_key("ret")

          def moved(_):
              windows = json.loads(as_account("niri msg --json windows"))
              now = next(w for w in windows if w["id"] == notes["id"])
              return now["workspace_id"] != notes["workspace_id"]

          retry(moved, timeout_seconds=10)

      # Stepping back out of a collection finds the launcher as it was left,
      # on the row that was selected, not back at the top.
      with subtest("the launcher remembers where it was when stepping back"):
          as_account("qs ipc call launcher toggle")
          launcher_ready()

          def selected():
              return (showing()["selected"] or {}).get("name", "")

          # Down past the first row to a collection's.
          while True:
              before = selected()
              machine.send_key("down")
              launcher_until(lambda shown: (shown["selected"] or {}).get("name", "") != before)
              if showing()["selected"]["verb"] == "open":
                  break
          chosen = selected()
          step_in()
          machine.send_key("down")
          machine.send_key("left")
          launcher_until(
              lambda shown: shown["held"] == [] and (shown["selected"] or {}).get("name") == chosen
          )
          machine.send_key("esc")
          launcher_until(lambda shown: not shown["open"])

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
          # The battery's charge leads, as the kernel's stand-in has it.
          charge = machine.succeed("cat /sys/class/power_supply/test_battery/capacity").strip()
          answer("battery")
          photograph("launcher-battery")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertTrue(as_account("wl-paste --no-newline").startswith(f"{charge}%, "))
          # Then what feeds it: the stand-in's mains, plugged in.
          answer("charger")
          machine.send_key("down")
          launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "mains")
          machine.send_key("esc")
          # A collection is found by what a person calls it, not only its
          # title, and Return goes straight into it.
          # Beside the field, it says what sort of thing each is.
          for typed, collection in [("audio", "Sound"), ("notifications", "Notices")]:
              answer(typed)
              launcher_until(
                  lambda shown: (shown["selected"] or {}).get("name") == collection
                  and shown["kind"] == "collection"
              )
              machine.send_key("esc")
          # The hottest part, from the sensors the kernel lists, of which
          # the stand-in battery is one.
          print(machine.succeed("grep -H . /sys/class/hwmon/*/name"))
          answer("temperature")
          photograph("launcher-heat")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertRegex(as_account("wl-paste --no-newline"), r"^-?\d+ \u00b0C at the ")
          # The date in full, as the machine's clock has it; the day may
          # turn while it's asked, so either will do.
          before = machine.succeed("date '+%A %-d %B %Y'").strip()
          answer("date")
          photograph("launcher-date")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          after = machine.succeed("date '+%A %-d %B %Y'").strip()
          t.assertIn(as_account("wl-paste --no-newline"), [before, after])
          # The stand-in backlight's share, as its files say.
          now, most = (
              int(machine.succeed(f"cat /sys/class/backlight/stand-in/{f}"))
              for f in ("brightness", "max_brightness")
          )
          # Rounded half up, as JavaScript's Math.round does.
          copied("brightness", f"{int(now * 100 / most + 0.5)}% bright")
          # With a number, Return sets it, as the brightness keys do.
          answer("brightness 30")
          launcher_until(
              lambda shown: (shown["selected"] or {}).get("name") == "set the screen to 30%"
          )
          machine.send_key("ret")
          machine.wait_until_succeeds(
              f"test $(cat /sys/class/backlight/stand-in/brightness) -eq {most * 30 // 100}", timeout=10
          )
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          # The session's actions answer to what people call them; Escape,
          # never Return, since Return would do them.
          for typed, action in [("shut down", "Power Off"), ("restart", "Reboot"), ("sign out", "Log Out")]:
              answer(typed)
              launcher_until(
                  lambda shown: (shown["selected"] or {}).get("name") == action
                  and shown["kind"] == "session"
              )
              machine.send_key("esc")
              machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          # The keyboards QEMU gives it, however many, and caps lock.
          answer("keyboards")
          photograph("launcher-keyboards")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          t.assertRegex(
              as_account("wl-paste --no-newline"), r"^(one keyboard|\d+ keyboards), caps lock off$"
          )
          answer("350 f in c")
          photograph("launcher-units")
          machine.send_key("esc")
          # The zones come from the list built from tzdata, so this proves it's
          # there. The minute may turn while it's asked, so either will do.
          before = machine.succeed("date -u +%H:%M").strip()
          answer("time in utc")
          photograph("launcher-time")
          machine.send_key("ret")
          machine.wait_until_fails(in_session("niri msg layers | grep -qw launcher"), timeout=10)
          after = machine.succeed("date -u +%H:%M").strip()
          t.assertIn(as_account("wl-paste --no-newline"), [before, after])
          # A theme's colour is found by its role, as the theme exports it.
          theme = json.loads(as_account("cat ~/.config/jitsusama/theme.json"))
          copied("colour accent", theme["roles"]["accent"].lower())
          # Slack takes its theme only by hand, so the launcher copies it,
          # in the theme's own roles.
          slack = ["deepest", "raised", "selection", "strong", "raised", "text"]
          slack += ["success", "alert", "deepest", "text"]
          copied("slack theme", ",".join(theme["roles"][role] for role in slack))
          answer("#f53")
          photograph("launcher-colour")
          machine.send_key("esc")
          answer("10.0.0.0/22")
          photograph("launcher-answer")
          machine.send_key("esc")
          # With no network, what has to ask one says it couldn't, and nothing
          # breaks. Each waits for its answer to settle, failed; asked again
          # while that failure stands, it says so at once. When a failure is
          # asked again is the launcher's answers check's to say.
          for typed in ["8.8.8.8", "weather tokyo", "100 usd in cad", "ip"]:
              answer(typed)
              machine.send_key("esc")
          answer("weather tokyo")
          photograph("launcher-offline")
          machine.send_key("esc")
          log = as_account("journalctl --user -u quickshell --no-pager -o cat")
          assert "ERROR" not in log and "Error:" not in log, "an answer raised an error"

      with subtest("the launcher finds emoji as one row, and steps into the rest"):
          copied("tada", "\U0001F389")
          answer("smiling")
          photograph("launcher-emoji")
          step_in()
          type_in(" cat")
          photograph("launcher-emoji-all")
          machine.send_key("ret")
          closed = in_session("niri msg layers | grep -qw launcher")
          machine.wait_until_fails(closed, timeout=10)
          # The grinning cat with smiling eyes, first among the smiling cats.
          t.assertEqual(as_account("wl-paste --no-newline"), "\U0001F638")
          # Super+E opens the emoji straight away, so "kitty" is a cat there,
          # where the main search would open the terminal.
          machine.send_key("meta_l-e")
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
          photograph("launcher-shortcuts")
          for _ in "paste":
              machine.send_key("backspace")
          # The touchscreen's swipes are among them, found by "swipe", each
          # drawn as its fingers and then its way.
          swipe = "swipe workspace below"
          type_in(swipe, selecting="Go to the workspace below")
          t.assertTrue(showing()["selected"]["chord"].startswith("3 "))
          photograph("launcher-swipes")
          for _ in swipe:
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
          photograph("launcher-copies-all")
          type_in("tues", selecting="Notes for Tuesday")
          photograph("launcher-copies")
          machine.send_key("ret")

          def copied_again(_):
              return as_account("wl-paste --no-newline") == "Notes for Tuesday"

          retry(copied_again, timeout_seconds=10)

      # A copy from a program that then closes: wl-copy in the foreground
      # serves it until it's stopped. It's stopped once the clipboard hands
      # the copy back, not after a guess: a two-second guess once ended a
      # copy the history never heard.
      def copied_then_closed(text, flags="", served=None):
          served = served or f"[ \"$(wl-paste --no-newline 2>/dev/null)\" = '{text}' ]"
          as_account(
              f"echo -n '{text}' | wl-copy --foreground {flags} > /dev/null 2>&1 & p=$!; "
              f"for i in $(seq 100); do {served} && break; sleep 0.1; done; kill $p"
          )

      with subtest("a copy outlives the program that made it, unless it wasn't kept"):
          with told("heard copy gone with its program"):
              copied_then_closed("gone with its program")

          # Empty for a moment between its program going and the helper's
          # offer, which the retry waits out.
          def still_there(_):
              return as_account("wl-paste --no-newline 2>&1 || true") == "gone with its program"

          retry(still_there, timeout_seconds=10)
          # One the history didn't keep, such as a password manager's, goes
          # with its program, leaving the clipboard empty rather than handing
          # back an older copy. The helper answers the moment it empties, so
          # empty twice over is empty for good.
          copied_then_closed(
              "a password",
              flags="--type x-kde-passwordManagerHint",
              served="wl-paste --list-types 2>/dev/null | grep -qx x-kde-passwordManagerHint",
          )
          machine.wait_until_fails(in_session("wl-paste --list-types > /dev/null 2>&1"), timeout=10)
          machine.fail(in_session("wl-paste --list-types > /dev/null 2>&1"))

      with subtest("a copy made as another's program closes is the one kept"):
          # The helper reads one event at a time, so the clipboard emptying
          # can wait behind a newer copy, as it does while a slow program
          # hands over what it copied. Paused, it hears both late.
          with told("heard copy closing as another copies"):
              as_account(
                  "systemd-run --user --unit=closing sh -c "
                  "\"printf %s 'closing as another copies' | wl-copy --foreground\""
              )
          as_account("systemctl --user kill --signal=SIGSTOP clipboard-history")
          as_account("systemctl --user stop closing")
          as_account("echo -n 'copied meanwhile' | wl-copy > /dev/null 2>&1")
          with told("heard copy copied meanwhile"):
              as_account("systemctl --user kill --signal=SIGCONT clipboard-history")
          # Nothing takes it back a moment later.
          machine.sleep(1)
          t.assertEqual(as_account("wl-paste --no-newline"), "copied meanwhile")

      def pasted_the_picture(_):
          pasted = in_session("wl-paste --type image/png | cmp -s - ${picture}")
          return machine.execute(pasted)[0] == 0

      with subtest("a picture copied is kept, shown as it is, and copied again"):
          with told("heard copy Image 256 \u00d7 256"):
              as_account("wl-copy --type image/png < ${picture} > /dev/null 2>&1")
          with told("heard copy something after the picture"):
              as_account("echo -n 'something after the picture' | wl-copy > /dev/null 2>&1")
          machine.send_key("meta_l-v")
          launcher_ready()
          with told("showed Image 256 \u00d7 256"):
              type_in("image", selecting="Image 256 \u00d7 256")
          photograph("launcher-copies-picture")
          machine.send_key("ret")
          retry(pasted_the_picture, timeout_seconds=10)

      with subtest("a picture outlives the program that copied it"):
          # The helper holds the same picture already, so wl-copy serves it
          # once the clipboard's types no longer include the helper's own.
          with told("heard copy Image 256 \u00d7 256"):
              as_account(
                  "wl-copy --foreground --type image/png < ${picture} > /dev/null 2>&1 & p=$!; "
                  "for i in $(seq 100); do t=$(wl-paste --list-types 2>/dev/null); "
                  "echo \"$t\" | grep -qx image/png && "
                  "! echo \"$t\" | grep -qx application/x-clipboard-history-kept && break; "
                  "sleep 0.1; done; kill $p"
              )
          retry(pasted_the_picture, timeout_seconds=10)

      with subtest("Forget the Clipboard forgets every copy, and what it held"):
          with told("heard copy kept until forgotten"):
              copied_then_closed("kept until forgotten")

          def still_kept(_):
              return as_account("wl-paste --no-newline 2>&1 || true") == "kept until forgotten"

          retry(still_kept, timeout_seconds=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("forget the clip", selecting="Forget the Clipboard")
          photograph("launcher-forget-copies")
          with told("forgot the copies"):
              machine.send_key("ret")
          t.assertEqual(as_account("wl-paste --no-newline 2>&1 || true"), "")
          # The pictures' files go with them.
          t.assertEqual(as_account("ls -A $XDG_RUNTIME_DIR/clipboard-history"), "")

      with subtest("the launcher's home shows each collection, and opens it"):
          windows = len(json.loads(as_account("niri msg --json windows")))
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          photograph("launcher-home")
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

      # Opens a file as a click on it would, and waits for the program meant
      # for it to take the focus. A program's first start here, drawn in
      # software, has taken 17 seconds on a busy host. If none does, it says
      # what would open the file, what opening it said, and what's open.
      def opens_in(path, kind, app_id):
          as_account(f"niri msg action spawn -- sh -c 'xdg-open {path} > /tmp/opened 2>&1'")
          try:
              machine.wait_until_succeeds(
                  in_session(f"niri msg --json focused-window | grep -q '\"app_id\":\"{app_id}\"'"),
                  timeout=60,
              )
          except Exception:
              print(as_account(f"xdg-mime query default {kind}; cat /tmp/opened; niri msg windows"))
              raise

      with subtest("a picture opens floating in swayimg, and e takes it to GIMP"):
          # Beside the screenshots, never among them, since the region
          # screenshot's subtest waits for a picture there of its own.
          looked = "/home/${account}/snowflake.png"
          as_account(f"cp ${picture} {looked}")
          opens_in(looked, "image/png", "swayimg")
          swayimg = window_titled("Swayimg: snowflake.png")
          assert swayimg["is_floating"], swayimg
          # Four fifths of the screen, centred, at the size niri gave it,
          # less the gaps and border niri takes out of it. Without swayimg's
          # size given again, it's twice that, off the screen.
          left, top, width, height = drawn_at(swayimg)
          wide = json.loads(as_account("niri msg --json focused-output"))["logical"]["width"]
          assert abs(width - wide * 0.8) <= wide * 0.02, swayimg
          assert abs(left + width / 2 - wide / 2) <= wide * 0.02, swayimg
          # The steps before leave notices, which go on their own.
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          photograph("swayimg")
          machine.send_key("e")
          machine.wait_until_succeeds(f"pgrep -u ${account} -f '[g]imp.*{looked}'", timeout=30)
          machine.wait_until_fails("pgrep -u ${account} -x swayimg", timeout=10)
          # As in the screenshot's subtest: GIMP goes before it draws.
          machine.succeed("pkill -KILL -u ${account} -f '[g]imp'")
          machine.wait_until_fails("pgrep -u ${account} -f '[g]imp'", timeout=30)
          machine.succeed(f"rm {looked}")

      with subtest("a document opens in zathura, beside the rest, and q closes it"):
          opens_in("${page}", "application/pdf", "org.pwmt.zathura")
          windows = json.loads(as_account("niri msg --json windows"))
          zathura = next(window for window in windows if window["app_id"] == "org.pwmt.zathura")
          assert not zathura["is_floating"], zathura
          photograph("zathura")
          machine.send_key("q")
          machine.wait_until_fails("pgrep -u ${account} -f '[z]athura'", timeout=10)

      with subtest("the launcher's Sound moves sound to another output, and back"):
          for output in ["stand-in-headphones", "stand-in-speakers"]:
              as_account("qs ipc call launcher collection sound")
              launcher_ready()
              photograph(f"launcher-sound-{output}")
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
              photograph("launcher-displays")
              type_in(f"scale {round(wanted * 100)}")
              machine.send_key("ret")
              retry(lambda _, wanted=wanted: scale() == wanted, timeout_seconds=15)
              machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)

      with subtest("a Wi-Fi network is joined from the launcher, a new one asking its password"):
          machine.wait_for_unit("lantern.service")
          machine.wait_for_unit("wifid.service")
          wifi = "varlinkctl call /run/wifid/io.jitsusama.Wifi io.jitsusama.Wifi"
          machine.wait_until_succeeds(
              f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Lantern\"'", timeout=60
          )
          as_account("qs ipc call launcher collection wifi")
          launcher_ready()
          photograph("launcher-wifi")
          type_in("lant", selecting="Lantern")
          # Never joined, it asks for its password, which a wrong one doesn't
          # satisfy; the launcher stays to say so.
          step_in("ret")
          type_in("not the password")
          photograph("launcher-wifi-password")
          with told("refused wifi Lantern", timeout=60):
              machine.send_key("ret")
          machine.succeed("journalctl -u wifid | grep -q \"couldn't join Lantern: WrongPassword\"")
          photograph("launcher-wifi-refused")
          machine.succeed(in_session("niri msg layers | grep -q launcher"))
          type_in("glowing in the dark")
          machine.send_key("ret")
          # Joined means an address too, and wifid says so once it's checked
          # how far the network reaches.
          machine.wait_until_succeeds("journalctl -u wifid | grep -q 'joined Lantern on wlan1'", timeout=60)
          machine.succeed("networkctl status wlan1 | grep -q 10.42.0.")
          # Once it's joined, the launcher's done.
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # Lantern wants a sign-in, so Sign In to it comes first at home, and
          # Return opens the window on its front desk.
          described = "varlinkctl --json=short call /run/wifid/io.jitsusama.Wifi"
          described += " io.jitsusama.Wifi.Describe '{}'"
          machine.wait_until_succeeds(
              f"{described} | grep -qF '\"reach\":\"signIn\"'", timeout=60
          )
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          photograph("launcher-sign-in")
          type_in("sign in", selecting="Sign In to Lantern")
          machine.send_key("ret")
          window = "niri msg --json windows | grep -qF '\"app_id\":\"io.jitsusama.WifiSignIn\"'"
          machine.wait_until_succeeds(in_session(window), timeout=60)
          # Its button can only be pressed once the front desk has served
          # the page, which a window that has to try again does later.
          machine.wait_until_succeeds(
              "journalctl -u lantern-front-desk | grep -q 'GET /sign-in'", timeout=60
          )
          photograph("sign-in", timeout=20)
          # Its one button signs in, and the window closes as Lantern lets
          # the machine through, its notice taken back with it.
          with told("withdrew sign-in Lantern", timeout=60):
              machine.send_key("ret")
          machine.wait_until_fails(in_session(window), timeout=60)
          machine.succeed(f"{described} | grep -qF '\"reach\":\"online\"'")
          # The Office asks for an account: met for the first time, it asks
          # the account's name, its server's domain and its password in
          # turn, and joins.
          machine.wait_until_succeeds(
              f"{wifi}.Scan '{{}}' && {wifi}.Describe '{{}}' | grep -q '\"Office\"'", timeout=60
          )
          as_account("qs ipc call launcher collection wifi")
          launcher_ready()
          type_in("office", selecting="Office")
          step_in("ret")
          type_in("joel")
          step_in("ret")
          type_in("radius.office.test")
          step_in("ret")
          type_in("hunter22")
          photograph("launcher-wifi-account")
          machine.send_key("ret")
          machine.wait_until_succeeds("journalctl -u wifid | grep -q 'joined Office on wlan1'", timeout=60)
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # Stepped into, a network says after what can be done to it what's
          # heard of it, a row each, the selected row's footer saying more:
          # the card's link to it once the card has read it, and its access
          # points a step further in.
          as_account("qs ipc call launcher collection wifi")
          launcher_ready()
          type_in("office", selecting="Office")
          step_in()
          type_in("out", selecting="Mb/s in")
          launcher_until(lambda shown: "over the noise" in shown["footer"])
          for _ in "out":
              machine.send_key("backspace")
          launcher_until(lambda shown: shown["typed"] == "")
          photograph("launcher-wifi-detail")
          type_in("access", selecting="1 access point")
          step_in()
          launcher_until(lambda shown: shown["footer"].startswith("02:00:00:00:02:00 \u00b7 "))
          assert showing()["selected"]["name"].startswith("2.4 GHz ch 1\u00a0"), showing()
          photograph("launcher-wifi-points")
          machine.send_key("esc")
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # Turning Wi-Fi off is an action like any.
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("wifi off")
          machine.send_key("ret")
          machine.wait_until_succeeds("rfkill list wifi | grep -q 'Soft blocked: yes'", timeout=15)
          machine.succeed("rfkill unblock wifi")

      with subtest("a notice that came and went is found again in the launcher"):
          # One its sender says not to keep isn't, though it's the newest match.
          as_account("notify-send --transient 'Build finished twice' 'not kept'")
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          # Its row at home counts the one missed earlier, since it left by itself.
          photograph("launcher-home-missed")
          type_in("build fin", selecting="the laptop is ready")
          photograph("launcher-notices")
          # Nothing sent it that can be gone to, so Return copies what it said.
          machine.send_key("ret")

          def told_again(_):
              said = as_account("wl-paste --no-newline")
              return said == "Build finished\nthe laptop is ready"

          retry(told_again, timeout_seconds=10)
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # With none on screen, Super+N opens all of them, which counts
          # them seen.
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          machine.send_key("meta_l-n")
          launcher_ready()
          photograph("launcher-notices-all")
          machine.send_key("esc")

      # What the notification server says to programs, as they hear it.
      def listen():
          as_account(
              "${pkgs.dbus}/bin/dbus-monitor --session"
              " \"type='signal',interface='org.freedesktop.Notifications'\""
              " > /tmp/signals 2>&1 < /dev/null & echo $! > /tmp/listening"
          )
          # It's listening once it hears a signal of its own.
          probe = (
              "${pkgs.dbus}/bin/dbus-send --session --type=signal"
              " /probe org.freedesktop.Notifications.Probe"
          )
          machine.wait_until_succeeds(
              in_session(f"{probe}; grep -q member=Probe /tmp/signals"), timeout=10
          )

      # The program was sent a token to bring its window forward, and then
      # told what was chosen: the order a compositor needs.
      def tokened():
          said = machine.succeed("cat /tmp/signals")
          machine.succeed("kill $(cat /tmp/listening)")
          token = re.search(r"member=ActivationToken\n\s+uint32 \d+\n\s+string \"([^\"]+)\"", said)
          t.assertIsNotNone(token, said)
          t.assertLess(said.index("member=ActivationToken"), said.index("member=ActionInvoked"), said)

      with subtest("a notice's action is done by the keys, and one not acted on stays unread"):
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          # It stays a minute, so it's still at the front when the keys come.
          with told("shows card Lunch"):
              as_account("notify-send --expire-time=60000 'Lunch' 'at noon'")
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
          # Super+N takes the stack, which waits while it holds the keys.
          # The newer notice waits behind the older; Down brings it to the
          # front, the right arrow chooses what it offers, and Return does
          # it, with a token to come forward.
          listen()
          with told("keys notices"):
              machine.send_key("meta_l-n")
          photograph("notices-held")
          machine.send_key("down")
          machine.send_key("right")
          photograph("notices-held-offer")
          machine.send_key("ret")
          machine.wait_until_succeeds("grep -qx play /tmp/answer", timeout=15)
          tokened()
          # Shift+Backspace lets the rest go, which gives the keys back.
          with told("lets notices go"):
              machine.send_key("shift-backspace")
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=10)
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
      def offer_to_play(picture, offers=""):
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          with told(f"shows card {picture}"):
              as_account(
                  f"notify-send --expire-time=60000 {offers} --action=play='Play it' '{picture}' 'y.mp4'"
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

      with subtest("a click on a notice opens what it's about, with a token to come forward"):
          listen()
          # Its default is the card's click, so the one glyph is Play it's.
          x, y = offer_to_play("notice-to-open", offers="--action=default=Open")
          click(x, y - 0.04)
          assert answered() == "default", "a click on the message didn't open what it's about"
          tokened()

      with subtest("Return on a notice in the launcher opens what it's about, with a token"):
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          listen()
          with told("shows card Thread with Sam"):
              as_account(
                  "notify-send --expire-time=60000 --action=default=Open 'Thread with Sam' 'see it'"
                  " > /tmp/answer 2>&1 < /dev/null &"
              )
          # Super+N takes the stack, and again hands on to the history.
          with told("keys notices"):
              machine.send_key("meta_l-n")
          machine.send_key("meta_l-n")
          launcher_ready()
          launcher_until(
              lambda shown: (shown["selected"] or {}).get("name", "").startswith("Thread with Sam")
          )
          machine.send_key("ret")
          machine.wait_until_succeeds("grep -qx default /tmp/answer", timeout=15)
          tokened()

      with subtest("the notices sit a step inside the window under them"):
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
          # A window of its own, the only one in view, two thirds of the
          # screen wide and centred, so its edge is far from the corner's;
          # it goes again after.
          as_account("niri msg action spawn -- kitty --title 'Under the notices'")
          machine.wait_until_succeeds(
              in_session("niri msg --json windows | grep -q '\"Under the notices\"'"), timeout=30
          )
          under = window_titled("Under the notices")
          as_account(f"niri msg action focus-window --id {under['id']}")
          screen = json.loads(as_account("niri msg --json focused-output"))["logical"]["width"]
          as_account(f"niri msg action set-column-width {int(screen * 2 / 3)}")
          as_account("niri msg action center-column")
          space = json.loads(as_account("cat ~/.config/jitsusama/theme.json"))["space"]

          # Where it will be once the view rests, from niri.
          def settled(last):
              layout = window_titled("Under the notices")["layout"]
              place = layout["tile_pos_in_workspace_view"]
              if last:
                  t.assertIsNotNone(place, "niri doesn't say where a tiled window is")
              return place is not None and screen - (place[0] + layout["tile_size"][0]) > 100

          retry(settled, timeout_seconds=15)
          with told("shows card Inside the window"):
              as_account("notify-send --expire-time=60000 'Inside the window' 'a step from its edges'")
          photograph("notice-inside")
          # The nearest window wholly on screen, as the shell picks it.
          ws = window_titled("Under the notices")["workspace_id"]
          tiles = [
              (w["layout"]["tile_pos_in_workspace_view"], w["layout"]["tile_size"])
              for w in json.loads(as_account("niri msg --json windows"))
              if w["workspace_id"] == ws and w["layout"]["tile_pos_in_workspace_view"]
          ]
          whole = [(p, s) for p, s in tiles if p[0] >= 0 and p[0] + s[0] <= screen]
          (left, top), (width, _) = sorted(whole, key=lambda t: (-(t[0][0] + t[1][0]), t[0][1]))[0]
          step = space["s"]
          wanted = [
              int(screen - (left + width) + step + 0.5),
              int(max(top, 0) + step + 0.5),
              min(380, int(width - 2 * step)),
          ]
          placed = as_account(
              "journalctl --user --unit=quickshell --no-pager --output=cat"
              " | grep -o 'report\\[places notices [0-9 ]*' | tail -n 1"
          ).split()[-3:]
          t.assertEqual([int(n) for n in placed], wanted, "the notices aren't a step inside the window")
          as_account("qs ipc call notices putAway")
          as_account(f"niri msg action close-window --id {under['id']}")
          machine.wait_until_fails(
              in_session("niri msg --json windows | grep -q '\"Under the notices\"'"), timeout=30
          )

      with subtest("the launcher keeps the machine awake, lid shut too, then lets it sleep"):
          held = "systemd-inhibit --list --no-pager | grep Caffeinate"
          machine.wait_until_fails(in_session("niri msg layers | grep -q launcher"), timeout=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          type_in("keep awake")
          photograph("launcher-keep-awake")
          machine.send_key("ret")
          machine.wait_until_succeeds(f"{held} | grep -qw idle", timeout=10)
          machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
          # Kept awake, a suspend asked for by the power key or the launcher
          # still goes through, and so does the lid's on battery.
          machine.fail(f"{held} | grep -q sleep")
          # Without power the lid is let go, and taken again when it's back.
          power("off")
          machine.wait_until_fails(f"{held} | grep -q handle-lid-switch", timeout=10)
          machine.succeed(f"{held} | grep -qw idle")
          power("on")
          machine.wait_until_succeeds(f"{held} | grep -q handle-lid-switch", timeout=10)
          as_account("qs ipc call launcher toggle")
          launcher_ready()
          photograph("launcher-home-awake")
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
          photograph("launcher-held")
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
          photograph("launcher-from-margin")
          machine.send_key("esc")
          # Opening them counted them seen, so nothing's waiting.
          machine.wait_until_fails(in_session(margin), timeout=10)

      with subtest("the microphone muted shows in the margin, and Return unmutes it"):
          try:
              with told("drawn margin"):
                  as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
          except AssertionError:
              # Which source Pipewire calls the default, whether it's muted,
              # and what Pipewire holds for it, beside what Quickshell heard.
              print(as_account("wpctl status; wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
              print(as_account("pw-dump stand-in-microphone"))
              raise
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)
          with told("keys margin"):
              machine.send_key("meta_l-o")
          photograph("margin-muted")
          machine.send_key("ret")
          muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
          machine.wait_until_fails(in_session(muted), timeout=10)
          machine.wait_until_fails(in_session(margin), timeout=10)
          # Unmuting shows the meter a moment, which the next pictures
          # would hold for as long as the host takes to reach them.
          machine.wait_until_fails(in_session("niri msg layers | grep -q microphone"), timeout=10)

      # Any player that tells the desktop what it plays shows in the margin
      # while it plays, and the media keys' command shows what it is.
      with subtest("what's playing shows in the margin, and a media key shows what it is"):
          machine.wait_until_fails(in_session(margin), timeout=10)
          # A track that starts by itself shows its card a moment, which
          # goes before the margin is photographed.
          with told("drawn notifications"), told("drawn margin"):
              as_account(
                  "systemd-run --user --unit=player mpv --ao=null --no-video --loop ${track}"
              )
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
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
          # new log ends the call. Super+N takes the stack, and again opens
          # the history on the newest notice's offers.
          with told("keys notices"):
              machine.send_key("meta_l-n")
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
          photograph("margin-capitals")
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
          t.assertEqual(showing()["selected"]["chord"], "Alt Print")
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

      with subtest("Capture records the window, and says why when niri can't share it"):
          as_account("niri msg action spawn -- kitty --title Tidepool")
          machine.wait_until_succeeds(
              in_session("niri msg --json windows | grep -qF '\"title\":\"Tidepool\"'"), timeout=60
          )
          capture()
          type_in("record this window", selecting="Record this window \u00b7 Tidepool")
          t.assertEqual(showing()["selected"]["chord"], "Ctrl Alt Print")
          # niri here offers no screencast on the session bus, so the
          # recording can't start, and the notice says why rather than it
          # failing unseen.
          with told("shows card The window could not be recorded"):
              machine.send_key("ret")
          kept = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
          print(machine.wait_until_succeeds(
              in_session(kept + " | grep -o '\"summary\":\"The window could not[^}]*'"), timeout=10
          ))
          windows = json.loads(as_account("niri msg --json windows"))
          pool = next(w for w in windows if w["title"] == "Tidepool")
          as_account(f"niri msg action close-window --id {pool['id']}")

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
          # The pictures before leave their notices, which go on their own.
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
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
          with told("picker typed b"):
              machine.send_chars("aab")
          machine.wait_until_fails(in_session("niri msg layers | grep -q notifications"), timeout=30)
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

          # A key that stops the recording, then the notice that it's saved.
          # A run has gone a minute without it, so what the recording was
          # doing then is printed.
          def saved(key):
              try:
                  with told("shows card Video saved", timeout=60):
                      machine.send_key(key)
              except AssertionError:
                  print(machine.execute("ps -eww -o pid,stat,wchan:32,args | grep -aE '[f]fmpeg|[m]pv'")[1])
                  print(machine.execute("ls -la /run/user/1000/camera-*; cat /run/user/1000/camera-recording.log")[1])
                  print(as_account("pw-top -b -n 2 || true"))
                  print(as_account("journalctl --user --no-pager -n 60 -o cat"))
                  raise

          # Tab makes Return record, and Return again stops it.
          machine.send_key("tab")
          machine.send_key("ret")
          retry(lambda last: recording(1, last), timeout_seconds=60)
          saved("ret")
          # Escape while recording saves the video, then closes.
          machine.wait_until_succeeds(in_session(camera), timeout=10)
          machine.send_key("ret")
          retry(lambda last: recording(2, last), timeout_seconds=60)
          saved("esc")
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
          with told("released lock"):
              machine.send_chars("${secret}\n")
          as_account("qs ipc call lock isLocked | grep -q false")

      # What locks before the machine sleeps returns only once niri has drawn
      # the lock on every screen, so nothing else shows on waking.
      with subtest("lock-screen returns once the lock is drawn"):
          marker = "$XDG_RUNTIME_DIR/quickshell-locked"
          as_account(f"grep -qx unlocked {marker}")
          as_account("lock-screen")
          as_account(f"grep -qx locked {marker}")
          with told("released lock"):
              machine.send_chars("${secret}\n")
          as_account(f"grep -qx unlocked {marker}")

      with subtest("polkit's prompt lets the account start a system service"):
          with told("keys polkit"):
              as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
          photograph("polkit")
          machine.send_chars("${secret}\n")
          machine.wait_for_file("/run/polkit-allowed", timeout=30)

      # ssh-tpm-agent asks before a key confirmed on use signs, and for a
      # key's PIN, and ssh for a security key's touch, all through the
      # shell's askpass, which answers as OpenSSH's contract says.
      with subtest("the askpass asks for a yes, a PIN and a touch"):
          # Started by the account's systemd, as ssh-tpm-agent is, so it has
          # what the agent has.
          def ask(kind, prompt):
              as_account(f"systemd-run --user --collect --quiet -- ${asking} {kind} '{prompt}'")

          def answered(kind, code, answer="", timeout=30):
              exit = f"$XDG_RUNTIME_DIR/asked-{kind}.exit"
              machine.wait_until_succeeds(in_session(f"test -e {exit}"), timeout=timeout)
              t.assertEqual(as_account(f"cat {exit}").strip(), str(code))
              t.assertEqual(as_account(f"cat $XDG_RUNTIME_DIR/asked-{kind}.answer"), answer)

          signing = (
              "Allow use of key joel-signing?\\nKey fingerprint SHA256:abc."
              "\\nRequested by ssh-keygen (1234) \u2190 tpm-sign \u2190 git \u2190 zsh"
          )
          with told("keys askpass"):
              ask("confirm", signing)
          photograph("askpass")
          machine.send_key("ret")
          answered("confirm", 0)

          with told("keys askpass"):
              ask("confirm", signing)
          with told("gone askpass"):
              machine.send_key("esc")
          answered("confirm", 1)

          with told("keys askpass"):
              ask("secret", "Enter PIN for ED25519-SK key /home/${account}/.ssh/id_ed25519_sk: ")
          photograph("askpass-pin")
          machine.send_chars("2468\n")
          answered("secret", 0, "2468\n")

          # A touch takes no keys, and goes once ssh stops waiting on it.
          with told("drawn askpass"):
              ask("none", "Confirm user presence for key ED25519-SK SHA256:abc")
          with told("gone askpass"):
              as_account("pkill -TERM -f '^[^ ]*bash [^ ]*ssh-askpass Confirm'")
          answered("none", 1)
          as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")

      # The shell holds an ask only in memory, as a switch restarts it.
      with subtest("the askpass asks a restarted shell again, and gives up on a gone one"):
          with told("keys askpass"):
              ask("confirm", signing)
          with told("keys askpass", timeout=60):
              as_account("systemctl --user restart quickshell")
          machine.send_key("ret")
          answered("confirm", 0)

          with told("keys askpass"):
              ask("confirm", signing)
          as_account("systemctl --user stop quickshell")
          answered("confirm", 1, timeout=60)
          as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")
          as_account("systemctl --user start quickshell")
          machine.wait_until_succeeds(in_session("qs ipc show | grep -q askpass"), timeout=60)

      # A switch restarts the shell, and can come while Joel is away and the
      # screens are locked. niri holds a lock whose shell died, so the shell
      # it starts again has to take that lock back, or nothing is left to
      # type the password into.
      with subtest("the lock holds when the shell restarts, and only for its niri"):
          locked = "$XDG_RUNTIME_DIR/quickshell-locked"
          holder = "$XDG_RUNTIME_DIR/quickshell-lock-holder"
          as_account("lock-screen")
          since = mark()
          as_account("systemctl --user restart quickshell")
          for word in ("secured lock", "keys lock"):
              reported(word, lambda rest: rest == "", since, timeout=60)
          as_account("qs ipc call lock isLocked | grep -qx true")
          as_account(f"grep -qx locked {locked}")
          with told("released lock"):
              machine.send_chars("${secret}\n")
          as_account(f"grep -qx unlocked {locked}")

          def started_unlocked():
              as_account("systemctl --user restart quickshell")
              machine.wait_until_succeeds(in_session("qs ipc show | grep -q askpass"), timeout=60)
              as_account("qs ipc call lock isLocked | grep -qx false")
              as_account(f"grep -qx unlocked {locked}")

          # Let go, the lock isn't taken again.
          started_unlocked()
          # The runtime directory outlives a session while the account
          # lingers, so a lock a crashed niri held is left to that niri.
          as_account(f"echo $XDG_RUNTIME_DIR/niri.wayland-1.1.sock > {holder}")
          as_account(f"echo locked > {locked}")
          started_unlocked()

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

      # Swipes on a touchscreen move through the desktop as the touchpad's do,
      # and never reach the window under them as a tap. wev says every touch
      # its window gets.
      with subtest("swipes on the touchscreen move through the workspaces, not the window"):
          machine.succeed("systemd-run --unit=touchscreen ${touchscreen}")
          machine.wait_for_file("/run/touchscreen", timeout=30)
          as_account(
              "systemd-run --user --collect --quiet --unit=wev -- sh -c "
              "'${pkgs.coreutils}/bin/stdbuf -oL ${pkgs.wev}/bin/wev > /tmp/wev.log 2>&1'"
          )

          def wev_open(_):
              windows = json.loads(as_account("niri msg --json windows"))
              return any(window["title"] == "wev" for window in windows)

          retry(wev_open, timeout_seconds=30)
          as_account("niri msg action maximize-column")

          def touches(kind):
              return int(machine.succeed(f"grep -c 'wl_touch] {kind}' /tmp/wev.log || true"))

          def focused_workspace():
              workspaces = json.loads(as_account("niri msg --json workspaces"))
              return next(w["idx"] for w in workspaces if w["is_focused"])

          def overview_open():
              return json.loads(as_account("niri msg --json overview-state"))["is_open"]

          # niri nested in Cage says its screen is Flipped180, and touches
          # reach it upside down, so the stand-in swipes up and down the way a
          # person seeing it would.
          outputs = json.loads(as_account("niri msg --json outputs"))
          turn = next(iter(outputs.values()))["logical"]["transform"]
          assert turn in ("Normal", "Flipped180"), f"no swipes for a screen turned {turn}"
          over = {"DU": "UD", "UD": "DU"} if turn == "Flipped180" else {}

          def swiped(fingers, way, until, across=0.5):
              drawn = over.get(way, way)
              machine.succeed(f"echo '{fingers} {drawn} {across}' > /run/touchscreen")
              try:
                  retry(lambda _: until(), timeout_seconds=10)
              except Exception:
                  print(machine.succeed("cat /tmp/wev.log | grep wl_touch || true"))
                  raise

          # One finger is the window's: wev hears it touched and let go.
          swiped(1, "tap", lambda: touches("up") == 1)
          first = focused_workspace()
          swiped(3, "DU", lambda: focused_workspace() == first + 1)
          # The two fingers wev already had are cancelled, not let go.
          retry(lambda _: touches("cancel") >= 1, timeout_seconds=10)
          swiped(3, "UD", lambda: focused_workspace() == first)
          swiped(4, "DU", overview_open)
          swiped(4, "UD", lambda: not overview_open())

          # A second window opens in a column to the right; the view follows
          # the fingers back to the first and on to the second again. Earlier
          # windows share this workspace, so the two go to the empty one at
          # the end, where nothing else is beside them.
          def focused():
              return json.loads(as_account("niri msg --json focused-window"))["id"]

          left = focused()
          workspaces = json.loads(as_account("niri msg --json workspaces"))
          last = max(w["idx"] for w in workspaces)
          as_account(f"niri msg action move-column-to-workspace {last}")
          first = focused_workspace()
          as_account(
              "systemd-run --user --collect --quiet --unit=wev-right -- "
              "${pkgs.wev}/bin/wev"
          )
          retry(lambda _: focused() != left, timeout_seconds=30)
          right = focused()
          t.assertTrue(held_still(), "the view never settled")
          swiped(3, "LR", lambda: focused() == left)
          swiped(3, "RL", lambda: focused() == right)
          t.assertEqual(touches("up"), 1, "a swipe reached the window as a tap")

          # Side by side, with the right one focused: a swipe over the left
          # one leaves the focus where it was, as a tap would not. The left one
          # hears the swipe's first two fingers land, which is how the test
          # knows they landed on it.
          as_account("niri msg action focus-column-left")
          as_account("niri msg action set-column-width 50%")
          as_account("niri msg action focus-column-right")
          as_account("niri msg action set-column-width 50%")
          t.assertTrue(held_still(), "the view never settled")
          landed = touches("down")
          swiped(3, "DU", lambda: focused_workspace() == first + 1, across=0.25)
          t.assertEqual(touches("down"), landed + 2, "the swipe missed the window beside")
          swiped(3, "UD", lambda: focused_workspace() == first, across=0.25)
          t.assertEqual(focused(), right, "a swipe focused the window under it")
          as_account("systemctl --user stop wev wev-right")

          # kitty scrolls by touch: a finger dragged down its whole window
          # brings back what had scrolled off the top, and the last line goes.
          as_account(
              "systemd-run --user --collect --quiet --unit=touched -- "
              "kitty --title touched sh -c 'seq 1 3000; exec sleep infinity'"
          )
          def touched_focused(_):
              windows = json.loads(as_account("niri msg --json windows"))
              return any(w["title"] == "touched" and w["is_focused"] for w in windows)

          retry(touched_focused, timeout_seconds=60)
          as_account("niri msg action set-column-width 100%")
          kitty = f"kitty @ --to unix:@kitty-{window_titled('touched')['pid']} get-text"
          retry(lambda _: "3000" in as_account(kitty).split(), timeout_seconds=30)
          swiped(1, "UD", lambda: "3000" not in as_account(kitty).split())
          as_account("systemctl --user stop touched")
          machine.succeed("systemctl stop touchscreen")

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

      # Taking its agent back from BlueZ is a call over the bus, which only
      # an agent still reading the bus can make, and stopping it promptly
      # is what keeps it from holding up every shutdown.
      with subtest("the Bluetooth agent stops at once, handing its agent back"):
          machine.succeed("timeout 15 systemctl stop bluetooth-agent")
          machine.succeed("journalctl -u bluetooth-agent | grep -q 'unregistering agent'")
          machine.fail("systemctl is-failed --quiet bluetooth-agent")

      photograph("desktop")
    '';
}
