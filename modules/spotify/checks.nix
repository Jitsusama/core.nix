# The checks of spotify, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
onLinux {
  # spotify-recent's tests run as it builds: what played lately is one row
  # a playlist, album or show, each with its size, repeats dropped, a track
  # from nowhere its album, Liked Songs and artists left inside, what
  # plays now first, a show by its episode, nine at most, no tab in a
  # name; each list Joel keeps is read a page at a time; a sign-in counts
  # as run out a minute early, a refresh without a new refresh token keeps
  # the old one, and the sign-in is written back 0600 in its own shape;
  # and against a stand-in for Spotify, a spent sign-in is refreshed and
  # kept before anything is asked, one another refreshed while it waited
  # on the lock is taken as it is, a play finds a device when none plays,
  # and signed out, nowhere to play and out of reach are each said by
  # their exit status.
  spotify-recent-reads-what-played = pkgs.callPackage ./spotify-recent/package.nix { };
  # clippy finds nothing in it, its tests included.
  spotify-recent-passes-clippy = passesClippy "spotify-recent-passes-clippy" (
    pkgs.callPackage ./spotify-recent/package.nix { }
  );

  # Spotify's program in the launcher reads Spotify only as it's stepped
  # into, once an opening, and holds its Recent, newest first, each row
  # timed and noting its kind and size, and Liked Songs, Playlists,
  # Albums, Artists and Podcasts, each counted, each played, opened and
  # copied as it says, its note saying when it's signed out or out of
  # reach (tests/spotify-program.qml), with spotify-recent, niri and
  # what's playing stood in.
  the-launcher-plays-from-spotify =
    let
      qs = lib.getExe' graphical.pkgs.quickshell "qs";
      stand = name: body: pkgs.writeText name body;
      niri = stand "Niri.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          property var spawned: []
          function spawn(command) { spawned = command; }
        }
      '';
      # Spotify's client running, as its player on the bus says, typed so
      # qmllint knows what the cases set on it.
      playing = stand "Playing.qml" ''
        pragma Singleton
        import Quickshell
        import QtQuick
        Singleton {
          component Player: QtObject {
            property string dbusName
            property bool isPlaying
          }
          property Player player: Player {
            dbusName: "org.mpris.MediaPlayer2.spotify"
            isPlaying: true
          }
          property var players: [player]
        }
      '';
      standIn = pkgs.writeShellScript "spotify-recent" ''
        exec ${lib.getExe pkgs.bash} ${../../tests/spotify-recent-stand-in.sh} "$@"
      '';
    in
    pkgs.runCommandLocal "the-launcher-plays-from-spotify" { } ''
      mkdir launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      cp ${../quickshell/Fuzzy.js} Fuzzy.js
      cp ${../quickshell/ProgramGlyphs.js} ProgramGlyphs.js
      cp ${../quickshell/Words.qml} Words.qml
      cp ${../quickshell/launcher/words.json} launcher/words.json
      cp ${../quickshell/launcher/Provider.qml} launcher/Provider.qml
      cp ${../quickshell/launcher/Asking.js} launcher/Asking.js
      cp ${../quickshell/launcher/Numbers.js} launcher/Numbers.js
      cp ${../quickshell/launcher/Ago.js} launcher/Ago.js
      cp ${./SpotifyProgram.qml} launcher/SpotifyProgram.qml
      cp ${./Spotify.js} launcher/Spotify.js
      cp ${playing} launcher/Playing.qml
      cp ${niri} Niri.qml
      cp ${../../tests/spotify-program.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export STAND_IN=${standIn}
      mkdir -m 700 $XDG_RUNTIME_DIR
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # pause-for-calls' tests run as it builds, what counts as a call being
  # modules/calls' and tested there: a call pauses Spotify after
  # two seconds of recording, once, and plays it five seconds after the
  # recording stops, a moment of recording pausing nothing and a reconnect
  # keeping the call; and each Spotify is found among the bus's names.
  pause-for-calls-follows-the-calls = pkgs.callPackage ./pause-for-calls/package.nix { };
  # clippy finds nothing in it, its tests included.
  pause-for-calls-passes-clippy = passesClippy "pause-for-calls-passes-clippy" (
    pkgs.callPackage ./pause-for-calls/package.nix { }
  );

  # Spotify's client is built in the theme: Spicetify patches the client
  # it's given, and one it can't patch, such as a newer Spotify than it
  # knows, would draw in Spotify's own colours. So the check looks for the
  # theme's background and faces in the client the account gets, and for
  # the page loading them.
  spotify-draws-in-the-theme =
    let
      home = account graphical;
      inherit (home.jitsusama.theme) colours font;
      client = "${home.programs.spicetify.spicedSpotify}/share/spotify/Apps/xpui";
    in
    pkgs.runCommandLocal "spotify-draws-in-the-theme" { } ''
      grep -q -F -- '--spice-main: ${lib.toLower colours.background};' ${client}/colors.css
      grep -q -F -- '--encore-body-font-stack: "${font.sans.family}"' ${client}/user.css
      grep -q "href='colors.css'" ${client}/index.html
      grep -q "href='user.css'" ${client}/index.html
      touch $out
    '';

  # Spotify's page answers the key language: the extension the account's
  # client carries and loads, with the language written in, scrolls a
  # stand-in for Spotify's page as Vim scrolls one, steps through its
  # history and opens its filter, and leaves Escape to Spotify
  # (tests/spotify-speaks-vims-keys.mjs).
  spotify-speaks-vims-keys =
    let
      home = account graphical;
      client = "${home.programs.spicetify.spicedSpotify}/share/spotify/Apps/xpui";
    in
    pkgs.runCommandLocal "spotify-speaks-vims-keys" { } ''
      if [ ! -e ${client}/extensions/speaks-vims-keys.js ]; then
        echo "Spotify's client carries no extension giving it Vim's keys"
        exit 1
      fi
      ${lib.getExe pkgs.nodejs} --no-warnings ${./tests/spotify-speaks-vims-keys.mjs} \
        ${client}/extensions/speaks-vims-keys.js
      grep -q -F "extensions/speaks-vims-keys.js" ${client}/index.html
      touch $out
    '';

  # spotify-player reads its settings before anything else and stops at a
  # value it can't use; it ignores a setting it doesn't know.
  spotify-player-accepts-its-configuration =
    let
      home = account graphical;
      files = home.xdg.configFile;
    in
    pkgs.runCommandLocal "spotify-player-accepts-its-configuration"
      { nativeBuildInputs = [ home.programs.spotify-player.package ]; }
      ''
        export HOME=$PWD
        mkdir config cache
        cp ${files."spotify-player/app.toml".source} config/app.toml
        spotify_player -c config -C cache features
        touch $out
      '';

  # spotify-player takes the language's keys its own lack: Ctrl+e and Ctrl+y
  # a line, Ctrl+o back a page. It reads its keymap before anything else and
  # stops at one it can't, so the keys are read from what it accepted.
  spotify-player-speaks-vims-keys =
    let
      home = account graphical;
      files = home.xdg.configFile;
      keymap = files."spotify-player/keymap.toml".source or (pkgs.writeText "keymap.toml" "");
    in
    pkgs.runCommandLocal "spotify-player-speaks-vims-keys"
      {
        nativeBuildInputs = [
          home.programs.spotify-player.package
          pkgs.yq-go
        ];
      }
      ''
        export HOME=$PWD
        mkdir config cache
        cp ${files."spotify-player/app.toml".source} config/app.toml
        cp ${keymap} config/keymap.toml
        spotify_player -c config -C cache features > /dev/null
        for pair in C-e:SelectNextOrScrollDown C-y:SelectPreviousOrScrollUp C-o:PreviousPage; do
          key=''${pair%%:*} command=''${pair#*:}
          given=$(yq -p toml ".keymaps[] | select(.key_sequence == \"$key\") | .command" config/keymap.toml)
          if [ "$given" != "$command" ]; then
            echo "spotify-player's $key doesn't do $command, as the language says: '$given'" >&2
            exit 1
          fi
        done
        touch $out
      '';

  # spotify-player's sign-in asks ncspot's app to send Joel back to the one
  # address that app takes. The sign-in prints where it sends him, then
  # waits for Spotify's answer, which never comes here, so it's stopped.
  spotify-player-signs-in-where-spotify-answers =
    let
      home = account graphical;
      files = home.xdg.configFile;
    in
    pkgs.runCommandLocal "spotify-player-signs-in-where-spotify-answers"
      { nativeBuildInputs = [ home.programs.spotify-player.package ]; }
      ''
        export HOME=$PWD BROWSER=true
        mkdir config cache
        cp ${files."spotify-player/app.toml".source} config/app.toml
        said=$(timeout 10 spotify_player -c config -C cache authenticate 2>&1 < /dev/null || true)
        # The address's query is in no set order.
        asked=$(grep -F 'client_id=d420a117a32841c2b3474932e49fb54b' <<< "$said" || true)
        if ! grep -q -E '[?&]redirect_uri=http%3A%2F%2F127\.0\.0\.1%3A8989%2Fncspot_login(&|$)' <<< "$asked"; then
          echo "spotify-player's sign-in asks ncspot's app for an address it doesn't take: $said" >&2
          exit 1
        fi
        touch $out
      '';

  # `spotify_player authenticate` signs the streaming client in with the
  # token Spotify gives it, which is what saves the credentials it plays
  # with. Spotify doesn't answer here, so the check reads that the step is
  # in the program, by what it says when the step fails.
  spotify-player-keeps-its-streaming-sign-in =
    let
      package = (account graphical).programs.spotify-player.package;
    in
    pkgs.runCommandLocal "spotify-player-keeps-its-streaming-sign-in" { } ''
      if ! grep -q -a -F 'sign the streaming client in' ${lib.getExe package}; then
        echo "spotify-player's authenticate drops the streaming client's token rather than signing it in, so it never saves the credentials it plays with." >&2
        exit 1
      fi
      touch $out
    '';
}
