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
  ...
}:
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

  # The launcher's Home Network place says the home network as the eero
  # daemon's record has it: the rows' order, what each row says, and every
  # figure the eero contract places in the internet's, a node's and a
  # device's drawer, as tests/home-network.qml says, in Qt's own engine.
  the-home-network-says-each-figure =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-home-network-says-each-figure" { } ''
      mkdir launcher
      cp ${../../../tests/home-network.qml} home-network.qml
      ${copied [ "launcher/HomeNetworkDrawers.js" ]}
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
  # not at all, never growing.
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
}
