# The checks of the launcher's networks: Wi-Fi and the home network, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  account,
  bareNixos,
  copied,
  graphical,
  holds,
  noFonts,
  pkgs,
  qs,
  self,
  shellFolderLint,
  singleton,
  ...
}:
let
  # A Wi-Fi case run through the real Wireless.qml, ReadyRows.qml and
  # Daemon.qml, with the launcher stood in by the case itself and Wifi's card
  # and the sign-in stood in here.
  wifiCase =
    name: case:
    let
      # Kept in Nix because each only stands in for what Wireless.qml reads
      # of the card and the sign-in, a card on and nothing saved, joined to
      # the network a case gives its card, as Wifi.qml says it.
      wifi = singleton "Wifi.qml" ''
        property var cards: [{ name: "wlan0" }]
        property var saved: []
        property var nearby: []
        property bool radio: true
        property bool described: true
        property int looks: 0
        readonly property var joined: cards.find(card => card.network) ?? null
        function isSaved(name) { return null; }
        function isJoined(name) { return cards.some(card => card.network === name); }
      '';
      signIn = singleton "SignIn.qml" ''
        readonly property string network: ""
        function open() {}
      '';
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what the shell runs them with.
    pkgs.runCommandLocal name { } ''
      mkdir -p launcher runtime && chmod 700 runtime
      ${copied [
        "Daemon.qml"
        "Words.qml"
        "launcher/Provider.qml"
        "launcher/ReadyRows.qml"
        "launcher/Wireless.qml"
        "launcher/words.json"
      ]}
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${wifi} Wifi.qml
      cp ${signIn} SignIn.qml
      cp ${case} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Until.qml} Until.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
in
{
  # The Home Network is offered wherever eero's module brings its daemon,
  # and nowhere else. The provider asks for the daemon's unit by name, so
  # a unit renamed without it would take the place off every machine
  # while the VM test, which turns the provider on itself, still passed.
  the-home-network-is-offered-where-eero-runs =
    let
      offers =
        modules:
        (account (bareNixos ([ self.nixosModules.graphical ] ++ modules)))
        .jitsusama.quickshell.launcher.providers.eero.enable;
    in
    holds "the-home-network-is-offered-where-eero-runs"
      "the launcher's Home Network should be offered where eero's module runs its daemon, and only there"
      (offers [ { home-manager.sharedModules = [ self.homeModules.eero ]; } ] && !offers [ ]);

  # The figures a network's rates and sizes are said in, and the Home
  # Network's holding, as modules/quickshell/tests/home-network.qml says, in
  # Qt's own engine; its rows and drawers are the eero daemon's, held by
  # modules/eero's tests.
  the-home-network-says-each-figure =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-home-network-says-each-figure" { } ''
      mkdir launcher
      cp ${../tests/home-network.qml} home-network.qml
      ${copied [ "launcher/HomeNetwork.js" ]}
      cp ${../../eero/home-network.json} home-network.json
      export QML_XHR_ALLOW_FILE_READ=1 TZ=UTC
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml home-network.qml
      touch $out
    '';

  # Every row Network's Address and Traffic show, and each answer as a row of
  # its own, has a drawer saying more than its line, each fact named by the
  # vocabulary (modules/quickshell/tests/network-drawers.qml): Address over two
  # stand-in connections, Traffic over the sandbox's own counters, Ethernet with
  # no card, Wi-Fi's own row's drawer, and a connection's drawer shown whole or
  # not at all, never growing. Kept in Nix because each line copies a file Nix
  # gives it beside the cases, or sets what the shell runs them with.
  every-network-row-has-a-drawer = pkgs.runCommandLocal "every-network-row-has-a-drawer" { } ''
    mkdir -p launcher runtime && chmod 700 runtime
    ${copied [
      "Daemon.qml"
      "Words.qml"
      "launcher/Networks.qml"
      "launcher/Provider.qml"
      "launcher/Traffic.qml"
      "launcher/WirelessPlate.js"
      "launcher/words.json"
    ]}
    cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
    cp ${../tests/network-drawers.qml} shell.qml
    cp ${../../../tests/lib/Expect.js} Expect.js
    cp ${../../../tests/lib/Until.qml} Until.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # Traffic, asked after a gap as each step into Network asks it, reads no
  # file and waits on none, and loads one sysfs file in the background for
  # an interface it already knows rather than all four again
  # (modules/quickshell/tests/traffic-reads.qml), since a blocking read of
  # each cost the step up to 133 ms under load. A build has no sysfs, so the
  # case writes lo's folder in sys/lo, made here.
  traffic-reads-a-known-interface-once-after-a-gap =
    pkgs.runCommandLocal "traffic-reads-a-known-interface-once-after-a-gap" { }
      # Kept in Nix because each line copies a file Nix gives it beside the
      # case, or sets what the shell runs it with.
      ''
        mkdir -p launcher runtime sys/lo && chmod 700 runtime
        ${copied [
          "Daemon.qml"
          "Words.qml"
          "launcher/Interfaces.js"
          "launcher/Provider.qml"
          "launcher/Traffic.qml"
          "launcher/words.json"
        ]}
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../tests/traffic-reads.qml} shell.qml
        cp ${../../../tests/lib/Expect.js} Expect.js
        cp ${../../../tests/lib/Until.qml} Until.qml
        cp ${../../../tests/lib/Written.js} Written.js
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 60 ${qs} -p shell.qml
        touch $out
      '';

  # Address answers from the routes as last loaded, waiting on no file, and
  # loads them again in the background as the launcher opens, as the links
  # change and when they're old, telling the launcher once a load changes a
  # gateway or its cost and never for the kernel's use counts alone
  # (modules/quickshell/tests/network-routes.qml), since a step into Network
  # asks Address and Ethernet, which each read both routing tables. The case writes its
  # own tables in net/, made here. Kept in Nix because each line copies a
  # file Nix gives it beside the case, or sets what the shell runs it with.
  address-waits-on-no-route-file = pkgs.runCommandLocal "address-waits-on-no-route-file" { } ''
    mkdir -p launcher runtime net && chmod 700 runtime
    ${copied [
      "Daemon.qml"
      "Words.qml"
      "launcher/Networks.qml"
      "launcher/Provider.qml"
      "launcher/words.json"
    ]}
    cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
    cp ${../tests/network-routes.qml} shell.qml
    cp ${../../../tests/lib/Expect.js} Expect.js
    cp ${../../../tests/lib/Until.qml} Until.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # Ethernet answers from a cable's files, the name servers and the time since
  # boot as last loaded, waiting on none, and loads them again in the
  # background as the launcher opens and each second while a cable's shown,
  # telling the launcher once a load changes them; a cable's drawer is made
  # only once its files land, a card gone or changed has its load let go, and a
  # card whose load is given up isn't asked again at once
  # (modules/quickshell/tests/network-docked-reads.qml), since docked with one
  # card a step into Network read over forty files. The case writes its two
  # cards' folders, resolv.conf and uptime, in folders made here. Its
  # waits add up to over a minute at their bounds, so its timeout sits above
  # them, and a wait that runs out says which it was.
  ethernet-waits-on-no-cable-file =
    pkgs.runCommandLocal "ethernet-waits-on-no-cable-file" { }
      # Kept in Nix because each line copies a file Nix gives it beside the
      # case, or sets what the shell runs it with.
      ''
        mkdir -p launcher runtime net etc sys/enp0s1/statistics sys/enp0s2 && chmod 700 runtime
        ${copied [
          "Daemon.qml"
          "Words.qml"
          "launcher/Networks.qml"
          "launcher/Provider.qml"
          "launcher/words.json"
        ]}
        cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
        cp ${../tests/network-docked-reads.qml} shell.qml
        cp ${../../../tests/lib/Expect.js} Expect.js
        cp ${../../../tests/lib/Until.qml} Until.qml
        cp ${../../../tests/lib/Written.js} Written.js
        export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
        export FONTCONFIG_FILE=${noFonts}
        ${shellFolderLint}
        timeout 120 ${qs} -p shell.qml
        touch $out
      '';

  # Wi-Fi joins a network by the acts wifid serves on its row, asking each word
  # a join needs in the field before it's sent
  # (modules/quickshell/tests/wifi-acts.qml), with the launcher's asking stood
  # in.
  wifi-asks-each-word-a-join-needs-first = wifiCase "wifi-asks-each-word-a-join-needs-first" ../tests/wifi-acts.qml;

  # Wi-Fi's cards look for networks only while its rows are shown, Wi-Fi held or
  # a find showing a row of its, or a terminal's show walked in, and never with
  # the launcher open on another place
  # (modules/quickshell/tests/wifi-looks.qml), since each look takes the card
  # off its channel for seconds.
  wifi-looks-only-while-its-rows-are-shown = wifiCase "wifi-looks-only-while-its-rows-are-shown" ../tests/wifi-looks.qml;

  # Wi-Fi holds the network joined ahead while the launcher's open, so a
  # step onto Wi-Fi or into it finds its drawer held, and a change to that
  # drawer alone, nobody looking at it, tells the launcher nothing
  # (modules/quickshell/tests/wifi-ahead.qml).
  wifi-holds-the-network-joined-ahead = wifiCase "wifi-holds-the-network-joined-ahead" ../tests/wifi-ahead.qml;

  # A network wanting a sign-in is told of once, and its notice taken back once
  # it stops wanting one, even a notice arriving only after that
  # (modules/quickshell/tests/sign-in.qml), with the real SignIn.qml and what it
  # reads stood in.
  a-sign-in-notice-is-taken-back-even-late =
    let
      # Kept in Nix because each only stands in for what SignIn.qml reads
      # or says, recording what it was told.
      wifi = singleton "Wifi.qml" ''
        property var cards: []
        readonly property var signingIn: null
      '';
      notices = singleton "Notices.qml" ''
        property var live: ({})
        property var history: []
        property var marked: []
        signal came(var notification, var notice)
        function mark(id, read, how) { marked = marked.concat([[id, read, how]]); }
      '';
      niri = singleton "Niri.qml" ''
        property var spawned: []
        function spawn(command) { spawned = spawned.concat([command]); }
      '';
      report = singleton "Report.qml" ''
        property var said: []
        function say(text) { said = said.concat([text]); }
      '';
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # case, or sets what the shell runs it with.
    pkgs.runCommandLocal "a-sign-in-notice-is-taken-back-even-late" { } ''
      mkdir -p runtime && chmod 700 runtime
      ${copied [ "SignIn.qml" ]}
      cp ${wifi} Wifi.qml
      cp ${notices} Notices.qml
      cp ${niri} Niri.qml
      cp ${report} Report.qml
      cp ${../tests/sign-in.qml} shell.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';
}
