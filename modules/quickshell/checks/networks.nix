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
      # of the card and the sign-in, a card on and nothing saved.
      wifi = singleton "Wifi.qml" ''
        property var cards: [{ name: "wlan0" }]
        property var saved: []
        property var nearby: []
        property bool radio: true
        property bool described: true
        property int looks: 0
        readonly property var joined: null
        function isSaved(name) { return null; }
        function isJoined(name) { return false; }
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
  # Network's holding, as tests/home-network.qml says, in Qt's own engine;
  # its rows and drawers are the eero daemon's, held by modules/eero's tests.
  the-home-network-says-each-figure =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-home-network-says-each-figure" { } ''
      mkdir launcher
      cp ${../../../tests/home-network.qml} home-network.qml
      ${copied [ "launcher/HomeNetwork.js" ]}
      cp ${../../eero/home-network.json} home-network.json
      export QML_XHR_ALLOW_FILE_READ=1 TZ=UTC
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml home-network.qml
      touch $out
    '';

  # Every row Network's Address and Traffic show, and each answer as a row
  # of its own, has a drawer saying more than its line, each fact named by
  # the vocabulary (tests/network-drawers.qml): Address over two stand-in
  # connections, Traffic over the sandbox's own counters, Ethernet with no
  # card, Wi-Fi's own row's drawer, and a connection's drawer shown whole or
  # not at all, never growing. Kept in Nix because each line copies a file
  # Nix gives it beside the cases, or sets what the shell runs them with.
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
    cp ${../../../tests/network-drawers.qml} shell.qml
    cp ${../../../tests/lib/Expect.js} Expect.js
    cp ${../../../tests/lib/Until.qml} Until.qml
    export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts}
    ${shellFolderLint}
    timeout 60 ${qs} -p shell.qml
    touch $out
  '';

  # Wi-Fi joins a network by the acts wifid serves on its row, asking each
  # word a join needs in the field before it's sent (tests/wifi-acts.qml),
  # with the launcher's asking stood in.
  wifi-asks-each-word-a-join-needs-first = wifiCase "wifi-asks-each-word-a-join-needs-first" ../../../tests/wifi-acts.qml;

  # Wi-Fi's cards look for networks only while its rows are shown, Wi-Fi
  # held or a find showing a row of its, or a terminal's show walked in, and
  # never with the launcher open on another place (tests/wifi-looks.qml),
  # since each look takes the card off its channel for seconds.
  wifi-looks-only-while-its-rows-are-shown = wifiCase "wifi-looks-only-while-its-rows-are-shown" ../../../tests/wifi-looks.qml;
}
