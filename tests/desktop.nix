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
# swayimg and a key takes it to GIMP, a new drawing opens in Inkscape from
# the template and leads its Recent, a document opens in zathura beside
# the rest, a file a Neovim in kitty holds is gone to from Neovim's page,
# a VM virsh lists is in Programs and starts from its acts,
# and GTK programs and monospaced text are in the theme. niri says which surfaces it drew,
# shots show their colours, to the test and to a person looking, and
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
  # The screen's size in pixels. Every area but looks, looks-machine,
  # looks-programs and launcher-keeps-up keeps the one its pictures were
  # taken on; those have the laptop's, so the launcher and its drawer fit
  # at both of Joel's scales.
  screen ? {
    width = 1280;
    height = 800;
  },
}:
let
  account = "jitsusama";
  secret = "correct-horse";
  # The areas whose machine brings Slack, as a machine of Joel's does: what
  # reads mail and Slack, and the shortcuts, which list Slack's own.
  bringsSlack = builtins.elem area [
    "launcher-reads-mail"
    "launcher-moves-windows"
  ];
  # The areas whose machine has a Chrome to say its tabs, its sites and
  # its downloads, as the chrome module's recents and its own provider give
  # them: a desktop entry that starts nothing, and its three lists read from
  # files the test writes, in chrome-recent-sites' form, since no browser
  # runs here. Opens-files opens its latest download, looks-programs
  # shoots its page, and keeps-pace steps into it under load.
  bringsChrome = builtins.elem area [
    "opens-files"
    "looks-programs"
    "keeps-pace"
  ];
  # The areas whose machine brings Neovim, as a workstation does, so
  # Programs › Neovim lists the files each Neovim in a kitty window holds:
  # opens-files goes to one, looks-programs shoots the page, and
  # keeps-pace steps into it under load. Only there, since its plugins and
  # grammars are much of the account's build, and no other area reads what
  # it holds; they build the one package, so each after the first costs no
  # build of its own.
  bringsNeovim = builtins.elem area [
    "opens-files"
    "looks-programs"
    "keeps-pace"
  ];
  # The areas whose launcher lists VMs, as a workstation's does
  # (modules/libvirt/home.nix): what the machine has to open and set, and
  # looks-programs, which shoots them.
  listsMachines = builtins.elem area [
    "opens-and-sets"
    "looks-programs"
  ];
  # The area whose For You lets a to-do go by itself, so its machine runs
  # the to-do daemon, as a workstation does (modules/to-do).
  clearsToDos = area == "launcher-waits-on-joel";
  # The areas that show pi's sessions, whose machine runs the realm's
  # mycelium node, as a workstation does, since the shell knows sessions
  # from the node alone, and looks-machine, which shoots Machines from
  # the record the node measures of its own machine, and launcher-answers,
  # whose Queue the node tells of the machine queue (Jobs.qml). Those two
  # link nothing, so they need no key.
  runsTheNode = builtins.elem area [
    "launcher-answers"
    "launcher-finds-sessions"
    "reopens-under-orchestrators"
    "launcher-draws-still"
    "launcher-waits-on-joel"
    "launcher-goes-places"
    "launcher-keeps-up"
    "looks"
    "looks-machine"
  ];
  # The areas whose launcher has WARP, as a machine whose system installs
  # warp-cli does: where its looks are shot, and where a terminal is shown
  # it through `launcher show`.
  standsInForWarp = builtins.elem area [
    "looks"
    "launcher-finds-by-name"
  ];
  # The area that switches its home, onto the sessions' kitty kept through
  # a switch from one that wasn't, as a machine first switching onto it
  # does: it boots on the unit before, which says no switch method, and
  # its specialisation is the one after. And onto a changed shell beside a
  # unit slow to stop, as disk-guard was on 10 November.
  switchesTheHome = area == "outlives-a-switch";

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

  # The VM's own time server, which its chrony keeps the clock by
  # (desktop/stand-in-time.conf).
  timePort = 11123;
  # The hardware address of opens-and-sets' cable, which names it cable0.
  cableAddress = "52:54:00:0c:ab:1e";
  # And of the switch at that cable's other end, which names it switch0.
  switchAddress = "52:54:00:0c:ab:1f";
  timeStandIn = pkgs.replaceVars ./desktop/stand-in-time.conf { port = toString timePort; };

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

  # Stands in for fwupd, which the VM doesn't run, on the system bus
  # (desktop/stand-in-fwupd.py): each of its answers is what the test wrote
  # to the runtime folder's fwupd/<command>.json, as fwupdmgr's --json says
  # it, or else the laptop's two devices with nothing offered.
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
  standInFwupd = pkgs.writers.writePython3 "stand-in-fwupd" {
    libraries = [ pkgs.python3Packages.jeepney ];
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-fwupd.py);
  # Lets the stand-in hold fwupd's name on the system bus, and anyone ask it.
  standInFwupdPolicy = pkgs.writeTextDir "share/dbus-1/system.d/stand-in-fwupd.conf" ''
    <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
      "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
    <busconfig>
      <policy user="root">
        <allow own="org.freedesktop.fwupd"/>
      </policy>
      <policy context="default">
        <allow send_destination="org.freedesktop.fwupd"/>
      </policy>
    </busconfig>
  '';
  # Stands in for fwupdmgr, which Firmware runs only to install: it runs
  # what the test wrote to the runtime folder's fwupd/update, as fwupd
  # would install, or else fails, with nothing to install.
  standInFwupdmgr = pkgs.writeShellScript "fwupdmgr" ''
    installs=$XDG_RUNTIME_DIR/fwupd/update
    if [ "$1" = update ] && [ -x "$installs" ]; then exec "$installs" "$@"; fi
    echo "No updatable devices"; exit 2
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

  # Stands in for the collector, keeping each line the launcher sends it
  # (desktop/stand-in-collector.py).
  standInCollector = pkgs.writers.writePython3 "stand-in-collector" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-collector.py);

  # Stands in for the google program, which would ask Google, on its socket
  # (desktop/stand-in-google.py).
  standInGoogle = pkgs.writers.writePython3 "stand-in-google" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-google.py);

  # Stands in for virsh, which would ask libvirt, giving the launcher one
  # session machine to list and start (desktop/stand-in-virsh.sh), since
  # the VM runs no libvirt.
  standInVirsh = pkgs.writeShellScriptBin "virsh" ''
    PATH=${pkgs.coreutils}/bin exec ${pkgs.lib.getExe pkgs.bash} ${./desktop/stand-in-virsh.sh} "$@"
  '';

  # Stands in for Slack's daemon, which would ask Slack, on its socket
  # (desktop/stand-in-slack.py).
  standInSlack = pkgs.writers.writePython3 "stand-in-slack" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-slack.py);

  # The areas that walk Passwords, looks-programs, which shoots it, and
  # keeps-pace, which steps into it under load, whose 1Password's manager
  # asks a stand-in for 1Password's command, op, since no 1Password account
  # signs in here (desktop/op-stand-in.sh).
  walksPasswords = builtins.elem area [
    "launcher-goes-places"
    "looks-programs"
    "keeps-pace"
  ];
  standInOp = pkgs.writeShellScript "op" ''
    PATH=${pkgs.coreutils}/bin exec ${pkgs.lib.getExe pkgs.bash} ${./desktop/op-stand-in.sh} "$@"
  '';

  # Stands in for the screen-text daemon, which would read screenshots with
  # its models, on its socket, saying what the runtime folder's
  # screen-text-texts.json says each says (desktop/stand-in-screen-text.py).
  standInScreenText = pkgs.writers.writePython3 "stand-in-screen-text" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-screen-text.py);

  # Stands in for the meaning daemon, which would embed the launcher's
  # things with its model, on its socket, finding for each text what the
  # runtime folder's meaning-means.json says it means
  # (desktop/stand-in-meaning.py).
  standInMeaning = pkgs.writers.writePython3 "stand-in-meaning" {
    doCheck = false;
  } (builtins.readFile ./desktop/stand-in-meaning.py);

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

  # Windows through X11 retitled without rest, as a crowd of agents'
  # windows is (desktop/window-storm.py).
  windowStorm = pkgs.writers.writePython3 "window-storm" {
    libraries = [ pkgs.python3Packages.xlib ];
    doCheck = false;
  } (builtins.readFile ./desktop/window-storm.py);

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

  # A stand-in keyboard, pressing one key each time it's told through
  # /run/keyboard and saying when, so a key is timed from the device
  # (desktop/keyboard.py).
  keyboard = pkgs.writers.writePython3 "keyboard" {
    libraries = [ pkgs.python3Packages.evdev ];
    doCheck = false;
  } (builtins.readFile ./desktop/keyboard.py);

  # A stand-in ErgoDox, plugged in for as long as it runs
  # (desktop/ergodox.py), and the layout its module describes for the
  # shell, from the firmware the board runs.
  ergodox = pkgs.writers.writePython3 "ergodox" {
    libraries = [ pkgs.python3Packages.evdev ];
    doCheck = false;
  } (builtins.readFile ./desktop/ergodox.py);
  ergodoxLayout =
    let
      firmware = import ../modules/ergodox/firmware.nix pkgs {
        firmware = self.inputs.qmk;
        inherit (self.inputs) lufa printf;
      };
    in
    "${firmware}/shell.json";

  # A picture and a document to open, as a download would be.
  picture = "${pkgs.nixos-icons}/share/icons/hicolor/256x256/apps/nix-snowflake.png";
  page = import ./lib/page.nix {
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

  # The status files the shell watches, each written again as their
  # writers write them, aside and renamed in, one every so many seconds in
  # turn, so the shell hears what it hears on a working machine. Each says
  # again what it said, so nothing it shows changes.
  rewrites = pkgs.writeShellApplication {
    name = "rewrites";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      every=$1
      shift
      while :; do
        for file in "$@"; do
          [ -f "$file" ] || continue
          aside="$(dirname "$file")/.writing"
          mkdir -p "$aside"
          cp "$file" "$aside/$(basename "$file")"
          mv "$aside/$(basename "$file")" "$file"
          sleep "$every"
        done
        sleep "$every"
      done
    '';
  };

  # pi, as the launcher opens it, since the VM has no pi.
  piStandIn = import ./pi-stand-in.nix { inherit pkgs; };

  # Stands in for Inkscape where keeps-copies opens a new drawing in it
  # (desktop/inkscape-stand-in.sh), adding it to the list of files opened
  # lately as Inkscape does, through the open-with module's recent-files.
  standInInkscape = pkgs.writeShellScript "inkscape" ''
    PATH=${pkgs.coreutils}/bin:${pkgs.callPackage ../modules/open-with/recent-files.nix { }}/bin \
      exec ${pkgs.lib.getExe pkgs.bash} ${./desktop/inkscape-stand-in.sh} "$@"
  '';

  # spotify-recent signed in, as the-launcher-plays-from-spotify stands it
  # in (spotify-recent-stand-in.sh), for looks-programs to shoot Spotify's page;
  # with a show kept, so Podcasts holds one.
  spotifyStandIn = pkgs.writeShellScript "spotify-recent" ''
    if [ "''${1:-}" = shows ]; then
      printf 'spotify:show:s1\tshow\tThe Show\tA Publisher\t\t1212\n'
      exit
    fi
    PATH=${pkgs.coreutils}/bin exec ${pkgs.lib.getExe pkgs.bash} ${./spotify-recent-stand-in.sh} well "$@"
  '';
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "desktop-${area}${pkgs.lib.optionalString gallery "-gallery"}";
  # Reads the colours of the windows it shoots. The desktop it drives
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
        # The laptop role's keyd describes the built-in keyboard's layout for
        # Super+K; its layout alone, since keyd itself would type Colemak
        # for the test's US keys.
        ../modules/keyd/layout.nix
      ]
      # The workstation role's builds, in build.slice at its weight, which
      # keeps-pace loads as a machine's builds do. Only there, since the
      # daemon's settings change nothing any other area reads.
      ++ lib.optional (area == "keeps-pace") self.nixosModules.nix-daemon;

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
        # For a stand-in desk light, a HID device as a Litra is.
        "uhid"
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
      # fwupd, as the machine daemon asks it, saying what the account's
      # runtime folder's fwupd/ says.
      services.dbus.packages = [ standInFwupdPolicy ];
      systemd.services.stand-in-fwupd = {
        wantedBy = [ "multi-user.target" ];
        after = [ "dbus.service" ];
        serviceConfig.ExecStart = "${standInFwupd} /run/user/1000/fwupd ${fwupdDevices}";
      };
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
      # stop, the captures area's recordings failed ("video 1 never grew").
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
        qemu.options = [
          "-vga none -device virtio-gpu-pci,xres=${toString screen.width},yres=${toString screen.height}"
        ]
        # A cable of its own, as the laptop's dock gives it: a card that
        # says it agreed on a gigabit, full duplex, on a network of its own
        # that hands out an address, and whose carrier QEMU's monitor drops
        # and brings back with set_link. A hub joins it to that network and
        # to a second card, the switch at its other end, which says what it
        # is over LLDP.
        ++ lib.optionals (area == "opens-and-sets") [
          "-netdev user,id=cablenet,net=10.9.0.0/24,restrict=on"
          "-netdev hubport,id=cablenetport,hubid=9,netdev=cablenet"
          "-netdev hubport,id=cable,hubid=9"
          "-device virtio-net-pci,netdev=cable,id=cable,mac=${cableAddress},speed=1000,duplex=full"
          "-netdev hubport,id=switch,hubid=9"
          "-device virtio-net-pci,netdev=switch,id=switch,mac=${switchAddress}"
        ];
      };
      # It takes an address from its network, but no route or name server,
      # so the way out stays the VM's own, as every other area has it.
      systemd.network = lib.mkIf (area == "opens-and-sets") {
        links."10-cable" = {
          matchConfig.MACAddress = cableAddress;
          linkConfig.Name = "cable0";
        };
        links."10-switch" = {
          matchConfig.MACAddress = switchAddress;
          linkConfig.Name = "switch0";
        };
        # The switch only says what it is, and takes no address.
        networks."10-switch" = {
          matchConfig.Name = "switch0";
          networkConfig = {
            EmitLLDP = "nearest-bridge";
            LLDP = false;
            LinkLocalAddressing = "no";
            IPv6AcceptRA = false;
          };
          linkConfig.RequiredForOnline = "no";
        };
        networks."10-cable" = {
          matchConfig.Name = "cable0";
          # A station's LLDP is heard too, not only a router's, since the
          # switch here is this machine's own second card.
          networkConfig.LLDP = true;
          networkConfig.DHCP = "ipv4";
          dhcpV4Config = {
            UseRoutes = false;
            UseGateway = false;
            UseDNS = false;
          };
          linkConfig.RequiredForOnline = "no";
        };
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
      # chrony keeps the clock as on a laptop, by a time server of the
      # VM's own, a second chronyd serving the same clock, so Time has a
      # chosen server with true figures wherever the VM runs, and by one
      # named that never answers, out of reach. The pool is left out, so
      # what the VM reaches when run by hand changes nothing.
      networking.hosts = {
        "127.0.0.1" = [ "time.harbour.test" ];
        "127.0.0.3" = [ "far.harbour.test" ];
      };
      services.chrony = {
        servers = lib.mkForce [ ];
        extraConfig = ''
          server time.harbour.test port ${toString timePort} iburst minpoll 0 maxpoll 2
          server far.harbour.test iburst
        '';
      };
      systemd.services.time-stand-in = {
        description = "Serves the VM's own clock as a time server";
        wantedBy = [ "multi-user.target" ];
        before = [ "chronyd.service" ];
        serviceConfig = {
          # It serves the clock and never sets it.
          ExecStart = "${pkgs.chrony}/bin/chronyd -n -x -u chrony -f ${timeStandIn}";
          RuntimeDirectory = "time-stand-in";
        };
      };
      home-manager.users.${account} = {
        # Slack, as a machine that brings it imports it, so Now's Slack piece
        # reads the file a test writes in slack-unreads' place. Nobody signs
        # in to Slack here, so slack-unreads itself never starts. Only where
        # an area reads it.
        imports =
          lib.optional bringsSlack self.homeModules.slack
          ++ lib.optional bringsNeovim self.homeModules.neovim
          ++ lib.optional listsMachines self.homeModules.libvirt
          ++ lib.optional clearsToDos self.homeModules.to-do
          ++ lib.optionals runsTheNode [
            self.homeModules.mycelium
            { jitsusama.mycelium.realm = "harbour"; }
          ];
        # And Slack's own desktop entry, as the machine's Slack brings it,
        # so Programs has the row whose note says why Slack can't be
        # checked. It starts nothing, since nothing here opens it.
        xdg.desktopEntries =
          lib.optionalAttrs bringsSlack {
            slack = {
              name = "Slack";
              genericName = "Messages";
              exec = "true %U";
            };
          }
          # And Chrome's, where an area opens Chrome or a login that
          # 1Password's extension fills in it.
          // lib.optionalAttrs (bringsChrome || walksPasswords) {
            google-chrome = {
              name = "Google Chrome";
              genericName = "Web Browser";
              exec = "true %U";
            };
          };
        # The sessions' kitty as it was before it was kept through a switch,
        # with no switch method, so a change restarts it.
        systemd.user.services.sessions-kitty.Unit.X-SwitchMethod = lib.mkIf switchesTheHome (
          lib.mkForce null
        );
        # The launcher's VMs read the stand-in, the virsh on the account's
        # path, in the daemon's place.
        jitsusama.quickshell.launcher.providers.vms = lib.mkIf listsMachines {
          virsh = lib.mkForce (lib.getExe' standInVirsh "virsh");
        };
        home.packages = lib.optional listsMachines standInVirsh;
        jitsusama.quickshell.launcher.recents = lib.optionals bringsChrome [
          {
            app = "google-chrome";
            kind = "site";
            asked = true;
            list = [
              "${pkgs.coreutils}/bin/cat"
              "/run/user/1000/chrome-visited"
            ];
            opened = [
              "${pkgs.coreutils}/bin/cat"
              "/run/user/1000/chrome-open"
            ];
          }
          # Its downloads, which the programs daemon reads as its page asks.
          {
            app = "google-chrome";
            name = "downloads";
            asked = true;
            list = [
              "${pkgs.coreutils}/bin/cat"
              "/run/user/1000/chrome-downloads"
            ];
          }
        ];
        # And Chrome's own provider, as its module gives it (modules/chrome/
        # home.nix), for its downloads and Open the Latest Download. Its
        # repositories aren't given, since no repository is here to open.
        jitsusama.quickshell.launcher.providers.chrome = lib.mkIf bringsChrome {
          order = 52;
          source = ../modules/chrome/ChromeProgram.qml;
          downloads = "listed:google-chrome:downloads";
        };
        xdg.configFile."quickshell/launcher/ChromeDownloads.js" = lib.mkIf bringsChrome {
          source = ../modules/chrome/ChromeDownloads.js;
        };
        home.stateVersion = "26.05";
        # A cursor that holds still, so two runs shoot kitty the same
        # rather than catching its fade wherever it happens to be.
        xdg.configFile."kitty/kitty.conf".text = lib.mkAfter "cursor_blink_interval 0";
        # And the launcher's caret, for the same reason.
        jitsusama.quickshell.caret.fade = 0;
        jitsusama.quickshell.reports = true;
        # A camera whose holders a module would name, for the test to
        # write in its place, since the VM has none.
        jitsusama.quickshell.margin.pieces.camera.holders = [ "camera" ];
        # The VM's disk is the host's, which waits for minutes on end while
        # other builds share it, so lag-watch's real windows would put the
        # Lag mark in the margin of every test that reads it. Waiting is
        # marked only after an hour here, longer than any test runs, and
        # the test of the mark writes a wait that long itself.
        jitsusama.quickshell.margin.pieces.lag.lasts = 3600;
        # Something on air lingers in the margin once it goes; a moment is
        # enough to see it, and keeps the test's waits for it to go short.
        jitsusama.quickshell.margin.linger = 2;
        # The test types nothing for long stretches while it reads the
        # journal, so it never counts as away, and its notices go by
        # themselves when they're meant to.
        jitsusama.quickshell.notices.away = 3600;
        # Nor does the screen lock by itself. A loaded builder let five
        # minutes pass with no key from the test, typing by wtype counting
        # for none, and the idle lock fell mid-run, failing every subtest
        # after it behind the lock (2026-11-03). The lock is tested by its
        # key and by logind's lock-session, which swayidle still answers,
        # and stepping away by swayidle-away run with short times, which
        # the test starts itself.
        systemd.user.services.swayidle-away.Install.WantedBy = lib.mkForce [ ];
        # Dictation's own service would load Whisper in the VM.
        systemd.user.services.dictation.Service.ExecStart = lib.mkForce "${standInDictation}";
        # The google program would ask Google, as an account nobody signed
        # in to in the VM; the test starts the stand-in when it wants it.
        systemd.user.services.google.Service.ExecStart =
          lib.mkForce "${standInGoogle} %t/daemons/google.sock";
        # tuple-call follows a stand-in for Tuple's command, whose stream
        # says what the test writes (desktop/tuple-stand-in.sh), since no
        # call can be made in the VM.
        systemd.user.services.tuple-call.Service.ExecStart = lib.mkForce (
          lib.escapeShellArgs [
            (lib.getExe (pkgs.callPackage ../modules/tuple/call/package.nix { }))
            "%t/daemons/tuple.sock"
            (pkgs.writeShellScript "tuple" ''
              PATH=${pkgs.coreutils}/bin exec ${lib.getExe pkgs.bash} ${./desktop/tuple-stand-in.sh} "$@"
            '')
          ]
        );
        # Tuple's daemon answering on its socket, which is all tuple-call
        # asks of it, started by the test with Tuple's own stopped.
        systemd.user.services.tuple-answers.Service.ExecStart =
          "${lib.getExe pkgs.socat} UNIX-LISTEN:%t/tuple-0,fork,unlink-early /dev/null";
        # The screen-text daemon, where Capture's Recent is searched by
        # what its screenshots say, a stand-in that reads no picture; in
        # looks too, where it's shot beside what meaning saw.
        systemd.user.services.screen-text.Service.ExecStart = lib.mkIf (builtins.elem area [
          "launcher-goes-places"
          "looks"
        ]) (lib.mkForce "${standInScreenText} %t/daemons/screen-text.sock %t/screen-text-texts.json");
        # The meaning daemon would embed every thing on the processor, with
        # no NPU here, beside every test; a stand-in that loads no model
        # answers what the test writes, and the real one is proven in the
        # build sandbox (modules/meaning/checks.nix).
        systemd.user.services.meaning.Service.ExecStart =
          lib.mkForce "${standInMeaning} %t/daemons/meaning.sock %t/meaning-means.json";
        # Passwords walked with 1Password's own manager, in the passwords
        # daemon's own unit, asking the stand-in for op in place of the
        # one 1Password's app would answer.
        # Its socket, which nothing starts on a real machine until agents are
        # fenced (modules/passwords/home.nix), is listened on here, where no
        # agent runs and no secret is real.
        systemd.user.sockets.passwords.Install.WantedBy = lib.mkIf walksPasswords [
          "sockets.target"
        ];
        jitsusama.passwords.managers."1password".command = lib.mkIf walksPasswords (
          lib.mkForce [
            (lib.getExe (pkgs.callPackage ../modules/onepassword/manager/package.nix { }))
            "--op"
            "${standInOp}"
          ]
        );
        # The collector where the launcher's traces are tested, started by
        # the test that reads what it was sent.
        systemd.user.services.stand-in-collector = lib.mkIf (area == "launcher-goes-places") {
          Service.ExecStart = "${standInCollector}";
        };
        # A unit whose stop takes 30 s, as disk-guard's did when a switch
        # stopped it mid-step, started by the test that switches past it.
        systemd.user.services.slow-to-stop = lib.mkIf switchesTheHome {
          Service = {
            ExecStart = "${lib.getExe' pkgs.coreutils "sleep"} infinity";
            ExecStop = "${lib.getExe' pkgs.coreutils "sleep"} 30";
          };
        };
        # Inkscape stood in where keeps-copies opens a new drawing in it.
        jitsusama.quickshell.launcher.providers.inkscape = lib.mkIf (area == "keeps-copies") {
          inkscape = lib.mkForce "${standInInkscape}";
        };
        # Spotify signed in where looks-programs shoots its page.
        jitsusama.quickshell.launcher.providers.spotify-program.command = lib.mkIf (
          area == "looks-programs"
        ) (lib.mkForce [ "${spotifyStandIn}" ]);
        jitsusama.google.window = [
          "${googleWindow}"
          "%u"
        ];
        # A source keeping a page for Grace, as a machine's own repository
        # may bring one, silent until a test writes what it knows.
        jitsusama.people.sources.pages = {
          order = 30;
          source = ./desktop/people-pages.qml;
        };
        # And one that knows a crowd, as Tuple knows every contact, silent
        # until a test says how many.
        jitsusama.people.sources.crowd = {
          order = 31;
          source = ./desktop/people-crowd.qml;
        };
        # And one whose crowd the people daemon holds, as Tuple's contacts
        # are, none of them listed, so only a search finds one.
        jitsusama.people.sources.roster = {
          order = 32;
          source = ./desktop/people-roster.qml;
          daemon = [
            (lib.getExe pkgs.bash)
            "${./desktop/people-roster.sh}"
          ];
        };
        # Slack's reader, where Slack's module is imported, a stand-in the
        # people daemon runs in its place, with no sign-in until a test
        # gives it one.
        jitsusama.people.sources.slack = lib.mkIf bringsSlack {
          daemon = lib.mkForce [
            (lib.getExe pkgs.bash)
            "${./desktop/people-slack.sh}"
          ];
        };
        # A source ending each search, as Search Slack does, silent until a
        # test says what it ends with.
        jitsusama.quickshell.launcher.providers.search-ends = {
          order = 990;
          source = ./desktop/search-ends.qml;
        };
        # Slack's daemon, for the Search Slack that Slack's module brings
        # where it's imported, as a stand-in the test starts when it wants
        # it; Search Slack is offered only while it runs, so no other
        # test's search ends with it.
        systemd.user.services.slack-stand-in.Service.ExecStart = "${standInSlack} %t/daemons/slack.sock";
        # The eero daemon would ask eero's cloud, which the VM can't reach.
        # Its record is written as the daemon writes it, the model the
        # daemon's parsers are tested against (modules/eero), so the Home
        # Network has a whole three-node network to show.
        jitsusama.quickshell.launcher.providers.eero.enable = lib.mkForce true;
        systemd.user.services.eero-stand-in = {
          Service = {
            Type = "oneshot";
            ExecStart = lib.escapeShellArgs [
              (lib.getExe' pkgs.coreutils "install")
              "-D"
              "-m"
              "644"
              "${../modules/eero/home-network.json}"
              "%t/eero/home-network.json"
            ];
          };
          Install.WantedBy = [ "default.target" ];
        };
        # Firmware's fwupd isn't in the VM, and would have nothing to say.
        jitsusama.quickshell.launcher.providers.firmware = {
          enable = lib.mkForce true;
          fwupdmgr = lib.mkForce "${standInFwupdmgr}";
        };
        # WARP's warp-cli isn't in the VM, so where WARP is shown a stand-in
        # answers with what the test writes, as the contract's files say it
        # (warp-cli-stand-in.sh), the real warp reading it.
        jitsusama.quickshell.launcher.providers.warp = lib.mkIf standsInForWarp {
          enable = lib.mkForce true;
          warpCli = lib.mkForce (
            toString (
              pkgs.writeShellScript "warp-cli" ''
                PATH=${pkgs.coreutils}/bin exec ${lib.getExe pkgs.bash} ${./warp-cli-stand-in.sh} "$@"
              ''
            )
          );
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
        rewrites
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
      # A switch starts tty1's getty, which Cage's unit conflicts with, so
      # it stops Cage and the session in it; greetd, which a machine runs,
      # keeps the getty off. getty.target starts it as autovt@tty1, which
      # is the name greetd turns off, and turning off getty@tty1 left the
      # switch back stopping Cage. Only where the area switches.
      systemd.services."autovt@tty1".enable = lib.mkIf switchesTheHome false;
      # Stands in for niri's own unit, which starts the graphical session, and
      # with it the desktop portal.
      systemd.user.services.compositor = {
        bindsTo = [ "graphical-session.target" ];
        before = [ "graphical-session.target" ];
        wants = [ "graphical-session-pre.target" ];
        after = [ "graphical-session-pre.target" ];
        serviceConfig.ExecStart = "${pkgs.coreutils}/bin/sleep infinity";
      };
      # The test instrumentation copies the whole journal to the serial port,
      # whose every byte a builder's nested VM takes slowly. The shell's
      # reports, thousands of bytes a key, held a processor in the port's
      # interrupt for 21 s at a time on 5 November, stalling keeps-pace's VM
      # and breaking its driver's pipe. Launcher keeps-up's held arrows report
      # as much, and its guest stalled the same way, 21 s on one processor
      # with the shell's reports on the console at each stall (#2747's runs).
      # Only warnings go there in those areas; the journal keeps the rest,
      # where the tests read it.
      services.journald.settings.Journal.MaxLevelConsole = lib.mkIf (lib.elem area [
        "keeps-pace"
        "launcher-keeps-up"
      ]) (lib.mkForce "warning");
      # The workstation's daemon collects garbage while under 10 GiB is free
      # (modules/nix-daemon), and this machine's store is 2 GiB of tmpfs over
      # the builder's, where only the system's own closure is registered. So
      # the first store operation at boot collected every other path the tests
      # reach, the stand-in keyboard and the tests' values among them, and
      # systemd-run found no keyboard to start. Before Nix 2.34.9 a daemon's
      # connection ended without waiting, and took the collection with it;
      # now it waits for the collection to finish. The area loads the CPU,
      # not the disk, so the daemon here never collects.
      nix.settings.min-free = lib.mkIf (area == "keeps-pace") (lib.mkForce 0);
      # The same machine with the sessions' kitty as modules/quickshell keeps
      # it, kept through a switch, and a changed command line as a new kitty
      # brings, so the switch onto it changes the unit in a way that would
      # restart it. Over the boot's own unit, which forces the method away.
      # And the same machine with the shell's unit and the slow one changed,
      # so a switch onto it stops the slow one and restarts the shell, and
      # with a shell.qml of its own, so the shell's files change under the
      # old shell while the slow one stops.
      specialisation = lib.mkIf switchesTheHome {
        switched.configuration.home-manager.users.${account}.systemd.user.services.sessions-kitty = {
          Unit.X-SwitchMethod = lib.mkOverride 40 "keep-old";
          Service.Environment = [ "SESSIONS_KITTY_SWITCHED=1" ];
        };
        slowed.configuration.home-manager.users.${account} = {
          systemd.user.services = {
            quickshell.Service.Environment = [ "SHELL_SWITCHED=1" ];
            slow-to-stop.Service.Environment = [ "SLOW_TO_STOP_SWITCHED=1" ];
          };
          xdg.configFile."quickshell/shell.qml".source = lib.mkForce (
            pkgs.concatText "shell.qml" [
              ../modules/quickshell/shell.qml
              (pkgs.writeText "switched" "// The switch's own.\n")
            ]
          );
        };
      };
    };

  tests = ./desktop/desktop-${area}-vm-test.py;
  values =
    { nodes, ... }:
    let
      inherit (nodes.machine.home-manager.users.${account}.jitsusama.theme)
        colours
        font
        shape
        space
        ;

      # A silent track with a title, an artist and a cover, for a player to
      # play through no sound card at all.
      track = pkgs.runCommandLocal "track.mp3" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
        ffmpeg -loglevel error -f lavfi -i color=c=0x${pkgs.lib.removePrefix "#" colours.program}:s=300x300 \
          -frames:v 1 cover.png
        ffmpeg -loglevel error -f lavfi -i anullsrc=r=44100:cl=stereo -i cover.png -t 60 \
          -map 0 -map 1 -c:a libmp3lame -c:v png -disposition:v attached_pic \
          -metadata title="Bless The Child" -metadata artist=Nightwish $out
      '';

      # A wide picture and a short video, for the file picker to preview in
      # their own shapes.
      wide = pkgs.runCommandLocal "wide.png" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
        ffmpeg -loglevel error -f lavfi -i gradients=s=1600x1000:seed=7 -frames:v 1 wide.png
        mv wide.png $out
      '';
      clip = pkgs.runCommandLocal "clip.mp4" { nativeBuildInputs = [ pkgs.ffmpeg-headless ]; } ''
        ffmpeg -loglevel error -f lavfi -i testsrc2=s=960x540:r=24 -t 20 -c:v mpeg4 -q:v 4 \
          -f mp4 $out
      '';

      # Every Nix value the script needs, which it reads once, at its top;
      # its Values says what each is.
      values = pkgs.writeText "desktop-values.json" (
        builtins.toJSON (
          {
            inherit
              account
              secret
              asking
              touchscreen
              keyboard
              ergodox
              picture
              page
              text
              track
              wide
              clip
              ;
            ask_portal = askPortal;
            choose_a_file = chooseAFile;
            copilot_key = copilotKey;
            ergodox_layout = ergodoxLayout;
            # A stand-in Litra, plugged in for as long as it runs
            # (desk-light-stand-in.py).
            desk_light = "${pkgs.python3}/bin/python3 ${./desk-light-stand-in.py}";
            # A crowd of pi's sessions, a few changing each beat
            # (desktop/stand-in-sessions.py).
            stand_in_sessions = "${pkgs.python3}/bin/python3 ${./desktop/stand-in-sessions.py}";
            tuple_window = tupleWindow;
            inherit (pkgs)
              coreutils
              dbus
              wev
              zenity
              ;
            ffmpeg_headless = pkgs.ffmpeg-headless;
            fontconfig_bin = pkgs.fontconfig.bin;
            stress_ng = pkgs.stress-ng;
            inherit (colours) accent action sunken;
            mono_family = font.mono.family;
            # What a floating frame leaves of the screen across: the screen
            # rung and its border on either side.
            floating_room = 2 * (space.rung.screen + shape.border);
            shots = gallery;
          }
          # BlueZ's emulator, for the looks area to hear a device in Nearby
          # by: two controllers of the machine's own, each seen as Buds, so
          # whichever the launcher looks from hears the other. Only there, so
          # no other area's test moves with it.
          // pkgs.lib.optionalAttrs (area == "looks") {
            btvirt = "${import ./btvirt.nix { inherit pkgs; }}/bin/btvirt";
          }
          # WARP's contract, for the stand-in warp-cli to answer with, only
          # where WARP is shown.
          # The storm of windows keeps-pace opens the launcher through, only
          # there, so no other area moves with it.
          // pkgs.lib.optionalAttrs (area == "keeps-pace") {
            window_storm = windowStorm;
          }
          // pkgs.lib.optionalAttrs standsInForWarp {
            warp_contract = "${../modules/warp/contract}";
          }
          # A document of twelve pages, for looks-programs to open twice and
          # shoot zathura's page saying the page each is on. Only there, so
          # no other area's test moves with it.
          // pkgs.lib.optionalAttrs (area == "looks-programs") {
            book = import ./lib/page.nix {
              inherit (pkgs) lib;
              inherit pkgs;
              pages = 12;
            };
          }
        )
      );
    in
    values;
  # nix fmt holds the tests and desktop_session.py to every ruff rule, so the
  # driver's own lint of its one line would add only its time.
  skipLint = true;
}
// pkgs.lib.optionalAttrs (!gallery) {
  gallery = import ./desktop.nix {
    inherit
      self
      pkgs
      area
      screen
      ;
    gallery = true;
  };
}
