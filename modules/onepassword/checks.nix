# The checks of onepassword, which checks.nix gathers with every other folder's;
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
  # The account's 1Password entry offers Quick Access and then Lock
  # 1Password, each through a switch the installed 1Password still takes,
  # so a release that drops one fails here rather than doing nothing.
  onepassword-offers-its-quick-acts =
    let
      home = account graphical;
      app = graphical.config.programs._1password-gui.package;
      entry = "${home.home.path}/share/applications/com.onepassword.OnePassword.desktop";
    in
    pkgs.runCommandLocal "onepassword-offers-its-quick-acts" { } ''
      fail() { echo "$1"; exit 1; }
      grep -q -x 'Actions=access;lock' ${entry} || fail "1Password's actions: $(grep '^Actions=' ${entry})"
      for named in 'Name=Quick Access' 'Exec=1password --quick-access' 'Name=Lock 1Password' \
        'Exec=1password --lock' 'StartupWMClass=com.onepassword.OnePassword'; do
        grep -q -x "$named" ${entry} || fail "1Password's entry lacks $named"
      done
      for switch in --quick-access --lock; do
        grep -a -q -- "\"$switch\"" ${app}/share/1password/resources/app.asar \
          || fail "1Password no longer takes $switch"
      done
      touch $out
    '';

  # The session starts the system's own 1Password without its window, so
  # its command-line tool and Chrome's extension find it after a boot,
  # through a switch the installed 1Password still takes.
  onepassword-starts-with-the-session =
    let
      service = (account graphical).systemd.user.services.onepassword or { };
      package = graphical.config.programs._1password-gui.package;
      started =
        lib.toList (service.Service.ExecStart or [ ]) == [ "${lib.getExe' package "1password"} --silent" ]
        && service.Install.WantedBy or [ ] == [ "graphical-session.target" ];
      unstarted = ''fail "The session doesn't start the system's 1Password with --silent."'';
    in
    pkgs.runCommandLocal "onepassword-starts-with-the-session" { } ''
      fail() { echo "$1"; exit 1; }
      ${lib.optionalString (!started) unstarted}
      grep -a -q -- '"--silent"' ${package}/share/1password/resources/app.asar \
        || fail "1Password no longer takes --silent"
      touch $out
    '';
}
// {
  # 1Password's Chrome extension talks to the desktop app through a helper in
  # the onepassword group, and every account may unlock the app with its own
  # password.
  onepassword-works-with-chrome =
    holds "onepassword-works-with-chrome" "1Password lacks its browser helper or its polkit owners."
      (
        graphical.config.security.wrappers."1Password-BrowserSupport".group or null == "onepassword"
        && graphical.config.programs._1password-gui.polkitPolicyOwners == [ "someone" ]
      );

  # The app answers its command-line tool only when the tool runs setgid in
  # the onepassword-cli group.
  onepassword-answers-op =
    let
      op = graphical.config.security.wrappers.op or { };
    in
    holds "onepassword-answers-op"
      "1Password's command-line tool isn't setgid in the onepassword-cli group, so the app won't answer it."
      (op.group or null == "onepassword-cli" && op.setgid or false && !(op.setuid or false));
}
