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
  shellFolderLint,
  ...
}:
# Passwords serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # The passwords daemon's tests run as it builds, with stand-in password
  # managers in the shell, each asked over the daemon's own standard input
  # and output, as the shell asks it: a list holds every entry and no
  # secret, even one a manager gives with its secrets on; a held list or
  # get asks no manager anything; a secret goes only to the ask for it,
  # never to a watch, or to the clipboard marked sensitive, cleared when
  # its time is up, and copying is refused while no copier is named;
  # a one-time code says its period, and its detail gives the code, never
  # its seed; a manager signed out is said so and
  # gives no secret; a watch hears entries arrive; an ask or kind it
  # doesn't know is refused; and an audit, asked by Joel alone and refused
  # otherwise or by a manager signed out, counts the entries sharing a
  # password and matches each hash's suffix against a stand-in Have I Been
  # Pwned's range, padding counting nothing, sending it five characters
  # alone and saying no secret in an answer or a log; what it found is kept
  # in the state folder for Joel alone, holding no secret nor any hash,
  # outlives the daemon, and holds since when trouble was found until a
  # check finds none, each told to a watch. The daemon can't be read
  # through /proc by the account's other processes, and ends as soon as
  # whatever ran it closes its end. A secret ask is drawn only for a
  # reference listed to release, none listed drawing none, and only Joel's
  # yes on its own key, for what was drawn and once it was shown long
  # enough, releases the value, into the asked command alone in a stand-in
  # bubblewrap, masked in what it prints. The command's tests run too: no
  # command reaches an entry or a secret, and each mistyped one says what
  # it needs. Its rows are said in the account's own vocabulary, every
  # word of them its, as the shell says them.
  passwords-holds-no-secret = pkgs.callPackage ./package.nix {
    words =
      pkgs.writeText "words.json"
        (account graphical).xdg.configFile."quickshell/launcher/words.json".text;
  };
  # passwords keys lists what an SSH agent holds where 1Password's
  # listens, ~/.1password/agent.sock: a row a key, its type, size,
  # fingerprint, comment and public half, the same as --json, nothing for
  # an agent holding nothing, and how to turn it on when none listens. Its
  # agent is a stand-in ssh-agent the tests start in a home of their own,
  # holding a key made for them, so 1Password's is never reached
  # (tests/keys.rs).
  passwords-lists-1passwords-ssh-keys = (pkgs.callPackage ./package.nix { }).overrideAttrs (old: {
    pname = "passwords-lists-1passwords-ssh-keys";
    doCheck = true;
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
  # passwords and the completion home.nix installs: its commands and
  # --json, and none of the commands that reached an entry before the
  # daemon answered the launcher alone (tests/passwords-completion.zsh).
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
        export HOME=$PWD
        zsh -f ${./tests/passwords-completion.zsh} \
          ${pkgs.callPackage ./completion.nix { }}/share/zsh/site-functions/_passwords
        touch $out
      '';
  # clippy finds nothing in the passwords daemon, its tests included.
  passwords-passes-clippy = passesClippy "passwords-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # On a machine with 1Password, the launcher's Passwords provider runs the
  # daemon itself, with 1Password's manager, and nothing else can: there's
  # no unit and no socket for it, and no copier is named, so nothing is
  # copied until agents are fenced off the clipboard. The place stays off
  # meanwhile, and the provider knows 1Password's program.
  passwords-runs-each-password-manager =
    let
      home = account graphical;
      provider = home.jitsusama.quickshell.launcher.providers.passwords or { };
      daemon = lib.concatStringsSep " " (provider.daemon or [ ]);
    in
    holds "passwords-runs-each-password-manager"
      "The passwords daemon doesn't run 1Password's manager as it should."
      (
        lib.hasInfix "/bin/passwords daemon /nix/store/" daemon
        && !(home.systemd.user.services ? passwords)
        && !(home.systemd.user.sockets ? passwords)
        && !(provider ? copier)
        && (provider.off or false)
        && home.jitsusama.passwords.managers ? "1password"
        && lib.any (lib.hasSuffix "/op") home.jitsusama.passwords.managers."1password".command
        && map (manager: manager.program) (provider.managers or [ ]) == [ "com.onepassword.OnePassword" ]
      );
  # Nothing of the Passwords place, nor the daemon interface it asks its
  # daemon through, answers the shell's IPC, so no `qs ipc call` another
  # process makes reaches a secret or lets a manager in, now or once
  # something is added: only Joel's keys in the launcher do.
  the-passwords-answer-no-ipc =
    let
      read = map builtins.readFile [
        ./Passwords.qml
        ./Copied.qml
        ./Clearing.qml
        ../quickshell/Daemon.qml
      ];
    in
    holds "the-passwords-answer-no-ipc"
      "The Passwords place or its daemon's interface answers the shell's IPC."
      (!lib.any (text: lib.hasInfix "IpcHandler" text || lib.hasInfix "Ipc." text) read);
  # Each kind's mark, who's listed with nothing typed and in what order,
  # what a row says, an entry's acts and their order, opening it where its
  # manager fills only on a web page, its link in its manager's program, its drawer's eyebrow, and its facts saying every
  # figure and never a secret, its health and a weak strength warning,
  # its password reused and breached once every password is checked and
  # nothing before, those needing attention and the foot counting a copy
  # down are Passwords.js's, a manager's entries by vault and what its
  # drawer says it holds and what the place's one row opens while it's off
  # Managers.js's, and For You's one to-do for what
  # the last check found, whether it stands or is done, its side and its
  # weight, Checked.js's, run in Qt's own engine (tests/passwords.qml),
  # beside the launcher's own wording of how long ago (Ago.js) and of a
  # figure (Numbers.js).
  the-passwords-show-every-figure-but-the-secret =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-passwords-show-every-figure-but-the-secret" { } ''
      cp ${../../tests/passwords.qml} passwords.qml
      cp ${./Passwords.js} Passwords.js
      cp ${./Managers.js} Managers.js
      cp ${./Checked.js} Checked.js
      cp ${../quickshell/launcher/Ago.js} Ago.js
      cp ${../quickshell/launcher/Numbers.js} Numbers.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml passwords.qml
      touch $out
    '';
}
