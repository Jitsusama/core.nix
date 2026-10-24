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

  # niri reads back the files an account gets and rejects anything it
  # wouldn't load, from a misspelt action to a colour it can't parse.
  niri-accepts-its-configuration =
    let
      files = (account graphical).xdg.configFile;
    in
    pkgs.runCommandLocal "niri-accepts-its-configuration" { nativeBuildInputs = [ pkgs.niri ]; } ''
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
