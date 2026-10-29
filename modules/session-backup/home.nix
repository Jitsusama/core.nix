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
# With no folder named, nothing runs. It signs in as the account's gcloud
# application-default login, which needs a Drive scope gcloud doesn't ask
# for by itself, so the account signs in once with
#
#   gcloud auth application-default login --scopes=openid,\
#     https://www.googleapis.com/auth/userinfo.email,\
#     https://www.googleapis.com/auth/cloud-platform,\
#     https://www.googleapis.com/auth/drive
#
# That rewrites the one login other tools may use too. To give the backup
# its own, sign in with CLOUDSDK_CONFIG set to a folder of its own and name
# the file it writes there as `credentials`.
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
      inherit (cfg)
        folder
        host
        keepDays
        recipients
        settledHours
        ;
      inherit (cfg) credentials;
    }
  );
  # The command, told its settings, for the account to ask what's sent.
  configured = pkgs.writeShellScriptBin "session-backup" ''
    case "''${1:-status}" in
      run | status) exec ${lib.getExe package} "''${1:-status}" --config ${configFile} "''${@:2}" ;;
      *) echo "usage: session-backup [status | run [--most N]]" >&2; exit 2 ;;
    esac
  '';
in
{
  options.jitsusama.session-backup = {
    folder = lib.mkOption {
      description = ''
        The id of the Google Drive folder the copies go in, from the end of
        its address. The account's login needs to be able to add to it.
      '';
      type = lib.types.nullOr lib.types.str;
      default = null;
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
    host = lib.mkOption {
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

  config = lib.mkIf (cfg.folder != null && pkgs.stdenv.hostPlatform.isLinux) {
    assertions = [
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
