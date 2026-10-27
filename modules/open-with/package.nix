# open-with, built from this directory's crate with its tests run as it
# builds, and the portal's choosers it serves, of an app and of files: its
# description for the portal, its D-Bus service and the account's unit that
# runs it, which asks the shell through the given Quickshell's qs.
{
  lib,
  rustPlatform,
  quickshell,
}:
rustPlatform.buildRustPackage (_finalAttrs: {
  pname = "open-with";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };
  cargoLock.lockFile = ./Cargo.lock;

  postInstall =
    let
      name = "org.freedesktop.impl.portal.desktop.launcher";
      run = "${placeholder "out"}/bin/open-with ${lib.getExe' quickshell "qs"}";
    in
    ''
      install -D -m 644 /dev/stdin $out/share/xdg-desktop-portal/portals/launcher.portal <<EOF
      [portal]
      DBusName=${name}
      Interfaces=org.freedesktop.impl.portal.AppChooser;org.freedesktop.impl.portal.FileChooser;
      EOF
      install -D -m 644 /dev/stdin $out/share/dbus-1/services/${name}.service <<EOF
      [D-BUS Service]
      Name=${name}
      Exec=${run}
      SystemdService=open-with.service
      EOF
      install -D -m 644 /dev/stdin $out/lib/systemd/user/open-with.service <<EOF
      [Unit]
      Description=The launcher's choosers of an app to open something with and of files
      PartOf=graphical-session.target
      After=graphical-session.target

      [Service]
      Type=dbus
      BusName=${name}
      ExecStart=${run}
      EOF
    '';

  meta = {
    description = "The portal's choosers of an app to open something with and of files, asked in the launcher";
    license = lib.licenses.mit;
    mainProgram = "open-with";
    platforms = lib.platforms.linux;
  };
})
