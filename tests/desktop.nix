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
# copied, a meeting on the calendar is found by its name and Return joins
# its call, the clipboard takes and gives text, a picture opens floating in
# swayimg and a key takes it to GIMP, a document opens in zathura beside
# the rest, and GTK programs and monospaced text are in the theme. niri says which surfaces it drew,
# photographs show their colours, to the test and to a person looking, and
# the portal and fontconfig answer for the settings. Quickshell has to
# start without a single QML error.
#
# The test runs as one VM test per area of the desktop, each on this same
# machine: its tests are desktop/desktop-<area>-vm-test.py, run through
# pytest inside the driver, which ask for the desktop, its boot and helpers,
# from desktop/desktop_session.py. So a change to one area reruns only that
# one, and the areas run side by side. checks.nix names each area.
#
# The pictures it takes for a person to look at, which `shots-changed`
# compares, are taken only by an area's gallery, `.gallery`, the same test
# with them on: proving a change needs none, and each waits for the screen
# to hold still.
{
  self,
  pkgs,
  area,
  gallery ? false,
}:
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
  # someone signs in (desktop/front-desk.py).
  frontDesk = ./desktop/front-desk.py;

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

  # Stands in for fwupd's fwupdmgr, which the VM doesn't run: each command's
  # --json answer is what the test wrote to the runtime folder's
  # fwupd/<command>.json, or else the laptop's two devices with nothing
  # offered, as fwupd says that, in words and failing.
  fwupdDevices = pkgs.writeText "fwupd-devices.json" (
    builtins.toJSON {
      Devices = [
        {
          DeviceId = "bios";
          Name = "System Firmware";
          Vendor = "Dell";
          Version = "1.12.0";
          Flags = [
            "internal"
            "updatable"
            "needs-reboot"
          ];
        }
        {
          DeviceId = "ssd";
          Name = "PVC10 SK hynix 1024GB";
          Vendor = "SK hynix";
          Version = "61001171";
          Flags = [
            "internal"
            "updatable"
          ];
        }
      ];
    }
  );
  standInFwupdmgr = pkgs.writeShellScript "fwupdmgr" ''
    said=$XDG_RUNTIME_DIR/fwupd/$1.json
    if [ -e "$said" ]; then exec cat "$said"; fi
    case $1 in
      get-devices) exec cat ${fwupdDevices} ;;
      get-history) echo '{"Devices": []}' ;;
      *) echo "No updatable devices"; exit 2 ;;
    esac
  '';

  # Stands in for dictation's service, on the socket systemd hands it
  # (desktop/stand-in-dictation.py). ruff and mypy read each of these
  # stand-ins through `nix fmt`, so the writers' flake8 is off.
  standInDictation = pkgs.writers.writePython3 "stand-in-dictation" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-dictation.py);

  # Stands in for pi-ask, which would ask a model, speaking pi's RPC
  # (desktop/stand-in-ask.py).
  standInAsk = pkgs.writers.writePython3 "stand-in-ask" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-ask.py);

  # Stands in for the google program, which would ask Google, on its socket
  # (desktop/stand-in-google.py).
  standInGoogle = pkgs.writers.writePython3 "stand-in-google" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-google.py);

  # Google's own pages' window, as jitsusama.google.window opens one: what
  # it was asked to open, written down, since the VM has no browser.
  googleWindow = pkgs.writeShellScript "google-window" ''
    echo "$1" > "$XDG_RUNTIME_DIR/google-opened"
  '';

  # Asks the portal to open a file, always asking what with, and writes down
  # its answer (desktop/ask-portal.py).
  askPortal = pkgs.writers.writePython3 "ask-portal" {
    libraries = [ pkgs.python3Packages.jeepney ];
    doCheck = false;
  } (builtins.readFile ./desktop/ask-portal.py);

  # Asks the portal for a file to open or save in a folder, and writes down
  # its answer (desktop/choose-a-file.py).
  chooseAFile = pkgs.writers.writePython3 "choose-a-file" {
    libraries = [ pkgs.python3Packages.jeepney ];
    doCheck = false;
  } (builtins.readFile ./desktop/choose-a-file.py);

  # A window through X11 named only as Tuple names its windows
  # (desktop/tuple-window.py).
  tupleWindow = pkgs.writers.writePython3 "tuple-window" {
    libraries = [ pkgs.python3Packages.xlib ];
    doCheck = false;
  } (builtins.readFile ./desktop/tuple-window.py);

  # A stand-in touchscreen, as wide as the screen, which swipes or taps as
  # it's told through /run/touchscreen (desktop/touchscreen.py).
  touchscreen = pkgs.writers.writePython3 "touchscreen" {
    libraries = [ pkgs.python3Packages.evdev ];
    doCheck = false;
  } (builtins.readFile ./desktop/touchscreen.py);

  # A stand-in Copilot key, sending Super, Shift and F23 as the laptop's
  # does, each time it's told through /run/copilot-key
  # (desktop/copilot-key.py).
  copilotKey = pkgs.writers.writePython3 "copilot-key" {
    libraries = [ pkgs.python3Packages.evdev ];
    doCheck = false;
  } (builtins.readFile ./desktop/copilot-key.py);

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

  # pi, as the launcher opens it: where it started and the quest it was
  # told to load, then what it was told, a line each, written down, since
  # the VM has no pi. While a test leaves pi-stays, it stays, its window
  # titled with all it was told, so the test can find where it opened.
  piStandIn = pkgs.writeShellApplication {
    name = "pi";
    text = ''
      echo "$PWD ''${QUEST_WORKFLOW_AUTOLOAD_ID:-}" > "$XDG_RUNTIME_DIR/pi-opened"
      printf '%s\n' "$@" >> "$XDG_RUNTIME_DIR/pi-opened"
      if [ -e "$XDG_RUNTIME_DIR/pi-stays" ]; then
        printf '\033]2;pi %s\007' "$*"
        exec sleep 600
      fi
    '';
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "desktop-${area}${pkgs.lib.optionalString gallery "-gallery"}";
  # Reads the colours of the windows it photographs. The desktop it drives
  # is desktop/desktop_session.py, a module of the driver's Python, which
  # every area imports.
  extraPythonPackages = python: [
    python.pillow
    (python.toPythonModule (
      pkgs.runCommandLocal "desktop-session" { } ''
        mkdir -p $out/${python.python.sitePackages}
        cp ${./desktop/desktop_session.py} $out/${python.python.sitePackages}/desktop_session.py
      ''
    ))
  ];
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

      # The VM has no backlight, keyboard light, light sensor, lid or sound
      # card, so it gets one of each that drives nothing, which the kernel and PipeWire report
      # as they do the laptop's. The lid shut asks nothing of logind, which
      # would otherwise suspend the machine.
      services.logind.settings.Login.HandleLidSwitch = "ignore";
      boot.extraModulePackages = [
        (config.boot.kernelPackages.callPackage ./stand-in-backlight { })
        (config.boot.kernelPackages.callPackage ./stand-in-light { })
        (config.boot.kernelPackages.callPackage ./stand-in-lid { })
        (config.boot.kernelPackages.callPackage ./stand-in-monitor { })
        config.boot.kernelPackages.v4l2loopback
      ];
      boot.kernelModules = [
        "stand-in-backlight"
        "stand-in-light"
        "stand-in-lid"
        "stand-in-monitor"
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
      # Where a machine's disk guard says how its disks stand, for the test
      # to write in its place. Where cameras' holders are said the graphical
      # role makes itself (usb-camera-holders).
      systemd.tmpfiles.rules = [ "d /run/disk-guard 0755 root root -" ];
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
      # sound waits for its first sound before it writes anything. It follows
      # the dummy driver, in its group, rather than driving: driving, it once
      # stopped for good under load the first time a slow reader kept every
      # buffer, and it says it drives itself, over any node.driver it's
      # given. With PipeWire 1.6.9 a reader stopped for 10 s no longer stops
      # a tone that drives, so no subtest can show this; when the sound did
      # stop, captures-and-locks's recordings failed ("video 1 never grew").
      services.pipewire.extraConfig.pipewire."91-stand-in-microphone" = {
        "context.spa-libs".audiotestsrc = "audiotestsrc/libspa-audiotestsrc";
        "context.objects" = [
          {
            factory = "adapter";
            args = {
              "factory.name" = "audiotestsrc";
              "node.name" = "stand-in-microphone";
              "node.group" = "pipewire.dummy";
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
      # GeoClue asks a geolocation service that isn't there, so whereabouts
      # runs as it does on a machine but never finds the laptop, and the
      # test writes where it is in its stead. Run by hand, the VM can reach
      # the network, and the host's own position would stand in for the
      # test's.
      services.geoclue2.geoProviderUrl = lib.mkForce "http://127.0.0.1:9/v1/geolocate";
      # A test VM turns timesyncd off, which would leave Time's keeping
      # unheard, since it's read only where timesyncd runs. It runs here as
      # on a laptop, asking a server that isn't there, so it makes no
      # exchange even where the VM reaches the network, and the test
      # writes one in its stead. The servers are forced, since a list is
      # otherwise joined to the machine's own pool, which it would reach.
      services.timesyncd = {
        enable = lib.mkForce true;
        servers = lib.mkForce [ "127.0.0.1" ];
        fallbackServers = [ ];
      };
      home-manager.users.${account} = {
        home.stateVersion = "26.05";
        # A cursor that holds still, so two runs photograph kitty the same
        # rather than catching its fade wherever it happens to be.
        xdg.configFile."kitty/kitty.conf".text = lib.mkAfter "cursor_blink_interval 0";
        jitsusama.quickshell.reports = true;
        # A camera whose holders a module would name, for the test to
        # write in its place, since the VM has none.
        jitsusama.quickshell.overview.pieces.camera.holders = [ "camera" ];
        # The VM's disk is the host's, which waits for minutes on end while
        # other builds share it, so lag-watch's real windows would put the
        # Lag mark in the margin of every test that reads it. Waiting is
        # marked only after an hour here, longer than any test runs, and
        # the test of the mark writes a wait that long itself.
        jitsusama.quickshell.overview.pieces.lag.lasts = 3600;
        # Something on air lingers in the margin once it goes; a moment is
        # enough to see it, and keeps the test's waits for it to go short.
        jitsusama.quickshell.margin.linger = 2;
        # The test types nothing for long stretches while it reads the
        # journal, so it never counts as away, and its notices go by
        # themselves when they're meant to.
        jitsusama.quickshell.notices.away = 3600;
        # Dictation's own service would load Whisper in the VM.
        systemd.user.services.dictation.Service.ExecStart = lib.mkForce "${standInDictation}";
        # The google program would ask Google, as an account nobody signed
        # in to in the VM; the test starts the stand-in when it wants it.
        systemd.user.services.google.Service.ExecStart = lib.mkForce "${standInGoogle} %t/google.sock";
        jitsusama.google.window = [
          "${googleWindow}"
          "%u"
        ];
        # Firmware's fwupd isn't in the VM, and would have nothing to say.
        jitsusama.quickshell.launcher.providers.firmware = {
          enable = lib.mkForce true;
          fwupdmgr = lib.mkForce "${standInFwupdmgr}";
        };
        # The Ask row's model would need the network and an account.
        jitsusama.quickshell.launcher.providers.ask = {
          enable = lib.mkForce true;
          command = lib.mkForce "${standInAsk}";
        };
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
        piStandIn
        pkgs.libnotify
        # The disk guard's own program, without its daemons, so Disk's Keep
        # marks a folder kept in the status file the test writes in the
        # account's guard's place, as the guard itself would.
        (pkgs.callPackage ../modules/disk-guard/package.nix { })
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
        # The machine's niri, with its patches, rather than nixpkgs's, through
        # the wrapper its session starts it by.
        program = "${config.security.wrapperDir}/niri-compositor";
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

  tests = ./desktop/desktop-${area}-vm-test.py;
  values =
    { nodes, ... }:
    let
      inherit (nodes.machine.home-manager.users.${account}.jitsusama.theme) colours font;

      # A silent track with a title, an artist and a cover, for a player to
      # play through no sound card at all.
      track = pkgs.runCommandLocal "track.mp3" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
        ffmpeg -loglevel error -f lavfi -i color=c=0x${pkgs.lib.removePrefix "#" colours.program}:s=300x300 \
          -frames:v 1 cover.png
        ffmpeg -loglevel error -f lavfi -i anullsrc=r=44100:cl=stereo -i cover.png -t 60 \
          -map 0 -map 1 -c:a libmp3lame -c:v png -disposition:v attached_pic \
          -metadata title="Bless The Child" -metadata artist=Nightwish $out
      '';

      # Every Nix value the script needs, which it reads once, at its top;
      # its Values says what each is.
      values = pkgs.writeText "desktop-values.json" (
        builtins.toJSON {
          inherit
            account
            secret
            asking
            touchscreen
            picture
            page
            text
            track
            ;
          ask_portal = askPortal;
          choose_a_file = chooseAFile;
          copilot_key = copilotKey;
          tuple_window = tupleWindow;
          inherit (pkgs)
            coreutils
            dbus
            wev
            zenity
            ;
          ffmpeg_headless = pkgs.ffmpeg-headless;
          fontconfig_bin = pkgs.fontconfig.bin;
          inherit (colours) accent action sunken;
          mono_family = font.mono.family;
          photographs = gallery;
        }
      );
    in
    values;
  # nix fmt holds the tests and desktop_session.py to every ruff rule, so the
  # driver's own lint of its one line would add only its time.
  skipLint = true;
}
// pkgs.lib.optionalAttrs (!gallery) {
  gallery = import ./desktop.nix {
    inherit self pkgs area;
    gallery = true;
  };
}
