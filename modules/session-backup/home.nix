# session-backup copies each of the account's settled pi session logs, packed
# and sealed with age, into one Google Drive folder, every hour, only ever
# adding to it (src/lib.rs says how). A log Drive holds stays on the machine
# for `keepDays` after its last write, then a run removes it. A module names
# the folder and who can open the copies:
#
#   jitsusama.session-backup = {
#     folder = "<the folder's id>";
#     recipients = [ "age1tag1..." "age1fido2-hmac1..." ];
#   };
#
# or, rather than a folder made in Drive's web page, `ownFolder = "<a
# name>"`, one the backup makes at the top of Drive and finds again. With
# neither, nothing runs.
#
# It signs in as a gcloud application-default login, which needs a Drive
# scope gcloud doesn't ask for by itself. A folder made in the web page
# needs the whole of Drive:
#
#   gcloud auth application-default login --scopes=openid,\
#     https://www.googleapis.com/auth/userinfo.email,\
#     https://www.googleapis.com/auth/cloud-platform,\
#     https://www.googleapis.com/auth/drive
#
# That rewrites the one login other tools use too, and any program running
# as the account can read it, so it would hand them all of Drive. Its own
# folder needs only the files the backup made (drive.file), in a login of
# its own, leaving the account's own with no Drive scope at all:
#
#   CLOUDSDK_CONFIG=~/.config/session-backup/gcloud \
#     gcloud auth application-default login --scopes=openid,\
#     https://www.googleapis.com/auth/userinfo.email,\
#     https://www.googleapis.com/auth/cloud-platform,\
#     https://www.googleapis.com/auth/drive.file
#
# with `credentials` naming the application_default_credentials.json it
# writes there. gcloud insists on cloud-platform in any login it makes.
# Drive refuses a login from gcloud's application-default client until it
# names a project to count its calls against, one with Drive's API on:
#
#   CLOUDSDK_CONFIG=~/.config/session-backup/gcloud \
#     gcloud auth application-default set-quota-project <the project>
#
# and the backup names it on every call.
#
# `session-backup status` says what is sent and what waits. A copy is
# opened with age and one of the recipients' keys, then zstd -d.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.session-backup;
  package = pkgs.callPackage ./package.nix { };
  configFile = pkgs.writeText "session-backup.json" (
    builtins.toJSON {
      sessions = "${config.home.homeDirectory}/.pi/agent/sessions";
      ledger = "${config.xdg.stateHome}/session-backup/sent.jsonl";
      # A row for each log sent, which mining reads first (src/index.rs).
      index = "${config.xdg.stateHome}/session-backup/index.jsonl";
      # agentic-harness.pi's record of each session, which names its quest;
      # a session started outside the harness has none, and its row says so.
      records = "${config.xdg.stateHome}/pi/agentic-harness.pi/quest-workflow/sessions";
      inherit (cfg)
        folder
        ownFolder
        machine
        keepDays
        recipients
        settledHours
        ;
      inherit (cfg) credentials identities;
    }
  );
  # The command, told its settings, for the account to ask what's sent and
  # to read a copy back. A flag alone, --json or --help, is status's; any
  # other word reaches the binary as it's given, which says what to try.
  configured = pkgs.writeShellScriptBin "session-backup" ''
    case "''${1:-status}" in
      run | status | fetch) exec ${lib.getExe package} "''${1:-status}" --config ${configFile} "''${@:2}" ;;
      -*) exec ${lib.getExe package} status --config ${configFile} "$@" ;;
      *) exec ${lib.getExe package} "$@" ;;
    esac
  '';
in
{
  # The option said host, which the glossary calls a machine; a machine
  # that still sets the old name reaches the new one, warned.
  imports = [
    (lib.mkRenamedOptionModule
      [ "jitsusama" "session-backup" "host" ]
      [ "jitsusama" "session-backup" "machine" ]
    )
  ];

  options.jitsusama.session-backup = {
    folder = lib.mkOption {
      description = ''
        The id of the Google Drive folder the copies go in, from the end of
        its address. The account's login needs to be able to add to it.
      '';
      type = lib.types.nullOr lib.types.str;
      default = null;
    };
    ownFolder = lib.mkOption {
      description = ''
        The name of a folder the backup makes at the top of Google Drive and
        keeps its copies in, rather than `folder`. Its login then needs only
        the files the backup made, and can read nothing else in Drive.
      '';
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "pi sessions";
    };
    recipients = lib.mkOption {
      description = ''
        Who can open a copy: age recipients, a plugin's such as a TPM's
        age1tag1 or a YubiKey's age1fido2-hmac1 among them.
      '';
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };
    credentials = lib.mkOption {
      description = ''
        The gcloud application-default login the backup signs in as: the
        account's own by default, or one made for the backup alone.
      '';
      type = lib.types.str;
      default = "${config.xdg.configHome}/gcloud/application_default_credentials.json";
      defaultText = lib.literalExpression ''"''${config.xdg.configHome}/gcloud/application_default_credentials.json"'';
    };
    identities = lib.mkOption {
      description = ''
        The age identity files `session-backup fetch` opens a copy with,
        such as a TPM's made without a PIN. Those a machine doesn't hold are
        passed over, so one list serves every machine. With one named, each
        hourly run also reads back up to 100 logs gone from the machine that
        have no row in the index, for their rows, and removes each copy once
        read.
      '';
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = lib.literalExpression ''[ "''${config.xdg.configHome}/age/tpm.txt" ]'';
    };
    machine = lib.mkOption {
      description = "The machine, as each copy's name begins.";
      type = lib.types.str;
    };
    settledHours = lib.mkOption {
      description = ''
        How long a log goes unwritten before it's copied. A session resumed
        later is copied again, so this only spares copying one still going.
      '';
      type = lib.types.ints.positive;
      default = 48;
    };
    keepDays = lib.mkOption {
      description = ''
        How many days after its last write a log stays on the machine. Past
        that, a run removes it, but only once Drive holds its copy under
        that last write; a log Drive no longer finds is kept. Null keeps
        every log.
      '';
      type = lib.types.nullOr lib.types.ints.positive;
      default = 90;
    };
  };

  config =
    lib.mkIf ((cfg.folder != null || cfg.ownFolder != null) && pkgs.stdenv.hostPlatform.isLinux)
      {
        assertions = [
          {
            assertion = cfg.folder == null || cfg.ownFolder == null;
            message = "jitsusama.session-backup names a folder and an own folder; name one.";
          }
          {
            assertion = cfg.recipients != [ ];
            message = "jitsusama.session-backup names a folder but no recipient, so nothing could open a copy.";
          }
        ];

        home.packages = [ configured ];

        systemd.user.services.session-backup = {
          Unit = {
            Description = "Copies the account's settled pi session logs, sealed, to Drive";
            # A run takes most of an hour, so a switch leaves a run going; the
            # next hour's starts on the new program.
            X-RestartIfChanged = false;
            # Tried four times in half an hour before it counts as failed, as
            # set out under Restart.
            StartLimitIntervalSec = "30min";
            StartLimitBurst = 4;
          };
          Service = {
            # Started once it runs, not once it's done: a oneshot's start job
            # lasts the whole run, and the hour a boot catches up on kept the
            # account's manager "starting" for minutes, in which a switch
            # reloads none of the account's units. A timer still never starts
            # a run while one is going.
            Type = "exec";
            ExecStart = "${lib.getExe package} run --config ${configFile}";
            # Whatever the account runs comes first: the slice gives it the
            # disk's idle priority and a small share of the processor.
            Slice = "background.slice";
            # The hour a resume catches up on starts before Wi-Fi is back, and
            # a user's unit can't wait on the machine's network, so a run that
            # fails is tried again a few minutes on. One still failing half an
            # hour later, a login refused or no connection at all, is a failed
            # unit.
            Restart = "on-failure";
            RestartSec = "5min";
          };
        };
        systemd.user.timers.session-backup = {
          Unit.Description = "Copies the account's settled pi session logs every hour";
          Timer = {
            OnCalendar = "hourly";
            RandomizedDelaySec = "10min";
            # An hour slept through runs once the machine wakes.
            Persistent = true;
          };
          Install.WantedBy = [ "timers.target" ];
        };
      };
}
