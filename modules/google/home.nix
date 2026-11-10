# The Google account, for the desktop: google asks Google's own API for the
# meetings on Joel's calendar and the mail that wants him, and keeps them
# for the shell in its daemon interface, a record for each thing it holds,
# on $XDG_RUNTIME_DIR/daemons/google.sock, a socket systemd holds for it
# (src/daemon.rs), where call transcripts ask it for the meeting on too
# (src/main.rs says how).
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
# wants him is counted there too (NewMail.qml). What became of each save
# the launcher's picker staged for Drive is said as a notice (Saves.qml),
# a failed one with Save a Copy, so nothing saved is lost. Drive's program
# holds Drive's own places, its Recent, Starred, Shared with Me and Shared
# Drives (DriveProgram.qml).
{ people, quickshell }:
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

  # A program's one act of making something new, opened in its window as
  # its page is, so the launcher finds it by name from the top. The link is
  # quoted, since a desktop entry's command reserves ? and &.
  making = name: url: {
    actions.new = {
      inherit name;
      exec = ''${lib.getExe opens} "${url}"'';
    };
  };
in
{
  imports = [
    people
    quickshell
    ../keys/home.nix
  ];

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

    # Each program's own keys, for the launcher's Shortcuts over its page.
    jitsusama.keys = import ./keys.nix;

    # Everyone Joel's meetings and mail name, as people (GoogleSource.qml),
    # consulted after a source that calls, so calling comes before mail.
    jitsusama.people.sources.google = {
      order = 20;
      source = ./GoogleSource.qml;
      # Who they are, and when each thing with them was, said to the people
      # daemon too, so it lists and orders them with everyone else
      # (src/people.rs); google-people follows google's daemon socket in
      # the runtime folder.
      daemon = [
        (lib.getExe' google "google-people")
        "daemons/google.sock"
      ];
    };

    # Google's programs, every one opened as jitsusama.google.window says,
    # so they're one family wherever they run. Docs, Sheets, Slides, Drive
    # and Meet were a machine's own entries before core made them, so a
    # machine's entry of the same name still wins over these until it's
    # gone.
    xdg.desktopEntries = {
      google-calendar =
        program "Calendar" "Meetings in the Google account" "https://calendar.google.com/"
        // making "New Event" "https://calendar.google.com/calendar/r/eventedit";
      # Gmail's own page for a new message, alone in its window.
      google-mail =
        program "Mail" "Mail in the Google account" "https://mail.google.com/"
        // making "Write a Mail" "https://mail.google.com/mail/u/0/?view=cm&fs=1";
    }
    // lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
      # Each makes a new one through its own create page, which docs.new
      # would reach too but in the browser, since opens.sh only gives
      # docs.google.com its window.
      google-docs =
        program "Docs" "Documents in the Google account" "https://docs.google.com/document/"
        // making "New Document" "https://docs.google.com/document/create";
      google-sheets =
        program "Sheets" "Spreadsheets in the Google account" "https://docs.google.com/spreadsheets/"
        // making "New Spreadsheet" "https://docs.google.com/spreadsheets/create";
      google-slides =
        program "Slides" "Presentations in the Google account" "https://docs.google.com/presentation/"
        // making "New Presentation" "https://docs.google.com/presentation/create";
      google-drive = program "Drive" "Files in the Google account" "https://drive.google.com/";
      # A new meeting, opened in Meet's window, where its link is copied.
      google-meet =
        program "Meet" "Video calls in the Google account" "https://meet.google.com/"
        // making "New Meeting" "https://meet.google.com/new";
    };

    xdg.configFile = {
      "quickshell/Google.qml".source = ./Google.qml;
      "quickshell/Google.js".source = ./Google.js;
      "quickshell/Calendar.qml".source = ./Calendar.qml;
      "quickshell/Calendar.js".source = ./Calendar.js;
      "quickshell/Repeats.js".source = ./Repeats.js;
      "quickshell/Mail.qml".source = ./Mail.qml;
      "quickshell/Mail.js".source = ./Mail.js;
      "quickshell/Meet.js".source = ./Meet.js;
      "quickshell/Drive.qml".source = ./Drive.qml;
      "quickshell/Drive.js".source = ./Drive.js;
      "quickshell/DrivePlaces.js".source = ./DrivePlaces.js;
      "quickshell/Saves.qml".source = ./Saves.qml;
      "quickshell/Saves.js".source = ./Saves.js;
    };

    # The account signed out, which wants Joel to sign in again, wearing
    # the signing-in mark; first among what of Google's wants him.
    jitsusama.quickshell.margin.pieces.google = {
      order = 55;
      source = ./SignedOut.qml;
      label = "google";
      tier = "when";
      mark = "signIn";
    };

    # A meeting coming, which wants Joel, wearing the calendar's own mark;
    # among what wants him, after the notices.
    jitsusama.quickshell.margin.pieces.meeting = {
      order = 56;
      source = ./Starting.qml;
      label = "meeting";
      tier = "when";
      mark = "meeting";
    };

    # The mail that wants Joel, wearing the mail mark; after what wants
    # him sooner, since it keeps.
    jitsusama.quickshell.margin.pieces.mail = {
      order = 61;
      source = ./NewMail.qml;
      label = "mail";
      tier = "when";
      mark = "mail";
    };

    # A Meet call, wearing the call mark Tuple's does, since it's a call
    # too; after Tuple's and before the sharing a call often brings.
    jitsusama.quickshell.margin.pieces.meet = {
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
    # Meet's own acts beside its New Meeting: Join by Code, and Copy a New
    # Meeting's Link once Joel's sign-in grants Meet's scope.
    jitsusama.quickshell.launcher.providers.meet = {
      order = 50;
      source = ./Meet.qml;
    };
    # Drive's own places in its program: its Recent of every kind, then
    # Starred, Shared with Me and Shared Drives.
    jitsusama.quickshell.launcher.providers.drive = {
      order = 50;
      source = ./DriveProgram.qml;
    };
    # Each wears a mark of its own in Programs rather than the program
    # mark every window without a logo shares: Calendar the calendar's,
    # apart from a meeting's, and Mail Gmail's own logo.
    jitsusama.quickshell.launcher.providers.applications.glyphs = {
      google-calendar = "mark:calendar";
      google-mail = "f02ab";
    };

    systemd.user.paths.google = {
      Unit.Description = "Starts the Google account's watch once pi has signed in";
      Path.PathExists = credentials;
      Install.WantedBy = [ "default.target" ];
    };

    # The daemon interface's socket is systemd's, so whatever asks finds it
    # there while google starts or starts again, and asking starts it. Like
    # google itself, it's only made once pi has signed in, so a machine
    # nobody signed in to Google on starts nothing.
    systemd.user.sockets.google = {
      Unit = {
        Description = "Where the shell asks the Google account's daemon";
        ConditionPathExists = credentials;
      };
      Socket = {
        ListenStream = "%t/daemons/google.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };

    systemd.user.services.google = {
      Unit = {
        Description = "Keeps the Google account's meetings and mail for the shell";
        # Its daemon socket is systemd's, handed to it as it starts.
        Requires = [ "google.socket" ];
        After = [ "google.socket" ];
        # Stopped before a switch reloads the units, google.path starts it
        # again at once, from the old unit, since the credentials are still
        # there, and the switch's start then finds it running; restarted
        # after the reload, it comes back as the new one.
        X-SwitchMethod = "restart";
      };
      Service = {
        Slice = "background.slice";
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe google)
          (lib.escapeShellArg credentials)
          "%t/daemons/google.sock"
        ];
        Restart = "on-failure";
        RestartSec = 10;
        # The files fetched to open go with the program, as what it held
        # does once its daemon stops answering.
        ExecStopPost = "${pkgs.coreutils}/bin/rm -rf %t/google-files";
      };
    };
  };
}
