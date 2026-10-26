# The checks of spotify, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # pause-for-calls' tests run as it builds: a call app's running
  # recording is seen and goes when it idles or its node does, while
  # PipeWire's own filters, dictation's recorder, the camera's videos, a
  # sink's monitor and playback are left out; a call pauses Spotify after
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
      inherit (home.jitsusama.theme) roles font;
      client = "${home.programs.spicetify.spicedSpotify}/share/spotify/Apps/xpui";
    in
    pkgs.runCommandLocal "spotify-draws-in-the-theme" { } ''
      grep -q -F -- '--spice-main: ${lib.toLower roles.background};' ${client}/colors.css
      grep -q -F -- '--encore-body-font-stack: "${font.sans.family}"' ${client}/user.css
      grep -q "href='colors.css'" ${client}/index.html
      grep -q "href='user.css'" ${client}/index.html
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
}
