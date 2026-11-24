# The xwayland-satellite niri starts whenever an X11 program needs a
# display, under its own name, so niri finds it on PATH. It passes niri a
# window's class but not its instance, the only name some programs give
# each of their windows, so the patch (untitled-instance.patch) titles a
# window that has no name with its instance, and its tests, which start an
# Xwayland of their own, run as it builds. It panics when the compositor
# falls behind enough to fill its socket, as under a storm of windows. The
# patch applied after it (newer-backend.patch) takes the wayland-backend
# whose buffer grows while the socket is full, so its vendored crates are
# fetched again, and the third (flush-when-full.patch) waits for room
# rather than panicking, leaving what it can't send to the main loop,
# which now wakes when the socket has room; its test fills the socket
# against a stand-in compositor. The fourth (drain-buffered-events.patch),
# upstream's own fix not yet released, handles the X11 events xcb read
# while the loop waited for a reply before it sleeps, so a window opened
# then isn't left unseen until its program does something else. The script
# in front of it (xwayland-satellite.sh) gives Xwayland a cookie, so only
# Joel's programs connect. nixos.nix installs it, and checks.nix starts it on a nested niri.
{
  coreutils,
  rustPlatform,
  writeShellApplication,
  xauth,
  xwayland,
  xwayland-satellite,
}:
let
  satellite = xwayland-satellite.overrideAttrs (old: {
    patches = (old.patches or [ ]) ++ [
      ./untitled-instance.patch
      ./newer-backend.patch
      ./flush-when-full.patch
      ./drain-buffered-events.patch
    ];
    cargoDeps = rustPlatform.fetchCargoVendor {
      inherit (old) pname src version;
      patches = [ ./newer-backend.patch ];
      hash = "sha256-WgaUvUmbuDm0cFbqXdHQNpD299YoJo2yymI2HT6ItFY=";
    };
    doCheck = true;
    nativeCheckInputs = (old.nativeCheckInputs or [ ]) ++ [ xwayland ];
    cargoTestFlags = [
      "--test"
      "integration"
      "--lib"
    ];
    # Each runs the tests whose names hold it. Every test in server::tests
    # runs satellite's main loop and its flush against a stand-in compositor,
    # with no Xwayland, so they all run; the integration tests start an
    # Xwayland each, so only those of the first patch.
    checkFlags = [
      "title"
      "instance"
      "class_alone"
      "server::tests::"
    ];
    preCheck = ''
      export XDG_RUNTIME_DIR=$(mktemp -d)
    '';
  });
in
writeShellApplication {
  name = "xwayland-satellite";
  runtimeInputs = [
    coreutils
    xauth
  ];
  runtimeEnv.SATELLITE = "${satellite}/bin/xwayland-satellite";
  text = builtins.readFile ./xwayland-satellite.sh;
}
