# call-under-load, built from this directory, its tests run as it builds,
# with the call's page in share/ and on its path what a run starts:
# Weston for the near end's display, a-call-is-on for Joel's calls,
# journalctl for the launcher's openings and Chromium for the call, and
# for the camera's check before a meeting notify-send and v4l2-ctl, by a
# link named camera-check. The kernel names a process by the path it was
# started by, and the desktop names a camera's reader by that name, so a
# camera lit before a meeting says the check lit it rather than a tool Joel
# never ran. Not Google Chrome: a managed machine's Chrome enrols every
# fresh profile with its owner's management, and Chromium reads none of
# Chrome's policies. It takes its shape from the terminal kit
# (../terminal-kit), so the source is cut from the folder holding both, and
# built from this one.
{
  lib,
  callPackage,
  chromium,
  libnotify,
  makeWrapper,
  rustPlatform,
  systemd,
  v4l-utils,
  weston,
}:
let
  calls = callPackage ../calls/package.nix { };
in
rustPlatform.buildRustPackage rec {
  pname = "call-under-load";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./page.html
      ../terminal-kit/Cargo.toml
      ../terminal-kit/src
    ];
  };
  sourceRoot = "${src.name}/call-under-load";
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  postInstall = ''
    install -Dm644 page.html $out/share/call-under-load/page.html
    mkdir -p $out/libexec/call-under-load
    ln -s ${v4l-utils}/bin/v4l2-ctl $out/libexec/call-under-load/camera-check
    wrapProgram $out/bin/call-under-load \
      --prefix PATH : $out/libexec/call-under-load \
      --prefix PATH : ${
        lib.makeBinPath [
          calls
          chromium
          libnotify
          systemd
          weston
        ]
      }
  '';
  meta = {
    description = "Measures Joel's calls and apps under the machine's load, each hour";
    license = lib.licenses.mit;
    mainProgram = "call-under-load";
    platforms = lib.platforms.linux;
  };
}
