# niri, the scrolling, tiling compositor Joel works in, and what it needs from
# the system: its session, its portals, Wayland for Chromium and Electron
# programs, Xwayland for the X11 programs left, the tool its brightness keys
# call, and the clipboard's. Its media keys go to the shell (Quickshell's
# Music.qml). Its touchscreen takes the touchpad's gestures too.
{
  config,
  lib,
  pkgs,
  ...
}:
{
  programs.niri = {
    enable = true;
    # niri 26.04 has no touchscreen gestures, and a reader beside it can't
    # keep a swipe's touches from the window under it, which took them as a
    # tap. The patch (touchscreen-gestures.patch says what it holds) lets
    # niri swipe the way its touchpad does and cancels the window's touches
    # once a swipe begins.
    #
    # niri 26.04 stamps a screencast's every frame with the same time, so
    # GStreamer, which the recorder hands a window's screencast to
    # (modules/quickshell/record-window), takes them all as one moment.
    # Upstream's a30ca79, after 26.04, leaves the time unsaid, so each is
    # timed as it comes.
    #
    # niri 26.04 drops what it imported from a client's buffers only after
    # drawing a frame, so with the monitors off every new buffer stayed, and
    # a locked laptop's terminal piled up 66 GB in three hours. The patch
    # (prune-imports.patch) clears them out as each new buffer arrives, and
    # upstream's bb38b7e, after 26.04, stops asking clients to draw while
    # the monitors are off.
    #
    # niri tells clients where a floating window sits but not a tiled one,
    # so the shell couldn't keep a notice inside the window under it. The
    # patch (tiled-positions.patch) reports a tiled window's place too,
    # where the view comes to rest, so a scroll sends one change, not one a
    # frame.
    #
    # niri casts a screen in every pixel of its mode, so a call took the
    # 4K monitor as 3840x2160, and Tuple, which ignores the size its viewer
    # asks for, squeezed it into 10 Mbit/s. The patch
    # (cast-logical-size.patch) casts a scaled screen at its logical size,
    # 2560x1440 there, as a Mac shares one; windows' casts keep theirs.
    #
    # niri asks the GPU to draw its frames at high priority, which xe grants
    # only to a process holding CAP_SYS_NICE, so without it the desktop's
    # frames queued beside Chrome's. A wrapper below gives it, putting it
    # in the inheritable and ambient sets too, which every program niri
    # starts would inherit; the patch (keep-capabilities.patch) clears
    # them as niri starts, so niri alone keeps it.
    package = pkgs.niri.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [
        ./touchscreen-gestures.patch
        ./prune-imports.patch
        ./tiled-positions.patch
        ./cast-logical-size.patch
        ./keep-capabilities.patch
        (pkgs.fetchpatch {
          name = "niri-screencast-pts-unknown.patch";
          url = "https://github.com/niri-wm/niri/commit/a30ca7983b2f1fc3ddeb209b3fe18fa78e0dbd25.patch";
          hash = "sha256-fywWXRRC7bFsX9uytntQLEWUNG1oyxUMn575hAG6Oow=";
        })
        (pkgs.fetchpatch {
          name = "niri-no-frame-callbacks-without-monitors.patch";
          url = "https://github.com/niri-wm/niri/commit/bb38b7e14f8f7bbef6584cf263bf36b0044d63d3.patch";
          hash = "sha256-83i8Mpyc6k4BqBjPYEY/M4XnEbxBb4RKiFJ7tk4mXfM=";
        })
      ];
    });
    # The GTK file chooser, rather than installing Nautilus to provide one.
    useNautilus = false;
  };

  # niri with CAP_SYS_NICE, which the session's niri starts through, under a
  # name of its own so `niri msg` and the rest stay the plain program.
  security.wrappers.niri-compositor = {
    owner = "root";
    group = "root";
    capabilities = "cap_sys_nice+ep";
    source = "${config.programs.niri.package}/bin/niri";
  };
  # niri asks the GPU for a high-priority context, and its own log filter
  # hides smithay's word on whether it got one, so its default filter is
  # kept with that one module's lines let through: one per context.
  systemd.user.services.niri = {
    overrideStrategy = "asDropin";
    serviceConfig.ExecStart = [
      ""
      "${config.security.wrapperDir}/niri-compositor --session"
    ];
    environment.RUST_LOG = lib.concatStringsSep "," [
      "niri=debug"
      "smithay::backend::renderer::gles=error"
      "smithay::backend::egl::context=info"
    ];
  };

  # nixpkgs's wrappers for Chromium and Electron programs, such as Chrome,
  # Slack and 1Password, ask for Wayland only when this is set, and Electron
  # doesn't ask on its own. Through Xwayland, every monitor would share the
  # smallest one's DPI, and each program would scale itself from it.
  environment.sessionVariables.NIXOS_OZONE_WL = "1";

  environment.systemPackages = [
    # niri starts it whenever an X11 program needs a display. It passes
    # niri a window's class but not its instance, the only name some
    # programs give each of their windows, so the patch
    # (untitled-instance.patch) titles a window that has no name with its
    # instance, and its tests, which start an Xwayland of their own, run as
    # it builds.
    (pkgs.xwayland-satellite.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [ ./untitled-instance.patch ];
      doCheck = true;
      nativeCheckInputs = (old.nativeCheckInputs or [ ]) ++ [ pkgs.xwayland ];
      cargoTestFlags = [
        "--test"
        "integration"
      ];
      checkFlags = [
        "title"
        "instance"
        "class_alone"
      ];
      preCheck = ''
        export XDG_RUNTIME_DIR=$(mktemp -d)
      '';
    }))
    pkgs.brightnessctl
    # wl-copy and wl-paste, which programs in a terminal reach the clipboard
    # through: Neovim's + register, and pi's copying and pasting of text and
    # images.
    pkgs.wl-clipboard
  ];
}
