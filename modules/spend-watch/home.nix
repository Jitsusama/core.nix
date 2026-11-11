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
# One for a Claude plan is kept here, as claudePlanFeed (feeds/claude/):
#
#   jitsusama.spend-watch.feeds.claude = {
#     label = "Claude";
#     command = [ (lib.getExe config.jitsusama.spend-watch.claudePlanFeed) ];
#   };
#
# With none, every figure is pi's own at list prices, and says so. Where the
# money comes from more than one place, each is a budget, and a feed says
# which is its own:
#
#   jitsusama.spend-watch.budgets.proxy = {
#     label = "Your proxy cap"; kind = "cap"; whose = "employer"; default = true;
#   };
#   jitsusama.spend-watch.budgets.delta = {
#     label = "Delta team token"; kind = "team"; whose = "team";
#   };
#   jitsusama.spend-watch.feeds.proxy.budget = "proxy";
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
    # The launcher's own words, so the rows the daemon serves ready say
    # what the launcher would (src/ready.rs).
    vocabulary = "${../quickshell/launcher/words.json}";
    feeds = lib.mapAttrsToList (
      name: feed:
      {
        inherit name;
        inherit (feed) label command environment;
      }
      // lib.optionalAttrs (feed.budget != null) { inherit (feed) budget; }
    ) cfg.feeds;
    budgets = lib.mapAttrsToList (
      name: budget:
      {
        inherit name;
        inherit (budget)
          label
          kind
          whose
          default
          providers
          ;
        allowances = lib.mapAttrsToList (
          name: allowance:
          {
            inherit name;
            inherit (allowance) unit windowHours warn;
          }
          // lib.optionalAttrs (allowance.allowance != null) { inherit (allowance) allowance; }
          // lib.optionalAttrs (allowance.resets != null) { inherit (allowance) resets; }
        ) budget.allowances;
      }
      // lib.optionalAttrs (budget.price != null) { inherit (budget) price; }
    ) cfg.budgets;
    notices = {
      inherit (cfg.notices) enable perWeek daily;
    };
    detectors = {
      inherit (cfg.detectors)
        burnRunLengthDays
        falseDiscovery
        floor
        notesOnly
        ;
    };
    inherit (cfg)
      premium
      overrideLead
      weeklyReport
      refusals
      providerNames
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
            budget = lib.mkOption {
              description = ''
                The budget its bill and caps are for, unless a record names another;
                the default budget when null.
              '';
              type = lib.types.nullOr lib.types.str;
              default = null;
            };
          };
        }
      );
    };

    budgets = lib.mkOption {
      description = ''
        Where pi's requests draw their money from, by name: a Claude plan, an API
        key, a proxy's cap, a team's token. Exactly one is the default, which a
        turn draws on when nothing says otherwise. With none declared, all of it
        is one budget, `default`, labelled "pi".
      '';
      default = { };
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            label = lib.mkOption {
              description = "What it's called where it's shown.";
              type = lib.types.str;
            };
            kind = lib.mkOption {
              description = "What it is: a Claude plan, a key, a cap or a team's token.";
              type = lib.types.enum [
                "plan"
                "key"
                "cap"
                "team"
              ];
            };
            whose = lib.mkOption {
              description = "Whose money it is.";
              type = lib.types.enum [
                "own"
                "employer"
                "team"
              ];
            };
            default = lib.mkOption {
              description = "Whether it's the one a turn draws on when nothing says otherwise.";
              type = lib.types.bool;
              default = false;
            };
            providers = lib.mkOption {
              description = ''
                pi's providers whose turns are always this budget's, whatever the
                session's log says, as `claude-code` is a Claude plan's.
              '';
              type = lib.types.listOf lib.types.str;
              default = [ ];
            };
            price = lib.mkOption {
              description = "What a plan costs, billed whether it's used or not.";
              default = null;
              type = lib.types.nullOr (
                lib.types.submodule {
                  options = {
                    dollars = lib.mkOption {
                      description = "Dollars a period.";
                      type = lib.types.numbers.nonnegative;
                    };
                    per = lib.mkOption {
                      description = "The period the price is for.";
                      type = lib.types.enum [ "month" ];
                      default = "month";
                    };
                  };
                }
              );
            };
            allowances = lib.mkOption {
              description = ''
                What it may spend in a period, by name: dollars against an allowance,
                or a share of a plan's window. A feed's cap of the same name for this
                budget is the same allowance, read.
              '';
              default = { };
              type = lib.types.attrsOf (
                lib.types.submodule {
                  options = {
                    unit = lib.mkOption {
                      description = "What it counts in.";
                      type = lib.types.enum [
                        "dollars"
                        "share"
                      ];
                      default = "dollars";
                    };
                    allowance = lib.mkOption {
                      description = "Dollars allowed in the window, for an allowance in dollars.";
                      type = lib.types.nullOr lib.types.numbers.positive;
                      default = null;
                    };
                    windowHours = lib.mkOption {
                      description = "The window, in hours.";
                      type = lib.types.ints.positive;
                    };
                    resets = lib.mkOption {
                      description = ''
                        An instant it resets at, RFC 3339, for one counted over periods
                        rather than a rolling window; it resets every window after.
                      '';
                      type = lib.types.nullOr lib.types.str;
                      default = null;
                    };
                    warn = lib.mkOption {
                      description = "The share of it to warn at.";
                      type = lib.types.numbers.between 0 1;
                      default = 0.9;
                    };
                  };
                }
              );
            };
          };
        }
      );
    };

    providerNames = lib.mkOption {
      description = ''
        What each of pi's providers is called as it names itself, by its id, for
        where a budget lists the providers it always draws for. One not named
        here is said by its id.
      '';
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = {
        anthropic = "Anthropic";
      };
    };

    claudePlanFeed = lib.mkOption {
      description = ''
        The feed for a Claude plan, for a machine on one to name in `feeds`: how
        much of the plan's five hours and week is used, read with the sign-in
        pi-claude keeps (feeds/claude/README.md).
      '';
      type = lib.types.package;
      readOnly = true;
      default = pkgs.callPackage ./feeds/claude/package.nix { };
      defaultText = lib.literalMD "feeds/claude, built";
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
      # A swarm's agents each keep a home at the top of their quest's
      # workspace; a review's reviewers and judges keep theirs with its
      # transcripts. A pattern never reaches into a workspace's tmp: on 7
      # November it held 46,272 folders of others' scratch, which the walks
      # opened 1.5 million times in 150 s, and no agent had written a log
      # there since a swarm's trials of 14 October.
      default = [
        {
          pattern = "${workspaces}/*/*/agents/*/session/*.jsonl";
          kind = "swarm";
        }
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
      perWeek = lib.mkOption {
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
      notesOnly = lib.mkOption {
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
      description = ''
        The default budget's allowance in dollars, held where no feed gives that
        budget a cap.
      '';
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
        "over your weekly cap"
        "credit balance is too low"
        "usage limit"
      ];
    };
  };

  config = {
    home.packages = [ package ];

    # The claude-code provider drives Claude Code, whose plan this module's
    # own feed reads (claudePlanFeed).
    jitsusama.spend-watch.providerNames.claude-code = lib.mkDefault "Claude Code";

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
