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

  # niri reads its settings again in place when a switch changes them, and
  # refuses all of them for one it doesn't know. The niri running through a
  # switch is the one the machine booted, from before the switch, so a
  # patch adding a setting leaves it refusing every change until the next
  # boot, as hold-first-frame.patch once did. A patch asks through niri's
  # environment or its command line instead. srgb-gamut.patch's setting is
  # in every niri booted since it came in.
  niri-patches-add-no-settings =
    let
      kept = [ "srgb-gamut.patch" ];
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
