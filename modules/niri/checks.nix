# The checks of niri, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # The session's niri starts through the wrapper giving it CAP_SYS_NICE,
  # without which the GPU won't draw its frames first; the desktop test
  # checks what niri holds once started, and what it hands on.
  niri-starts-able-to-go-first =
    let
      inherit (graphical) config;
      wrapper = config.security.wrappers.niri-compositor;
    in
    holds "niri-starts-able-to-go-first"
      "The session's niri doesn't start through its wrapper with CAP_SYS_NICE, so the GPU draws its frames in turn with everything else's."
      (
        wrapper.capabilities == "cap_sys_nice+ep"
        && wrapper.source == "${config.programs.niri.package}/bin/niri"
        &&
          lib.hasInfix "ExecStart=${config.security.wrapperDir}/niri-compositor --session"
            config.systemd.user.units."niri.service".text
      );

  # Whether the GPU gave niri the priority it asked for is smithay's to say,
  # at info, which niri's own log filter drops; the session's niri lets
  # that one module through, and keeps the rest of the filter niri's source
  # sets, so a niri that changes it is caught here.
  niri-says-what-the-gpu-gave-it =
    let
      inherit (graphical) config;
      filter = config.systemd.user.services.niri.environment.RUST_LOG or "";
      kept = lib.removeSuffix ",smithay::backend::egl::context=info" filter;
    in
    pkgs.runCommandLocal "niri-says-what-the-gpu-gave-it" { } ''
      if [ ${lib.escapeShellArg kept} = ${lib.escapeShellArg filter} ]; then
        echo "The session's niri doesn't let smithay's EGL contexts through its log filter, so its journal can't say whether the GPU gave it a high priority." >&2
        exit 1
      fi
      grep -qF 'const DEFAULT_LOG_FILTER: &str = "'${lib.escapeShellArg kept}'";' \
        ${config.programs.niri.package.src}/src/main.rs || {
        echo "The session's niri sets a log filter other than niri's own default with smithay's EGL contexts added." >&2
        exit 1
      }
      grep -qF 'RUST_LOG=${filter}' ${
        pkgs.writeText "niri.service" config.systemd.user.units."niri.service".text
      } || {
        echo "The session's niri unit doesn't carry its log filter." >&2
        exit 1
      }
      touch $out
    '';

  # Each screen has one output block in the account's config.kdl, the
  # panel's holding srgb-gamut, since niri reads only a screen's first block;
  # and a machine adding a second block for a screen as text is refused.
  niri-names-each-screen-once =
    let
      refused =
        home:
        lib.any (check: !check.assertion && lib.hasInfix "output block" check.message) home.assertions;
      home = account graphical;
      twice = account (
        graphical.extendModules {
          modules = [
            {
              home-manager.users.someone.xdg.configFile."niri/config.kdl".text = lib.mkAfter ''
                output "eDP-1" {
                    scale 2
                }
              '';
            }
          ];
        }
      );
      panel = lib.count (line: line == ''output "eDP-1" {'') (
        lib.splitString "\n" home.xdg.configFile."niri/config.kdl".text
      );
    in
    holds "niri-names-each-screen-once"
      "The panel needs one output block holding srgb-gamut, and a second block for a screen must be refused."
      (
        !refused home
        && refused twice
        && panel == 1
        && lib.elem "srgb-gamut" home.jitsusama.niri.outputs."eDP-1"
      );

  # niri's unit asks for full-range RGB on a screen that takes it by the
  # name full-range-when-selectable.patch reads, so the ask isn't lost to a
  # rename on either side.
  niri-is-asked-for-full-range =
    let
      unit =
        (account graphical).xdg.configFile."systemd/user/niri.service.d/full-range.conf" or {
          text = "";
        };
      patch = builtins.readFile ./full-range-when-selectable.patch;
    in
    holds "niri-is-asked-for-full-range"
      "niri's unit doesn't ask for full-range RGB, or asks by a name full-range-when-selectable.patch doesn't read."
      (
        lib.hasInfix ''var_os("NIRI_FULL_RANGE_WHEN_SELECTABLE")'' patch
        && lib.hasInfix "Environment=NIRI_FULL_RANGE_WHEN_SELECTABLE=1" unit.text
      );

  # niri's unit fixes to the screen the namespace the margin cat's surface
  # gives, so a rename on either side doesn't bring back the second cat
  # riding in on a switch (fixed-to-output.patch).
  niri-fixes-the-cat-to-its-screen =
    let
      unit =
        (account graphical).xdg.configFile."systemd/user/niri.service.d/fixed-to-output.conf" or {
          text = "";
        };
    in
    holds "niri-fixes-the-cat-to-its-screen"
      "niri's unit doesn't name the margin cat's namespace in NIRI_FIXED_TO_OUTPUT, so a switch draws it once on each workspace."
      (
        lib.hasInfix ''WlrLayershell.namespace: "cat"'' (
          builtins.readFile ../margin-cat/MarginCatScreen.qml
        )
        && lib.hasInfix "Environment=NIRI_FIXED_TO_OUTPUT=cat\n" unit.text
      );

  # niri's unit asks for a laptop panel's saturation by the name
  # panel-saturation.patch reads, only on a machine that sets one, so the ask
  # isn't lost to a rename on either side.
  niri-is-asked-for-panel-saturation =
    let
      unit =
        home:
        (home.xdg.configFile."systemd/user/niri.service.d/panel-saturation.conf" or { text = ""; }).text;
      vivid = account (
        graphical.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.niri.panelSaturation = 1.25; } ];
        }
      );
      patch = builtins.readFile ./panel-saturation.patch;
    in
    holds "niri-is-asked-for-panel-saturation"
      "niri's unit doesn't ask for the panel's saturation a machine sets, asks on one that sets none, or asks by a name panel-saturation.patch doesn't read."
      (
        lib.hasInfix ''var_os("NIRI_PANEL_SATURATION")'' patch
        && lib.hasInfix "Environment=NIRI_PANEL_SATURATION=1.25" (unit vivid)
        && unit (account graphical) == ""
      );

  # niri's unit asks for the render deadline, and keeps it to screens of
  # 60 Hz and slower, by the names the two patches read, and niri is built
  # with both, so neither a rename on either side nor a patch dropped from
  # the list lets it reach the 120 Hz panel, where it missed vblanks. Which
  # rates defer is proven in niri's own tests, as it builds.
  niri-renders-by-the-deadline-up-to-60-hz =
    let
      unit =
        ((account graphical).xdg.configFile."systemd/user/niri.service.d/render-deadline.conf" or {
          text = "";
        }
        ).text;
      deadline = builtins.readFile ./renders-by-the-deadline.patch;
      rate = builtins.readFile ./renders-by-the-deadline-up-to-a-rate.patch;
      built = map (patch: baseNameOf (toString patch)) graphical.config.programs.niri.package.patches;
    in
    holds "niri-renders-by-the-deadline-up-to-60-hz"
      "niri isn't built with both render deadline patches, or its unit doesn't ask for the deadline, doesn't keep it to screens of 60 Hz and slower, or asks by a name the patches don't read."
      (
        lib.elem "renders-by-the-deadline.patch" built
        && lib.elem "renders-by-the-deadline-up-to-a-rate.patch" built
        && lib.hasInfix ''var_os("NIRI_RENDER_DEADLINE")'' deadline
        && lib.hasInfix ''var("NIRI_RENDER_DEADLINE_UP_TO_HZ")'' rate
        && lib.hasInfix "Environment=NIRI_RENDER_DEADLINE=1\n" unit
        && lib.hasInfix "Environment=NIRI_RENDER_DEADLINE_UP_TO_HZ=60\n" unit
      );

  # niri reads its settings again in place when a switch changes them, and
  # refuses all of them for one it doesn't know. The niri running through a
  # switch is the one the machine booted, from before the switch, so a
  # patch adding a setting leaves it refusing every change until the next
  # boot, as hold-first-frame.patch once did. A patch asks through niri's
  # environment or its command line instead. srgb-gamut.patch's setting is
  # in every niri booted since it came in. opens-next-window-on.patch, and
  # opens-next-window-as-column.patch on it, add to niri-config only an
  # action marked knuffel(skip), and a field of it, which IPC alone takes
  # and the config can't name, so the config a niri before it reads is the
  # same; such a niri refuses the action, and the shell moves the window
  # after, as it did before. panel-saturation.patch adds the same kind of
  # action, SetPanelSaturation, and asks for its value through the
  # environment. render-deadline-action.patch adds one more,
  # SetRenderDeadline, for the deadline renders-by-the-deadline.patch takes
  # from the environment; a niri before it refuses only the action, which
  # niri-deadline then says. powers-off-each-output.patch adds two actions
  # a bind may name, power-off-monitor and power-on-monitor, and no setting:
  # the config a niri before it reads is the same while no bind names
  # them, and the shell asks for them over IPC only once niri says it can.
  niri-patches-add-no-settings =
    let
      kept = [
        "srgb-gamut.patch"
        "opens-next-window-on.patch"
        "opens-next-window-as-column.patch"
        "panel-saturation.patch"
        "render-deadline-action.patch"
        "powers-off-each-output.patch"
      ];
      patches = lib.filter (
        patch: !lib.elem (baseNameOf (toString patch)) kept
      ) graphical.config.programs.niri.package.patches;
    in
    pkgs.runCommandLocal "niri-patches-add-no-settings" { } ''
      for patch in ${lib.escapeShellArgs (map (patch: "${patch}") patches)}; do
        if grep -q '^+++ b/niri-config/' "$patch"; then
          echo "$patch adds a setting to niri's config, which a niri started before it refuses whole at a switch, and every change with it; ask through niri's environment instead." >&2
          exit 1
        fi
      done
      touch $out
    '';

  # A window alone on its workspace sits in the middle of the screen, for
  # every program, rather than against the left edge: niri's layout block
  # says always-center-single-column, and nothing comments it out.
  niri-centres-a-window-alone =
    let
      text = (account graphical).xdg.configFile."niri/config.kdl".text;
      after = lib.splitString "\nlayout {\n" text;
      layout = lib.head (lib.splitString "\n}\n" (lib.last after));
    in
    holds "niri-centres-a-window-alone"
      "niri's layout block in config.kdl no longer says always-center-single-column, so a window alone sits against the screen's left edge."
      (lib.length after == 2 && lib.elem "    always-center-single-column" (lib.splitString "\n" layout));

  # A shared screen shows the launcher, which Joel shows people his desktop
  # through on a call, and never polkit's password, askpass's PIN or a
  # notice: no layer rule keeping a surface out of a share or a capture
  # names the launcher's namespace, and the last rule saying what each of
  # the others is kept out of, as niri reads them, keeps it out of every
  # capture, a recorder's screencopy as much as a call's screencast, since
  # a recording is a share made later (5GLTPH, 2026-11-17). The rules are
  # what this holds; a capture that comes out black is the nested niri's
  # to show.
  niri-shares-the-launcher-but-no-password =
    let
      text = (account graphical).xdg.configFile."niri/config.kdl".text;
      rules = map (block: lib.head (lib.splitString "\n}\n" block)) (
        lib.tail (lib.splitString "\nlayer-rule {\n" text)
      );
      # What a rule keeps a surface out of, by its own last block-out-from
      # line, never one commented out; null where it keeps it out of
      # nothing.
      blockOut =
        rule:
        let
          said = lib.concatMap (
            line:
            let
              from = builtins.match " *block-out-from \"([a-z-]+)\".*" line;
            in
            if from == null then [ ] else from
          ) (lib.splitString "\n" rule);
        in
        if said == [ ] then null else lib.last said;
      patterns =
        rule:
        lib.concatMap (
          line:
          let
            named = builtins.match " *match namespace=\"([^\"]*)\".*" line;
          in
          if named == null then [ ] else named
        ) (lib.splitString "\n" rule);
      # niri searches for a rule's pattern within the namespace, where
      # builtins.match matches the whole of it.
      matches = namespace: pattern: builtins.match ".*(${pattern}).*" namespace != null;
      names = namespace: rule: lib.any (matches namespace) (patterns rule);
      # niri reads its rules in order, so the last one saying decides.
      keptOutOf =
        namespace:
        let
          saying = lib.filter (rule: blockOut rule != null && names namespace rule) rules;
        in
        if saying == [ ] then null else blockOut (lib.last saying);
      wrong =
        lib.optional (keptOutOf "launcher" != null) "the launcher"
        ++ lib.filter (namespace: keptOutOf namespace != "screen-capture") [
          "polkit"
          "askpass"
          "notifications"
        ];
      meant = "the launcher shows in a share, and polkit, askpass and the notices in no capture";
    in
    holds "niri-shares-the-launcher-but-no-password"
      "config.kdl misplaces ${lib.concatStringsSep ", " wrong}: ${meant}."
      (wrong == [ ]);

  # niri reads back the files an account gets and rejects anything it
  # wouldn't load, from a misspelt action to a colour it can't parse.
  niri-accepts-its-configuration =
    let
      files = (account graphical).xdg.configFile;
    in
    # The session's own niri reads it, patches and all, since an option one
    # of them adds is unknown to niri as it ships.
    pkgs.runCommandLocal "niri-accepts-its-configuration"
      { nativeBuildInputs = [ graphical.config.programs.niri.package ]; }
      ''
        mkdir niri
        cp ${files."niri/config.kdl".source} niri/config.kdl
        cp ${files."niri/theme.kdl".source} niri/theme.kdl
        cp ${files."niri/binds.kdl".source} niri/binds.kdl
        cp ${files."niri/keyboard.kdl".source} niri/keyboard.kdl
        niri validate --config niri/config.kdl
        touch $out
      '';

  # The Copilot key types, in the keymap niri builds from its account's
  # files with the libxkbcommon it links, the key its dictation bind names
  # (tests/copilot-key-types-its-bind.py). xkeyboard-config 2.49 moved it
  # from F23 to XF86TouchpadOff, leaving the bind matching nothing where
  # only a desktop VM test saw; a new one that moves it fails here.
  niri-hears-the-copilot-key =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
    in
    pkgs.runCommandLocal "niri-hears-the-copilot-key" { } ''
      ${lib.concatLines (
        lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
      )}
      ${lib.getExe pkgs.python3} ${./tests/copilot-key-types-its-bind.py} \
        ${graphical.pkgs.libxkbcommon}/lib/libxkbcommon.so.0 niri
      touch $out
    '';

  # niri walks every window at most once a frame, however often a client's
  # commits wake it (refreshes-once-a-frame.patch): its real niri, with the
  # account's configuration, nested in a headless sway in the sandbox
  # (tests/nested-niri.sh), counts its refreshes in its log while a
  # stand-in commits a thousand times a second
  # (tests/refreshes-once-a-frame.py). niri's own tests say what it puts
  # off and what it doesn't; this shows the count falls on a running niri.
  niri-refreshes-once-a-frame =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-refreshes-once-a-frame"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.python3
          pkgs.sway-unwrapped
        ];
        # niri draws in software, as the desktop's machine does.
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        # niri's own filter, with the line tallying its refreshes let through.
        RUST_LOG = "niri=debug,refresh_pacing=debug";
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/refreshes-once-a-frame.py logs/niri.log
        touch $out
      '';

  # A window's resize reaches IPC within a refresh of the commit that takes
  # it, rather than a paced frame later (refreshes-once-a-frame.patch), and
  # its layout says a resize is moving the tile and how long ago it began,
  # longer on a later report (tiled-positions.patch): the margin cat rides
  # a window's edge from those reports. The account's niri, nested in the
  # sandbox, resizes a stand-in window over and over
  # (tests/tells-a-resize-at-once.py).
  niri-tells-a-resize-at-once =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-tells-a-resize-at-once"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.python3
          pkgs.sway-unwrapped
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/tells-a-resize-at-once.py
        touch $out
      '';

  # A bottom layer surface the unit's NIRI_FIXED_TO_OUTPUT names is drawn
  # once on its screen, where it is, through a workspace switch, rather than
  # once on every workspace moving with it (fixed-to-output.patch): the
  # margin cat is one, so no second cat rides in on the next workspace. The
  # account's niri, with its animations slowed tenfold so the switch is
  # caught partway, nests in the sandbox and gets a stand-in window and
  # surface (tests/draws-a-fixed-surface-once.py).
  niri-draws-a-fixed-surface-once =
    let
      slowed = account (
        graphical.extendModules {
          modules = [ { home-manager.users.someone.jitsusama.theme.motion.slowdown = "10"; } ];
        }
      );
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) slowed.xdg.configFile;
      unit = slowed.xdg.configFile."systemd/user/niri.service.d/fixed-to-output.conf".text;
      asked = "Environment=NIRI_FIXED_TO_OUTPUT=";
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-draws-a-fixed-surface-once"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          (pkgs.python3.withPackages (python: [ python.pillow ]))
          pkgs.sway-unwrapped
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        # What the session's unit gives niri, read from the unit itself.
        NIRI_FIXED_TO_OUTPUT = lib.removePrefix asked (
          lib.findFirst (lib.hasPrefix asked) "" (lib.splitString "\n" unit)
        );
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/draws-a-fixed-surface-once.py shots
        touch $out
      '';

  # A layer surface the account's rule blurs, as the launcher's, takes back
  # the textures its blur let go while its blur region was empty, rather
  # than making them again in the frame that shows it
  # (blur-kept-for-its-size.patch). The account's niri, nested in the
  # sandbox, says where each blur's textures came from while a stand-in
  # opens, shuts and opens one, grows it and takes it back to its size, and
  # walks it through more sizes than the pool's 64 MB keeps, going back to
  # one it holds after three newer sizes. That bounds the cap between about
  # 23 and 96 MB, not at 64 (tests/keeps-a-blur-ready.py).
  niri-keeps-a-blur-ready =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-keeps-a-blur-ready"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.python3
          pkgs.sway-unwrapped
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
        # niri's own filter, which lets its line on a blur's textures through.
        RUST_LOG = "niri=debug";
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/keeps-a-blur-ready.py logs/niri.log
        touch $out
      '';

  # A sandbox's window, one opened through pi-fence-display's
  # wp-security-context-v1 socket as a fenced command's would be, opens on
  # a workspace no screen shows and never takes the focus, whether it opens,
  # uses a token from Joel's window, or opens while he's on the empty
  # workspace at the end (sandboxed-windows-stay-behind.patch), while Joel
  # still goes to it by its id, as his launcher does. The account's niri,
  # nested in the sandbox, gets a stand-in window of Joel's and the
  # sandbox's (tests/opens-sandboxed-windows-behind.py).
  niri-opens-sandboxed-windows-behind =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      display = pkgs.callPackage ../pi/fence-display/package.nix { };
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-opens-sandboxed-windows-behind"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.python3
          pkgs.sway-unwrapped
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/opens-sandboxed-windows-behind.py ${lib.getExe display}
        touch $out
      '';

  # A copy a password manager marks secret, as wl-copy --sensitive does,
  # reaches no sandbox's window through wl_data_device or the primary
  # selection, whether it's copied while the window has the focus or the
  # window takes the focus while it's held
  # (keeps-secret-copies-from-sandboxes.patch); ordinary text still reaches
  # it, and Joel's own window is offered the secret and reads it, as before.
  # The account's niri, nested in the sandbox, gets a stand-in window of
  # Joel's and the sandbox's (tests/keeps-secret-copies-from-sandboxes.py).
  niri-keeps-secret-copies-from-sandboxes =
    let
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) (account graphical).xdg.configFile;
      display = pkgs.callPackage ../pi/fence-display/package.nix { };
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-keeps-secret-copies-from-sandboxes"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          pkgs.dbus
          pkgs.python3
          pkgs.sway-unwrapped
          pkgs.wl-clipboard
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          python3 ${./tests}/keeps-secret-copies-from-sandboxes.py ${lib.getExe display}
        touch $out
      '';

  # Xwayland without a cookie lets any account on the machine connect, read
  # the keys typed into X11 windows and type into them. The session's
  # xwayland-satellite, started on demand by the account's niri nested in
  # the sandbox, lets in a client holding the cookie where the session's
  # XAUTHORITY says, and refuses one without it
  # (tests/x11-needs-its-cookie.sh).
  niri-lets-x11-in-only-with-its-cookie =
    let
      home = account graphical;
      files = lib.filterAttrs (name: _: lib.hasPrefix "niri/" name) home.xdg.configFile;
      # As the user manager expands it.
      auth =
        lib.replaceStrings [ "\${XDG_RUNTIME_DIR}" ] [ "$XDG_RUNTIME_DIR" ]
          home.systemd.user.sessionVariables.XAUTHORITY;
      inherit (pkgs) mesa;
    in
    pkgs.runCommandLocal "niri-lets-x11-in-only-with-its-cookie"
      {
        nativeBuildInputs = [
          graphical.config.programs.niri.package
          (pkgs.callPackage ./xwayland-satellite.nix { })
          pkgs.dbus
          pkgs.sway-unwrapped
          pkgs.xdpyinfo
          pkgs.xwayland
        ];
        __EGL_VENDOR_LIBRARY_FILENAMES = "${mesa}/share/glvnd/egl_vendor.d/50_mesa.json";
        LIBGL_DRIVERS_PATH = "${mesa}/lib/dri";
        GBM_BACKENDS_PATH = "${mesa}/lib/gbm";
        LIBGL_ALWAYS_SOFTWARE = 1;
      }
      # Kept in Nix because each line sets a folder, a variable or a file
      # from what Nix says the session holds, then runs the scripts Nix
      # gives it, as niri-refreshes-once-a-frame does.
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_RUNTIME_DIR=$PWD/runtime
        mkdir -m 700 runtime
        export XAUTHORITY=${auth}
        # The sockets' folder as the system makes it, open to all and sticky,
        # before the nested sway makes it with the build's umask, which niri
        # refuses.
        mkdir -p /tmp/.X11-unix
        chmod 1777 /tmp/.X11-unix
        ${lib.concatLines (
          lib.mapAttrsToList (name: file: "install -D -m 644 ${file.source} ${lib.escapeShellArg name}") files
        )}
        ${lib.getExe pkgs.bash} ${../../tests/nested-niri.sh} niri/config.kdl logs \
          ${lib.getExe pkgs.bash} ${./tests/x11-needs-its-cookie.sh} logs/niri.log
        touch $out
      '';

  # niri's own screens are drawn in colours no theme reaches, so no key
  # opens one: not its screenshot screen, not its key list, and no quit that
  # asks first. The launcher's Log Out quits without asking too.
  niri-never-shows-its-own-screens =
    let
      actions = map (bind: bind.action) (account graphical).jitsusama.niri.binds;
      opens =
        action:
        lib.hasPrefix "screenshot;" action
        || lib.hasPrefix "screenshot " action
        || lib.hasPrefix "show-hotkey-overlay" action
        || (lib.hasPrefix "quit" action && !(lib.hasInfix "skip-confirmation=true" action));
      session = builtins.readFile ../quickshell/launcher/Session.qml;
    in
    holds "niri-never-shows-its-own-screens"
      "A key opens one of niri's own screens, or Log Out asks niri's question: ${lib.concatStringsSep ", " (lib.filter opens actions)}"
      (!(lib.any opens actions) && lib.hasInfix "Quit: { skip_confirmation: true }" session);

  # niri names a cursor theme, and a theme without a shape a program asks
  # for leaves niri's built-in arrow in its place without a word, so the
  # check looks for the shapes programs ask for most in the theme it names.
  niri-cursor-has-its-shapes =
    let
      home = account graphical;
      cursor = home.home.pointerCursor;
    in
    pkgs.runCommandLocal "niri-cursor-has-its-shapes" { } ''
      grep -q 'xcursor-theme "${cursor.name}"' ${home.xdg.configFile."niri/theme.kdl".source}
      for shape in default text pointer grab grabbing crosshair not-allowed help \
        ew-resize ns-resize col-resize n-resize; do
        if [ ! -e ${cursor.package}/share/icons/${cursor.name}/cursors/$shape ]; then
          echo "${cursor.name} has no $shape cursor"
          exit 1
        fi
      done
      touch $out
    '';
}
