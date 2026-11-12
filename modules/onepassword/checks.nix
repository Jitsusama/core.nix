# The checks of onepassword, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # A switch keeps the running 1Password, which restarting would lock.
  onepassword-outlasts-a-switch =
    holds "onepassword-outlasts-a-switch" "A switch restarts 1Password, locking it under Joel."
      (((account graphical).systemd.user.services.onepassword.Unit.X-SwitchMethod or null) == "keep-old");

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

  # 1Password opened from its entry (in-its-unit.sh) hands its window to
  # the unit's app once while the unit stays up, and once more when the
  # unit restarted under it, as when the app died making the window.
  onepassword-opens-its-window-when-its-unit-starts-again =
    let
      # systemctl saying the unit's invocation as a file holds it, and
      # 1Password counting its runs and, when told to, dying as its unit
      # starts again.
      systemctl = pkgs.writeShellScriptBin "systemctl" ''
        case "$*" in *show*) cat "$STATE/invocation" ;; esac
      '';
      app = pkgs.writeShellScript "1password" ''
        echo "$*" >> "$STATE/runs"
        if [ -e "$STATE/dies" ]; then
          rm "$STATE/dies"
          echo second > "$STATE/invocation"
          exit 133
        fi
      '';
    in
    pkgs.runCommandLocal "onepassword-opens-its-window-when-its-unit-starts-again"
      { nativeBuildInputs = [ systemctl ]; }
      # Kept in Nix because each line runs the script Nix gives it with the
      # stand-ins Nix makes above, or reads what they wrote.
      ''
        fail() { echo "$1"; exit 1; }
        { echo 'app=${app}'; cat ${./in-its-unit.sh}; } > in-its-unit
        export STATE=$PWD
        echo first > invocation
        bash -euo pipefail in-its-unit --x || fail "an opening with its unit up failed"
        [ "$(cat runs)" = --x ] || fail "an opening with its unit up ran: $(cat runs)"
        rm runs
        touch dies
        bash -euo pipefail in-its-unit --x || fail "an opening its unit restarted under failed"
        [ "$(wc -l < runs)" = 2 ] || fail "a restarted opening ran $(wc -l < runs) times"
        touch $out
      '';

  # The account's zsh is given op's own completion, the very file the op
  # the system wraps was built with, from the account's own profile, which
  # home-manager puts on zsh's fpath.
  onepassword-brings-ops-zsh-completion =
    let
      home = account graphical;
      op = graphical.config.programs._1password.package;
      given = "${home.home.path}/share/zsh/site-functions/_op";
    in
    pkgs.runCommandLocal "onepassword-brings-ops-zsh-completion" { } ''
      cmp ${given} ${op}/share/zsh/site-functions/_op \
        || { echo "The account's zsh isn't given op's own completion."; exit 1; }
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

  # 1Password's manager for Passwords tests itself as it builds, against
  # op's own JSON: an entry carries no secret op gives, a password's
  # history included, a secret is read only for the detail asked,
  # 1Password's words become Passwords' own, and, run against a stand-in
  # op, it says the setting to turn on while 1Password keeps op out.
  onepassword-manager-passes-no-secret-on = pkgs.callPackage ./manager/package.nix { };
  # clippy finds nothing in 1Password's manager, its tests included.
  onepassword-manager-passes-clippy = passesClippy "onepassword-manager-passes-clippy" (
    pkgs.callPackage ./manager/package.nix { }
  );
}
