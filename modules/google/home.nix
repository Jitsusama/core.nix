# The Google account, for the desktop: google asks Google's own API for the
# meetings on Joel's calendar and the mail that wants him, and keeps them
# for the shell on a socket, $XDG_RUNTIME_DIR/google.sock, and in a file
# beside it, google.json, which the shell watches (src/main.rs says how).
#
# It asks as the account the harness signed in, from the harness's own
# file, which pi writes when Joel signs in with /google-auth and rewrites
# each time it refreshes a token. google only reads it. Until the file
# exists nothing runs, so a machine nobody signed in to Google on asks
# Google nothing; once it appears, google starts.
#
# It's background work, a small request a minute over the line, so it runs
# in background.slice, behind everything Joel uses.
#
# The shell hears it through Google.qml. Calendar.qml holds the meetings,
# which the launcher's Today row and the Calendar program's meetings
# (Meetings.qml) read, and Mail.qml the mail that wants Joel, which the
# Mail program's mail (Messages.qml) reads. Calendar and
# Mail are programs of their own, each its Google web app in a window of
# its own, opened as jitsusama.google.window says, since which browser
# makes one is the machine's to choose. A meeting's call is joined through
# google-opens (opens.sh), which opens a Meet link the same way, and a
# call open in a window of its own is on air while it is (Call.qml). A
# meeting coming within the hour wants Joel, and is said as a notice at
# each of its reminders and as it starts (Starting.qml); the account
# signed out wants him to sign in again (SignedOut.qml); and the mail that
# wants him is counted there too (NewMail.qml).
{ quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  google = pkgs.callPackage ./package.nix { };
  credentials = "${config.xdg.configHome}/agentic-harness/google/credentials.json";
  xdgOpen = lib.getExe' pkgs.xdg-utils "xdg-open";

  # The window command, each %u in it the link, as a desktop entry writes it.
  window = pkgs.writeShellScript "google-window" ''
    exec ${
      lib.concatMapStringsSep " " (
        word: lib.concatStringsSep ''"$1"'' (map lib.escapeShellArg (lib.splitString "%u" word))
      ) config.jitsusama.google.window
    }
  '';

  opens = pkgs.writeShellScriptBin "google-opens" ''
    exec ${lib.getExe pkgs.bash} ${./opens.sh} ${window} ${xdgOpen} "$@"
  '';

  # A Google web app as a program: its name, what it is, and its page.
  program = name: comment: url: {
    inherit name comment;
    exec = "${lib.getExe opens} ${url}";
    categories = [ "Network" ];
  };
in
{
  imports = [ quickshell ];

  options.jitsusama.google.window = lib.mkOption {
    type = lib.types.nonEmptyListOf lib.types.str;
    default = [
      xdgOpen
      "%u"
    ];
    defaultText = lib.literalExpression ''[ "xdg-open" "%u" ]'';
    example = [
      "google-chrome-stable"
      "--app=%u"
    ];
    description = ''
      The command that opens a Google page (Calendar, Mail, a Meet call) in
      a window of its own, each %u in it the link. By default it opens in
      the default browser, as a tab where that's what it does.
    '';
  };

  config = {
    home.packages = [ opens ];

    # Google's programs, every one opened as jitsusama.google.window says,
    # so they're one family wherever they run. Docs, Sheets, Slides, Drive
    # and Meet were a machine's own entries before core made them, so a
    # machine's entry of the same name still wins over these until it's
    # gone.
    xdg.desktopEntries = {
      google-calendar =
        program "Calendar" "Meetings in the Google account"
          "https://calendar.google.com/";
      google-mail = program "Mail" "Mail in the Google account" "https://mail.google.com/";
    }
    // lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
      google-docs = program "Docs" "Documents in the Google account" "https://docs.google.com/document/";
      google-sheets =
        program "Sheets" "Spreadsheets in the Google account"
          "https://docs.google.com/spreadsheets/";
      google-slides =
        program "Slides" "Presentations in the Google account"
          "https://docs.google.com/presentation/";
      google-drive = program "Drive" "Files in the Google account" "https://drive.google.com/";
      google-meet = program "Meet" "Video calls in the Google account" "https://meet.google.com/" // {
        # A new meeting, opened in Meet's window, where its link is copied.
        actions.new = {
          name = "New Meeting";
          exec = "${lib.getExe opens} https://meet.google.com/new";
        };
      };
    };

    xdg.configFile = {
      "quickshell/Google.qml".source = ./Google.qml;
      "quickshell/Google.js".source = ./Google.js;
      "quickshell/Calendar.qml".source = ./Calendar.qml;
      "quickshell/Calendar.js".source = ./Calendar.js;
      "quickshell/Mail.qml".source = ./Mail.qml;
      "quickshell/Mail.js".source = ./Mail.js;
      "quickshell/Meet.js".source = ./Meet.js;
      "quickshell/Drive.qml".source = ./Drive.qml;
      "quickshell/Drive.js".source = ./Drive.js;
    };

    # The account signed out, which wants Joel to sign in again, wearing
    # the signing-in mark; first among what of Google's wants him.
    jitsusama.quickshell.overview.pieces.google = {
      order = 55;
      source = ./SignedOut.qml;
      label = "google";
      tier = "when";
      mark = "signIn";
    };

    # A meeting coming, which wants Joel, wearing the calendar's own mark;
    # among what wants him, after the notices.
    jitsusama.quickshell.overview.pieces.meeting = {
      order = 56;
      source = ./Starting.qml;
      label = "meeting";
      tier = "when";
      mark = "meeting";
    };

    # The mail that wants Joel, wearing the mail mark; after what wants
    # him sooner, since it keeps.
    jitsusama.quickshell.overview.pieces.mail = {
      order = 61;
      source = ./NewMail.qml;
      label = "mail";
      tier = "when";
      mark = "mail";
    };

    # A Meet call, wearing the call mark Tuple's does, since it's a call
    # too; after Tuple's and before the sharing a call often brings.
    jitsusama.quickshell.overview.pieces.meet = {
      order = 79;
      source = ./Call.qml;
      label = "meet";
      tier = "when";
      mark = "call";
    };

    # The meetings, as parts of the Calendar program.
    jitsusama.quickshell.launcher.providers.calendar = {
      order = 47;
      source = ./Meetings.qml;
    };
    # The mail that wants Joel, as parts of the Mail program.
    jitsusama.quickshell.launcher.providers.mail = {
      order = 48;
      source = ./Messages.qml;
    };
    # The documents Joel looked at lately, as each program's Recent, and
    # found by title.
    jitsusama.quickshell.launcher.providers.documents = {
      order = 49;
      source = ./Documents.qml;
    };
    # Meet's own acts beside its New Meeting: Join by Code.
    jitsusama.quickshell.launcher.providers.meet = {
      order = 50;
      source = ./Meet.qml;
    };
    # Each wears a mark of its own in Programs rather than the program
    # mark every window without a logo shares: Calendar the meeting's,
    # which it holds, and Mail Gmail's own logo.
    jitsusama.quickshell.launcher.providers.applications.glyphs = {
      google-calendar = "f0ed7";
      google-mail = "f02ab";
    };

    systemd.user.paths.google = {
      Unit.Description = "Starts the Google account's watch once pi has signed in";
      Path.PathExists = credentials;
      Install.WantedBy = [ "default.target" ];
    };

    systemd.user.services.google = {
      Unit = {
        Description = "Keeps the Google account's meetings and mail for the shell";
        # Stopped before a switch reloads the units, google.path starts it
        # again at once, from the old unit, since the credentials are still
        # there, and the switch's start then finds it running; restarted
        # after the reload, it comes back as the new one.
        X-SwitchMethod = "restart";
      };
      Service = {
        Slice = "background.slice";
        ExecStart = "${lib.getExe google} %t/google.sock ${lib.escapeShellArg credentials}";
        Restart = "on-failure";
        RestartSec = 10;
        # What it held goes with it, so the shell shows no account rather than
        # what was true when it stopped.
        # The files fetched to open go with the program, as what it held does.
        ExecStopPost = lib.concatStringsSep " " [
          "${pkgs.coreutils}/bin/rm -rf"
          "%t/google.json"
          "%t/google.json.new"
          "%t/google-files"
        ];
      };
    };
  };
}
