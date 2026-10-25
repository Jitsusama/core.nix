# spend-watch keeps the account told what its AI use costs: a daemon that
# reads pi's session logs as they grow, its agents' too (a swarm's, a
# review council's), the harness's subagent runs and its quests, and
# whatever feeds a machine adds for a provider's bill, keeps a history of
# every dollar, and writes what the shell draws: the overview's
# row, the margin while something wants Joel, and the launcher's collection
# on Super+U. AGENTS.md beside this says how it decides.
#
# A machine whose provider can say what it bills adds a feed, a program
# speaking the contract in contract/README.md:
#
#   jitsusama.spend-watch.feeds.proxy = {
#     label = "the AI proxy";
#     command = [ (lib.getExe feed) ];
#   };
#
# With none, every figure is pi's own at list prices, and says so.
{
  config,
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for the zone days are local to.
  osConfig ? { },
  ...
}:
let
  cfg = config.jitsusama.spend-watch;
  package = pkgs.callPackage ./package.nix { };
  home = config.home.homeDirectory;
  inherit (config.xdg) cacheHome dataHome stateHome;
  # Where the coding harness keeps what it knows of runs, sessions and quests.
  harness = {
    state = "${stateHome}/pi/agentic-harness.pi";
    data = "${dataHome}/pi/agentic-harness.pi";
    cache = "${cacheHome}/pi/agentic-harness.pi";
  };
  workspaces = cfg.paths.workspaces;

  rendered = {
    sessions = cfg.paths.sessions;
    inherit (cfg) agents;
    inherit workspaces;
    runs = cfg.paths.runs;
    registry = cfg.paths.registry;
    quests = cfg.paths.quests;
    state = cfg.paths.state;
    feeds = lib.mapAttrsToList (name: feed: {
      inherit name;
      inherit (feed) label command environment;
    }) cfg.feeds;
    notices = {
      inherit (cfg.notices) enable budget daily;
    };
    detectors = {
      inherit (cfg.detectors)
        burnRunLengthDays
        falseDiscovery
        floor
        quiet
        ;
    };
    inherit (cfg)
      premium
      overrideLead
      weeklyReport
      refusals
      ;
  }
  // lib.optionalAttrs (cfg.zone != null) { inherit (cfg) zone; }
  // lib.optionalAttrs (cfg.cap != null) { inherit (cfg) cap; };

  # The config as the daemon reads it, refused at build by the daemon itself.
  configFile =
    pkgs.runCommandLocal "spend-watch.json"
      {
        text = builtins.toJSON rendered;
        passAsFile = [ "text" ];
      }
      ''
        ${lib.getExe package} check-config "$textPath"
        cp "$textPath" $out
      '';

  path =
    description: default:
    lib.mkOption {
      inherit description default;
      type = lib.types.str;
    };
in
{
  options.jitsusama.spend-watch = {
    feeds = lib.mkOption {
      description = ''
        Programs that speak for a provider's bill, by name: each is started,
        given its environment, and restarted when it stops.
      '';
      default = { };
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            label = lib.mkOption {
              description = "What it's called where its health is shown.";
              type = lib.types.str;
            };
            command = lib.mkOption {
              description = "The program and its arguments.";
              type = lib.types.nonEmptyListOf lib.types.str;
            };
            environment = lib.mkOption {
              description = ''
                Variables it's started with, beside the daemon's own. They're written
                into the Nix store, which anyone on the machine can read, so never a
                secret: a feed reads its credentials itself, from a command or a file.
              '';
              type = lib.types.attrsOf lib.types.str;
              default = { };
            };
          };
        }
      );
    };

    paths = {
      sessions = path "pi's session logs." "${home}/.pi/agent/sessions";
      runs = path "The harness's record of subagent runs." "${harness.state}/observability/runs.db";
      registry = path "Open sessions, to go to one." "${harness.state}/quest-workflow/sessions";
      quests = path "The harness's quests." "${harness.data}/quest-workflow/quests";
      workspaces = path "Each quest's workspace, named by its id." "${harness.cache}/quest-workspace";
      state = path "Where the history and the weekly reports are kept." "${stateHome}/spend-watch";
    };

    agents = lib.mkOption {
      description = ''
        Where pi keeps the logs of agents nobody types into, each a whole path
        with `*` for any one name, and the kind of work they are. A run's
        record says more of its agents' sessions, and wins.
      '';
      type = lib.types.listOf (
        lib.types.submodule {
          options = {
            pattern = lib.mkOption {
              description = "The logs' paths, each `*` one name.";
              type = lib.types.str;
            };
            kind = lib.mkOption {
              description = "The kind of work they are, one word.";
              type = lib.types.strMatching "[A-Za-z0-9-]+";
            };
          };
        }
      );
      # A swarm's agents each keep a home in their quest's workspace, at its
      # top or in a trial under its tmp; a review's reviewers and judges
      # keep theirs with its transcripts.
      default =
        map
          (home: {
            pattern = "${workspaces}/*/${home}agents/*/session/*.jsonl";
            kind = "swarm";
          })
          [
            "*/"
            "tmp/*/"
            "tmp/*/*/"
          ]
        ++ [
          {
            pattern = "${harness.state}/review/transcripts/runs/*/reviewers/*/session/*.jsonl";
            kind = "review";
          }
        ];
      defaultText = lib.literalMD "a swarm's homes in each quest's workspace, and review transcripts";
    };

    zone = lib.mkOption {
      description = "The zone days are local to; the machine's own when null.";
      type = lib.types.nullOr lib.types.str;
      default = osConfig.time.timeZone or null;
    };

    notices = {
      enable = lib.mkOption {
        description = "Whether a finding that wants the person is a notice too.";
        type = lib.types.bool;
        default = true;
      };
      budget = lib.mkOption {
        description = "Notices a week, outside a cap's, so the watch never nags.";
        type = lib.types.ints.unsigned;
        default = 3;
      };
      daily = lib.mkOption {
        description = "One quiet line each morning saying how yesterday went.";
        type = lib.types.bool;
        default = true;
      };
    };

    detectors = {
      burnRunLengthDays = lib.mkOption {
        description = "Days between false alarms the burn's threshold is set for.";
        type = lib.types.numbers.positive;
        default = 30;
      };
      falseDiscovery = lib.mkOption {
        description = "The false discovery rate held across everything compared at once.";
        # Below one: at one, everything compared would count as found.
        type = lib.types.addCheck (lib.types.numbers.between 0 1) (rate: rate < 1);
        default = 5.0e-2;
      };
      floor = lib.mkOption {
        description = "The least money, in dollars, a finding is made for.";
        type = lib.types.numbers.nonnegative;
        default = 25;
      };
      quiet = lib.mkOption {
        description = "Detectors whose findings are only ever notes, by name.";
        type = lib.types.listOf lib.types.str;
        default = [ ];
      };
    };

    premium = lib.mkOption {
      description = "Parts of model names that count as premium.";
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };

    overrideLead = lib.mkOption {
      description = "Days before a raised cap ends to warn, earliest first.";
      type = lib.types.listOf lib.types.ints.positive;
      default = [
        3
        1
      ];
    };

    cap = lib.mkOption {
      description = "A cap to hold spend to where no feed gives one.";
      default = null;
      type = lib.types.nullOr (
        lib.types.submodule {
          options = {
            allowance = lib.mkOption {
              description = "Dollars allowed in the window.";
              type = lib.types.numbers.positive;
            };
            windowHours = lib.mkOption {
              description = "The rolling window, in hours.";
              type = lib.types.ints.positive;
              default = 168;
            };
            warn = lib.mkOption {
              description = "The share of the allowance to warn at.";
              type = lib.types.numbers.between 0 1;
              default = 0.9;
            };
          };
        }
      );
    };

    weeklyReport = lib.mkOption {
      description = "Whether last week's report is written each Monday morning.";
      type = lib.types.bool;
      default = true;
    };

    refusals = lib.mkOption {
      description = ''
        What a provider's error says when it refuses for being over a cap,
        matched without case in pi's logs, so being cut off is seen with no
        feed at all.
      '';
      type = lib.types.listOf lib.types.str;
      default = [
        "over the allowed cap"
        "credit balance is too low"
        "usage limit"
      ];
    };
  };

  config = {
    home.packages = [ package ];

    xdg.configFile."spend-watch/config.json".source = configFile;

    # So an agent asked what something cost answers from the bill.
    home.file.".pi/agent/skills/spend-watch/SKILL.md".source = ./SKILL.md;

    systemd.user.services.spend-watch = {
      Unit = {
        Description = "Keeps the account told what its AI use costs";
        X-Restart-Triggers = [ "${configFile}" ];
      };
      Service = {
        ExecStart = "${lib.getExe package} --config ${configFile} run";
        # Whatever the account runs comes first; this only watches.
        Slice = "background.slice";
        RuntimeDirectory = "spend-watch";
        # Kept when the watch stops, so the shell finds its last status and
        # says nothing watches rather than showing nothing at all.
        RuntimeDirectoryPreserve = "yes";
        Restart = "on-failure";
        RestartSec = "5s";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
}
