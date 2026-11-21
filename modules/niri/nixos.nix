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
    # frame. It says which animation moving the tile began last and how
    # long before the report, so the margin cat can follow a moving edge
    # in step with niri rather than a frame or more behind.
    #
    # niri casts a screen in every pixel of its mode, so a call took the
    # 4K monitor as 3840x2160, and Tuple, which ignores the size its viewer
    # asks for, squeezed it into 10 Mbit/s. The patch
    # (cast-logical-size.patch) casts a scaled screen at its logical size,
    # 2560x1440 there, as a Mac shares one; windows' casts keep theirs.
    #
    # Tuple's Linux client asks a screen's cast for no pointer and sends
    # none of its own, so the person watching never saw where Joel
    # pointed. The patch (hidden-pointer-drawn.patch) draws the pointer
    # into any cast that asks for it hidden. The portal names no app for
    # niri to tell Tuple's casts apart by, so every program asking that
    # gets it drawn too.
    #
    # niri asks the GPU to draw its frames at high priority, which xe grants
    # only to a process holding CAP_SYS_NICE, so without it the desktop's
    # frames queued beside Chrome's. A wrapper below gives it, putting it
    # in the inheritable and ambient sets too, which every program niri
    # starts would inherit; the patch (keep-capabilities.patch) clears
    # them as niri starts, so niri alone keeps it.
    #
    # niri sends its sRGB frames to a screen as they are, so a wide-gamut
    # panel, the XPS 14's OLED at 1.55 times sRGB's area, stretched every
    # colour out to its own and beside an sRGB monitor made that look
    # washed out. The patch (srgb-gamut.patch, last, since it's made on
    # the rest) gives an output `srgb-gamut`, which maps sRGB to the
    # primaries its EDID declares in the display engine: a degamma curve,
    # a colour matrix and a gamma curve, at no cost to drawing.
    #
    # niri darkens a laptop's own panel when its lid shuts only while
    # another monitor is connected, so a laptop kept awake shut with
    # nothing plugged in, working for others, lit a panel nobody could
    # see. The patch (shut-lid-darkens-the-panel.patch) darkens it either
    # way; opening the lid lights it again. A lid's switch can read shut
    # with the lid open, so while no screen is lit a key, a click or a
    # pointer moved takes the lid as open.
    #
    # niri draws its first frame the moment it starts, its bare backdrop,
    # so the boot splash's picture went dark for the second or more until
    # the shell's wallpaper was drawn. The patch (hold-first-frame.patch,
    # made on the rest) holds it when niri starts with NIRI_HOLD_FIRST_FRAME
    # set, which home.nix sets, never through the config, which a running
    # niri older than the patch would refuse whole: on a screen of its own, not nested in a window, an output's first
    # frame waits until a background or overlay layer surface covering it
    # maps, the wallpaper or the greeter, for at most 5 seconds; a smaller
    # one, such as the closed launcher's single pixel or the margin's strip,
    # would show the bare backdrop around it, so it doesn't count. Never while
    # the session locks, so the screen keeps what the splash left until
    # then. Its tests run as niri builds, and the screens test, whose niri is
    # on its screens as on a laptop, reads that it held.
    #
    # The driver sends a monitor limited-range RGB at a TV mode, such as
    # 3840x2160 at 60 Hz, and one reading full range shows grey blacks. The
    # patch (full-range-when-selectable.patch, made on the rest) sends full
    # range to a screen whose EDID says its range is selectable, when niri
    # starts with NIRI_FULL_RANGE_WHEN_SELECTABLE set, which home.nix sets;
    # any other screen is left to the driver.
    #
    # niri warps the pointer to the next window when the focused one goes,
    # which, when its program quit or died, it did from inside that client's
    # destruction: the pointer's leave went to a wl_pointer already gone,
    # and niri aborted, taking the session with it. The patch
    # (warp-after-a-close.patch) warps once the client is gone.
    #
    # niri drops every action asked for over its socket while the session
    # is locked, and answers that it handled it, so a session another
    # session handed the shell then was lost outright. The patch
    # (spawn-behind-the-lock.patch) starts a spawn asked for then behind
    # the lock, with no activation token; every other action, and every
    # key binding, is still refused while locked.
    #
    # niri counts only its own input as activity, and a virtual keyboard's
    # keys go straight to the window, so typing through one, as dictation
    # does through wtype, never kept the session awake: five minutes after
    # the last real key the screen locked mid-sentence. The patch
    # (virtual-keys-are-activity.patch) counts each such key as activity
    # while the monitors are on, as a real key counts; it doesn't turn
    # them back on.
    #
    # niri connects every screen as it starts and hears the lid only after,
    # so a laptop started on a dock with its lid shut lit its own panel and
    # laid the monitor out beside it, then took the panel away 67 ms later.
    # The patch (lid-read-before-outputs.patch) asks logind whether the
    # lid is shut before any screen connects, so a shut panel never is,
    # waiting a second at most; the screens test starts with its stand-in
    # lid shut to show it. It doesn't cover a monitor that comes up only
    # after niri has started, as one behind an MST hub can: until it does,
    # niri has the shut panel alone and keeps it dark.
    #
    # niri opens a window on a workspace it's told of only through a rule
    # naming a named one, so a session one session started opened on the
    # workspace Joel was on, and the shell moved it after, a flash and a
    # column taken from him. The patch (opens-next-window-on.patch) adds
    # OpenNextWindowOn, which says the next window with an app ID opens
    # on a workspace, by its id, from its first frame and without the
    # focus; it's used once and forgotten after 30 seconds. niri moves only
    # the focused column to an index, so the next patch
    # (opens-next-window-as-column.patch) lets it say the index it opens
    # at as a new column, which Reopen Them All puts each session back at.
    #
    # niri gives an on-demand layer surface the keys only as it first maps
    # or is clicked, so the shell's launcher, kept mapped while closed,
    # asked for them exclusively: a click on a window left them in the
    # launcher, and a surface over it never had them, since between two
    # exclusive surfaces niri takes the first. The patch
    # (on-demand-when-asked.patch) gives them to a mapped surface as it
    # turns on-demand, the newest to ask having them, and back to the one
    # that asked before as it lets go, until a click gives a window them.
    #
    # A laptop panel whose primaries are sRGB's shows niri's sRGB
    # accurately, which beside the vivid profile its own system ships with
    # looked washed out. The patch (panel-saturation.patch, made on
    # srgb-gamut.patch and the rest) shows a laptop's own panel more
    # colourful through the same degamma curve, colour matrix and gamma
    # curve, as much as NIRI_PANEL_SATURATION says, which home.nix sets from
    # jitsusama.niri.panelSaturation; an IPC-only action changes it live.
    #
    # niri walks every window after each wakeup of its loop, and each
    # commit a client makes wakes it, so with 81 windows open, most of them
    # terminals, it woke about 990 times a second and spent 35 to 49% of a
    # core walking them. The patch (refreshes-once-a-frame.patch)
    # walks them at most once a frame of the fastest screen, a timer taking
    # what a wakeup puts off; input, an action, a focus change, a window
    # coming or going and a commit resizing one still have it at once, so a
    # resize's layout reaches the shell within a refresh of the terminal
    # drawing it, not 22 to 39 ms on. Its tests run as niri builds,
    # niri-refreshes-once-a-frame counts its refreshes under a client
    # committing a thousand times a second, and niri-tells-a-resize-at-once
    # times a resize's report.
    #
    # niri powers its monitors off only all at once, and any key lights
    # them all, so a laptop working through the night lit the monitor
    # beside the bed, and its own button cuts the hub and network card it
    # carries. The patch (powers-off-each-output.patch, made on the rest)
    # adds power-off-monitor and power-on-monitor, naming one output as
    # niri msg outputs does: its CRTC goes inactive and it stays connected
    # and laid out. Input leaves it dark while another screen is lit, and
    # lights them all once every one is off by name; a reconnect or niri
    # restarting lights it too. niri msg outputs says which is off, and
    # NIRI_POWERS_OFF_EACH_OUTPUT tells the shell it can ask.
    #
    # niri never told a window it was hidden, and kitty skips drawing only
    # a window told it's suspended, so the kitty holding 74 sessions' windows
    # drew every hidden one, 106 ms of its main thread's CPU each second at
    # the median, on the thread that draws the window being typed in. The
    # patch (suspends-hidden-windows.patch) suspends a window within a
    # second of no output drawing it, unless it's cast, and unsuspends it as
    # a frame holds it again. Its tests run as niri builds.
    #
    # niri renders a frame as soon as the vblank before it passes, and sends
    # each window its frame callback after that render, so a window that
    # answers a key a few milliseconds later waits for the next vblank to be
    # rendered and the one after it to be shown. Joel's keys spent a median
    # 48 ms from their frame's start to the screen on 11 November, of 58 ms
    # in all. The patch (renders-by-the-deadline.patch, upstream's #4692, not yet
    # merged there) renders as late as it can and still make the next
    # vblank, its margin growing when it misses one and shrinking as it
    # doesn't, and renders at once on a VRR screen, after a few idle
    # refreshes, or where its margin passes three quarters of a refresh.
    # Its tests run as niri builds. A lab on vkms, drawing in software,
    # found it presented no sooner, so it's off unless NIRI_RENDER_DEADLINE=1
    # is in niri's environment, which adds no setting a niri before it would
    # refuse. render-deadline-action.patch turns it on and off while niri
    # runs: `niri-deadline on`, below, tries it on Joel's own GPU, and `off`
    # puts it back. Tried so on 18 November, it missed no vblank in 3 hours
    # on a 60 Hz screen and took a median 6 ms off the launcher's frames'
    # trip from swap to screen, but missed 8 in 20 s on the 120 Hz panel.
    # So home.nix starts niri with it on, and the patch made on the rest,
    # renders-by-the-deadline-up-to-a-rate.patch, keeps it to screens of
    # 60 Hz and slower: a faster one renders at once. Over a busy afternoon
    # on 20 November it missed 3,832 vblanks on the 60 Hz screen in 2.5
    # hours, as the GPU's other work came and went: each miss at a new
    # margin let it shrink back into the next within a second.
    # renders-by-the-deadline-holds-after-misses.patch doubles its hold on
    # the margin, up to ten minutes, for a miss soon after the last hold
    # ended at whatever margin, so it gives up a few milliseconds of the
    # gain while the GPU stays busy. A long hold's margin sits at its floor
    # nearly all the time, so a burst within one keeps the margin it climbs
    # to for about a minute, longer if it levels off just above the
    # margin its own last miss left, at whatever level, and the steady
    # need's hold then grows back.
    # One soon after a hold ended can't be told from a need that came
    # back, so it keeps its margin for up to ten minutes, which costs a few
    # milliseconds but no misses.
    #
    # niri tells a sandbox's client apart only to hide globals from it, so
    # a window a fenced command opened through pi-fence-display took the
    # focus on the workspace Joel was using, and a token from his window
    # let it take it again; no window rule can single out a sandbox, since
    # it names its own app ID. The patch
    # (sandboxed-windows-stay-behind.patch) opens a window of any
    # wp-security-context-v1 client on the empty workspace at the end of
    # the focused output, adding one after it while Joel is there, never
    # activates it, and lets an activation reach it as urgency at most. It
    # rests on the connection rather than a setting, so it adds none.
    #
    # Such a client was still offered ext_idle_notifier_v1, which says
    # when the person stops touching the machine and comes back, so a
    # program in a sandbox could wait for nobody watching. smithay's idle
    # notifier takes no client filter, so the patch
    # (sandbox-sees-no-idle.patch) dispatches its global in niri, as
    # smithay's macro would, hiding it from those clients. It adds no
    # setting either.
    #
    # The launcher's frame reached the screen a median 6 refreshes after
    # the shell's swap at 120 Hz, though niri's code queues a redraw as a
    # layer surface commits and renders it at once, so where the time
    # goes can't be read from the code. The patch
    # (says-a-frames-steps-after-idle.patch, made on the rest) says at
    # debug, for each frame after 100 ms idle, when its redraw was asked,
    # how long its render waited and took, and how long it waited for the
    # screen once queued, by the clock the shell's own times use. It adds
    # no setting.
    #
    # A sandbox's window that had the keyboard was still offered the
    # clipboard and the primary selection through its data device, as any
    # focused window is, so a copy a password manager marked secret
    # reached it. The patch (keeps-secret-copies-from-sandboxes.patch)
    # offers a selection holding such a mark to no wp-security-context-v1
    # client, as the clipboard door judges the mark, while ordinary copies
    # and Joel's own windows are served as before. It adds no setting.
    #
    # niri draws each bottom layer surface once on every workspace, moved
    # with it, so as a switch began a second margin cat rode in on the
    # incoming workspace before the shell had heard of the switch. The
    # patch (fixed-to-output.patch) draws a bottom surface whose namespace
    # NIRI_FIXED_TO_OUTPUT names once on its screen, where it is, outside
    # the overview; home.nix's unit names the cat's. It adds no setting.
    #
    # The launcher stays mapped while it's shut, its blur region empty, so
    # niri draws no blur for it then, and smithay's damage tracker lets go
    # of the framebuffer and blur textures, keeping an effect's only while a
    # frame draws it. Each opening made all five again inside the frame that
    # showed it: 22 to 48 ms of niri's render on Joel's screen under load,
    # and 36 to 56 ms on its GPU for a first set in a lab. The patch
    # (blur-kept-for-its-size.patch) keeps the textures a blur let go, up
    # to 64 MB on each renderer and for as long as nothing pushes them out,
    # and the next blur of their size takes them back; the launcher's set is
    # about 21 MB on a 4K screen at 1.5, held while it's shut, though two
    # or more sets of sizes it hasn't seen, other surfaces' or its own at
    # other heights, as a talk place or a viewer's command line makes, push
    # it out. A
    # set past half the pool, as a full-screen prompt's is, is never kept,
    # so it can't push out the rest. It says at debug, which the session
    # logs, each time a blur's textures change, how many it made and took
    # and how long that took, and each time the pool lets some go. It adds
    # no setting.
    #
    # niri's own frames after idle reach the screen within 2 refreshes of
    # its redraw, yet the launcher's reached it 4 to 7 after the shell's
    # swap, so the time goes before niri is asked to redraw: behind the
    # buffer's fence, or a frame already in flight. The patch
    # (says-a-layers-wake.patch, made on the rest) says at debug, for a
    # layer surface's first buffer after 100 ms quiet, how long niri took
    # to take it, what the output was doing then, when its frame rendered
    # and reached the screen, and how fast the screen refreshed over the
    # spell before. It adds no setting.
    package = pkgs.niri.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [
        ./touchscreen-gestures.patch
        ./prune-imports.patch
        ./tiled-positions.patch
        ./cast-logical-size.patch
        ./hidden-pointer-drawn.patch
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
        ./shut-lid-darkens-the-panel.patch
        ./srgb-gamut.patch
        ./hold-first-frame.patch
        ./full-range-when-selectable.patch
        ./warp-after-a-close.patch
        ./spawn-behind-the-lock.patch
        ./virtual-keys-are-activity.patch
        ./lid-read-before-outputs.patch
        ./opens-next-window-on.patch
        ./opens-next-window-as-column.patch
        ./on-demand-when-asked.patch
        ./panel-saturation.patch
        ./refreshes-once-a-frame.patch
        ./powers-off-each-output.patch
        ./suspends-hidden-windows.patch
        ./renders-by-the-deadline.patch
        ./render-deadline-action.patch
        ./sandboxed-windows-stay-behind.patch
        ./sandbox-sees-no-idle.patch
        ./says-a-frames-steps-after-idle.patch
        ./keeps-secret-copies-from-sandboxes.patch
        ./fixed-to-output.patch
        ./says-a-layers-wake.patch
        ./renders-by-the-deadline-up-to-a-rate.patch
        ./renders-by-the-deadline-holds-after-misses.patch
        ./blur-kept-for-its-size.patch
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
    serviceConfig = {
      ExecStart = [
        ""
        "${config.security.wrapperDir}/niri-compositor --session"
      ];
      # First inside session.slice with the shell, ahead of the portals and
      # the bus at their tenth, and its memory held hard: in swap, it read
      # keys up to 281 ms late (modules/slices). It used 126 MB on 29
      # October.
      CPUWeight = 1000;
      MemoryMin = "256M";
    };
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
    # Switches renders-by-the-deadline.patch's deadline on and off while
    # niri runs (niri-deadline.sh), through the installed niri's own msg.
    (pkgs.writeShellApplication {
      name = "niri-deadline";
      runtimeEnv.NIRI = "${config.programs.niri.package}/bin/niri";
      text = builtins.readFile ./niri-deadline.sh;
    })
    # What niri starts whenever an X11 program needs a display, with the
    # cookie only Joel's programs hold (xwayland-satellite.nix).
    (pkgs.callPackage ./xwayland-satellite.nix { })
    pkgs.brightnessctl
    # wl-copy and wl-paste, which programs in a terminal reach the clipboard
    # through: Neovim's + register, and pi's copying and pasting of text and
    # images.
    pkgs.wl-clipboard
  ];
}
