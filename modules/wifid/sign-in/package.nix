# wifid's sign-in window, built from the workspace beside wifid itself but
# as a package of its own, so the machine's wifid never carries WebKit.
# Its tests run as it builds.
{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook4,
  gtk4,
  webkitgtk_6_0,
  glib-networking,
}:
rustPlatform.buildRustPackage {
  pname = "wifid-sign-in";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../daemon
      ./.
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [
    "--package"
    "wifid-sign-in"
  ];
  cargoTestFlags = [
    "--package"
    "wifid-sign-in"
  ];
  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];
  buildInputs = [
    gtk4
    webkitgtk_6_0
    # TLS for the pages, which GLib's networking gives libsoup.
    glib-networking
  ];
  meta = {
    description = "A window onto the sign-in page of a network that wants one";
    license = lib.licenses.mit;
    mainProgram = "wifid-sign-in";
    platforms = lib.platforms.linux;
  };
}
