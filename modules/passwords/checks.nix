# The checks of passwords, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
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
# Passwords serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # The passwords daemon's tests run as it builds, with stand-in password
  # managers in the shell: a list holds every entry and no secret, even
  # one a manager gives with its secrets on; a held list or get asks no
  # manager anything; a secret goes only to the client that asked to see
  # it, or to the clipboard marked sensitive, cleared when its time is up;
  # a one-time code says its period; a manager signed out is said so and
  # gives no secret; a watch hears entries arrive; and an ask or kind it
  # doesn't know is refused. The command's tests run too: each answer set
  # in columns under a header for a person and tab-separated for a
  # program, its --json the daemon's answer with no secret but show's, a
  # title and a detail's label naming what they name, each exit status the
  # usage says, and what completion asks answered from what's held.
  passwords-holds-no-secret = pkgs.callPackage ./package.nix { };
  # passwords keys lists what an SSH agent holds where 1Password's
  # listens, ~/.1password/agent.sock: a row a key, its type, size,
  # fingerprint, comment and public half, the same as --json, nothing for
  # an agent holding nothing, and how to turn it on when none listens. Its
  # agent is a stand-in ssh-agent the tests start in a home of their own,
  # holding a key made for them, so 1Password's is never reached
  # (tests/keys.rs).
  passwords-lists-1passwords-ssh-keys = (pkgs.callPackage ./package.nix { }).overrideAttrs (old: {
    pname = "passwords-lists-1passwords-ssh-keys";
    nativeCheckInputs = (old.nativeCheckInputs or [ ]) ++ [ pkgs.openssh ];
    cargoTestFlags = [
      "--test"
      "keys"
    ];
    checkFlags = [ "--ignored" ];
    installPhase = ''
      touch $out
    '';
  });
  # passwords completes in zsh, typed in a real line editor with the real
  # passwords and the completion home.nix installs, against a daemon with
  # a stand-in manager: its commands, each entry by its title once a list
  # has read it, an entry's details by their labels once it's been read in
  # full, each manager, and --json; and no Tab asks the manager anything
  # (tests/passwords-completion.zsh).
  passwords-completes-in-zsh =
    let
      passwords = pkgs.callPackage ./package.nix { };
    in
    pkgs.runCommandLocal "passwords-completes-in-zsh"
      {
        nativeBuildInputs = [
          passwords
          pkgs.bash
          pkgs.zsh
        ];
      }
      ''
        export HOME=$PWD PASSWORDS_SOCKET=$PWD/run/passwords.sock ASKED=$PWD/asked.log
        mkdir -m 700 run
        cat > managers.json <<EOF
        {"managers": [{"name": "keeping",
          "command": ["${lib.getExe pkgs.bash}", "${./tests/keeping-manager.sh}", "$ASKED"]}]}
        EOF
        passwords daemon "$PASSWORDS_SOCKET" managers.json &
        daemon=$!
        for _ in $(seq 100); do
          [ -S "$PASSWORDS_SOCKET" ] && passwords health | grep -q answering && break
          sleep 0.1
        done
        status=0
        zsh -f ${./tests/passwords-completion.zsh} \
          ${pkgs.callPackage ./completion.nix { }}/share/zsh/site-functions/_passwords || status=$?
        kill $daemon
        [ $status = 0 ] || exit $status
        touch $out
      '';
  # clippy finds nothing in the passwords daemon, its tests included.
  passwords-passes-clippy = passesClippy "passwords-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # On a machine with 1Password, the daemon runs it as a password manager
  # on the daemons' socket, which systemd holds, with wl-copy to copy and
  # no core dump to write a secret in, and the launcher's Passwords
  # provider knows its program.
  passwords-runs-each-password-manager =
    let
      home = account graphical;
      service = home.systemd.user.services.passwords or { };
      started = lib.concatStringsSep " " (lib.toList (service.Service.ExecStart or ""));
      provider = home.jitsusama.quickshell.launcher.providers.passwords or { };
    in
    holds "passwords-runs-each-password-manager"
      "The passwords daemon doesn't run 1Password's manager as it should."
      (
        lib.hasInfix "%t/daemons/passwords.sock" started
        && (home.systemd.user.sockets.passwords.Socket.ListenStream or "") == "%t/daemons/passwords.sock"
        && (service.Service.LimitCORE or null) == 0
        && lib.elem "graphical-session.target" (service.Unit.PartOf or [ ])
        && lib.any (lib.hasInfix "PASSWORDS_COPIER=") (lib.toList (service.Service.Environment or [ ]))
        && home.jitsusama.passwords.managers ? "1password"
        && lib.hasSuffix "/op" (lib.last home.jitsusama.passwords.managers."1password".command)
        && map (manager: manager.program) (provider.managers or [ ]) == [ "com.onepassword.OnePassword" ]
      );
}
