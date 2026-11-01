# greetd starts niri at boot. Unlocking the disk has just proved it's Joel, so
# the machine's own account signs in by itself, once; after logging out, the
# greeter asks who's there.
#
# The greeter (greeter.qml) is Quickshell in a niri of its own, run as
# greetd's greeter account, wearing the look of the account that signs in:
# its Quickshell files and niri's look and keyboard come from that account's
# home configuration, so a theme reaches the greeter as it reaches the
# desktop. That niri has no keys bound, so the greeter's field is all there
# is to type into. Should the greeter stop without having signed anyone in,
# tuigreet asks on the console instead, so a broken greeter never locks Joel
# out. A machine with no such account, or no home configuration for it, gets
# tuigreet alone.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  # Prints the name of each variable in the environment that systemd takes as
  # valid, which is every name of ASCII letters, digits and underscores not
  # starting with a digit. gawk lists AWKPATH and AWKLIBPATH whether they're
  # set or not, so they're left out.
  environmentNames = pkgs.writeShellScript "environment-names" ''
    exec ${lib.getExe pkgs.gawk} 'BEGIN {
      for (v in ENVIRON)
        if (v ~ /^[A-Za-z_][A-Za-z0-9_]*$/ && v != "AWKPATH" && v != "AWKLIBPATH")
          print v
    }'
  '';

  # niri's session script imports the login's environment with a bare
  # `systemctl --user import-environment`, which systemd deprecates with a
  # warning that lands on the console as the desktop starts. This passes the
  # same variables by name. A name systemd won't take fails the whole call,
  # hence the filter. Upstream, resources/niri-session's line would become
  # `systemctl --user import-environment $(awk 'BEGIN { for (v in ENVIRON) if
  # (v ~ /^[A-Za-z_][A-Za-z0-9_]*$/ && v != "AWKPATH" && v != "AWKLIBPATH")
  # print v }')`; take this out once niri's does.
  niriSession = pkgs.runCommand "niri-session" { } ''
    substitute ${config.programs.niri.package}/bin/niri-session $out \
      --replace-fail 'systemctl --user import-environment' \
        'systemctl --user import-environment $(${environmentNames})'
    chmod +x $out
  '';

  cfg = config.jitsusama.login;
  gpu = config.jitsusama.gpu.unit;
  niri = config.programs.niri.package;

  tuigreet = [
    (lib.getExe pkgs.tuigreet)
    "--time"
    "--remember"
    "--asterisks"
    "--cmd"
    (lib.escapeShellArgs cfg.session)
  ];

  # The account's home configuration, when the machine has one for it.
  inherit (import ./account.nix { inherit config lib options; }) home;
  files = home.home-files;

  # Whether the account's niri holds each screen's first frame for its
  # wallpaper (../niri/home.nix), which the greeter's niri is asked through
  # its environment as the session's is, so the greeter ends it.
  holds = lib.attrByPath [ "jitsusama" "niri" "holdFirstFrame" ] false home;
  # And whether it sends full-range RGB to a screen that takes it, which
  # the greeter's niri is asked the same way, so the greeter's blacks are
  # the desktop's.
  fullRange = lib.attrByPath [ "jitsusama" "niri" "fullRangeWhenSelectable" ] false home;

  # Every account that can sign in, the machine's own first, each called by
  # the first word of its description.
  accounts =
    let
      people = lib.filterAttrs (_: user: user.isNormalUser) config.users.users;
      called =
        name:
        let
          words = lib.splitString " " people.${name}.description;
        in
        if people.${name}.description == "" then name else builtins.head words;
      names = [ cfg.account ] ++ lib.filter (name: name != cfg.account) (lib.attrNames people);
    in
    map (name: {
      inherit name;
      called = called name;
    }) (lib.filter (name: people ? ${name}) names);

  # The account's Quickshell files with the greeter beside them.
  shell = pkgs.runCommand "greeter-shell" { } ''
    cp -rL --no-preserve=mode ${files}/.config/quickshell $out
    cp ${
      pkgs.replaceVars ./greeter.qml {
        accounts = builtins.toJSON accounts;
        session = builtins.toJSON cfg.session;
        analyze = lib.getExe' config.systemd.package "systemd-analyze";
      }
    } $out/greeter.qml
  '';

  # Starts the greeter inside its niri, and ends that niri once the greeter
  # has gone, whether it signed someone in or fell over. niri gives what it
  # starts nowhere to write, so the greeter writes to the journal.
  greet = pkgs.writeShellScript "greet" ''
    ${lib.getExe' config.systemd.package "systemd-cat"} --identifier=greeter \
      ${lib.getExe pkgs.quickshell} -p ${shell}/greeter.qml
    exec ${lib.getExe niri} msg action quit --skip-confirmation
  '';

  # The account's niri, bound to nothing, starting the greeter. The account's
  # binds would open a terminal as the greeter's account, so they're left out.
  # Its config already keeps niri's overlay of keys from showing at start.
  # Nothing here needs the pointer, which would sit over the time, so it
  # hides once it's still or the keys are typed on.
  compositor = pkgs.runCommand "greeter-niri" { nativeBuildInputs = [ niri ]; } ''
    mkdir $out
    cp -L ${files}/.config/niri/theme.kdl ${files}/.config/niri/keyboard.kdl $out/
    chmod u+w $out/theme.kdl
    substituteInPlace $out/theme.kdl --replace-fail 'cursor {' \
      "$(printf 'cursor {\n    hide-when-typing\n    hide-after-inactive-ms 1000')"
    echo 'binds {}' > $out/binds.kdl
    cat ${files}/.config/niri/config.kdl - > $out/config.kdl <<'EOF'

    // The greeter's own: it starts Quickshell's greeter and nothing else.
    spawn-at-startup "${greet}"
    EOF
    HOME=$TMPDIR niri validate -c $out/config.kdl
  '';

  # Runs the greeter, falling back to tuigreet (greeter.sh). It's a plain
  # script rather than an application, since stopping at the first failure
  # would skip the fall-back.
  greeter = pkgs.writeShellScript "greeter" (
    lib.concatLines [
      (lib.concatStringsSep " " [
        "mktemp=${lib.getExe' pkgs.coreutils "mktemp"}"
        "rm=${lib.getExe' pkgs.coreutils "rm"}"
        "systemd_cat=${lib.getExe' config.systemd.package "systemd-cat"}"
      ])
      "compositor=(${lib.escapeShellArgs cfg.compositor}) niri_config=${compositor}/config.kdl"
      (lib.optionalString holds "export NIRI_HOLD_FIRST_FRAME=1")
      (lib.optionalString fullRange "export NIRI_FULL_RANGE_WHEN_SELECTABLE=1")
      "tuigreet=(${lib.escapeShellArgs tuigreet})"
      (builtins.readFile ./greeter.sh)
    ]
  );

  # What asks who's there: the greeter, or tuigreet alone where there's no
  # account's look for the greeter to wear.
  asks = if home != null then "${greeter}" else lib.escapeShellArgs tuigreet;
in
{
  imports = [ ../gpu/nixos.nix ];

  options.jitsusama.login = {
    account = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "joel";
      description = ''
        The machine's own account, set per machine: the one the greeter
        asks first and dresses in, and the one that signs in by itself at
        boot when automatic is on.
      '';
    };

    automatic = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether the account signs in by itself at boot, without the greeter.";
    };

    # What the desktop test changes, since its VM can't run niri on a screen.
    session = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      internal = true;
      default = [ "${niriSession}" ];
      description = "The command that starts the session once someone's signed in.";
    };

    compositor = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      internal = true;
      default = [ (lib.getExe niri) ];
      description = "How the greeter's niri starts, before its config is named.";
    };
  };

  config = {
    services.greetd = {
      enable = true;
      settings = {
        default_session.command = asks;
      }
      // lib.optionalAttrs (cfg.account != null && cfg.automatic) {
        initial_session = {
          user = cfg.account;
          command = lib.escapeShellArgs cfg.session;
        };
      };
    };

    # nixpkgs starts greetd once every other boot job has finished, which held
    # the XPS 14's desktop back by up to 5.6 s, for jobs it doesn't need, such
    # as waiting for the network. What niri does need is the GPU, so where the
    # hardware names it, greetd waits for that instead. Wants rather than
    # Requires: a GPU that never comes up delays the login, not blocks it.
    systemd.services.greetd = lib.mkIf (gpu != null) {
      wants = [ gpu ];
      after = [ gpu ];
      serviceConfig.Type = lib.mkForce "simple";
    };
  };
}
