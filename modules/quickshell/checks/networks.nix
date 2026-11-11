# The checks of the launcher's networks: Wi-Fi and the home network, which the
# folder's checks.nix gathers with every other place's; its opening comment says
# why each place keeps its own.
{
  account,
  bareNixos,
  copied,
  holds,
  noFonts,
  pkgs,
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
}
