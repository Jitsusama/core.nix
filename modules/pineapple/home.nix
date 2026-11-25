# The safe word that stops Joel's agents, read where it's said: Slack and
# GitHub, and any site speaking GitHub's API that a machine names in
# jitsusama.pineapple.sites, through Joel's own sign-ins, only read. Slack
# is live since Joel read two days of its shadow log: its stops stand, it
# answers where asked, and it tells his phone. GitHub is live too, where
# only a repository's owner, its organization's members and its
# collaborators stop him, since anyone in the world may comment there.
# Each named site is in shadow, its stops written down with nothing acting
# on them, until its own are read and the machine says it's live
# (src/lib.rs says how the pieces fit). A machine signed in to Slack as Joel
# imports it.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  pineapple = pkgs.callPackage ./package.nix { };
  # The realms this machine's mycelium nodes keep, as cadence reads them,
  # since each realm's agents start and run apart from the others'.
  realms = lib.optionals (options.jitsusama ? mycelium) (
    lib.optional (config.jitsusama.mycelium.realm != null) config.jitsusama.mycelium.realm
    ++ lib.attrNames config.jitsusama.mycelium.alsoJoins
  );
  # Prints pi's Slack sign-in from the desktop's keyring, or says the
  # keyring is locked, as slack-unreads reads it (../slack/keyring.nix).
  keyring = pkgs.callPackage ../slack/keyring.nix { };
  # Where the Slack reader tells For You it can't hear Slack, beside each
  # quest's ask (WaitingOnYou.qml): named once, so the file the reader
  # writes is the file its stop removes.
  waiting = "${config.xdg.stateHome}/mission/waiting/pineapple.json";
  # Prints Joel's GitHub sign-in as gh keeps it, through gh itself, so
  # wherever gh keeps it stays gh's business; it fails while he has none.
  # Its output goes to the program alone.
  githubSignIn = pkgs.writeShellScript "pineapple-github-sign-in" ''
    exec ${lib.getExe pkgs.gh} auth token --hostname github.com
  '';
  # The GitHub reader again for each other site, finding what changed by
  # the pull requests Joel opened in the repositories named, since such a
  # site has no notifications to read (src/bin/pineapple-github/). A site
  # stays in shadow until a machine says it's live, each once Joel has read
  # two days of its own, as Slack went live after its own.
  siteReader =
    name: site:
    lib.nameValuePair "pineapple-${name}" {
      Unit = {
        Description =
          "Reads ${name} for the safe word that stops Joel's agents"
          + lib.optionalString (!site.live) ", in shadow";
        X-SwitchMethod = "restart";
      };
      Service = {
        Slice = "background.slice";
        ExecStart = lib.escapeShellArgs (
          [
            "@${pineapple}/bin/pineapple-github"
            "pineapple-${name}"
            "run"
          ]
          ++ lib.optional site.live "--live"
          ++ [
            "--origin"
            site.origin
            "--site"
            name
            "--authored-by"
            site.authoredBy
            "--login"
            site.login
          ]
          ++ lib.concatMap (repo: [
            "--repo"
            repo
          ]) site.repos
        );
        Environment = [ "SIGN_IN_KEYRING=${site.signIn}" ];
        Restart = "on-failure";
        RestartSec = 30;
      };
      Install.WantedBy = [ "default.target" ];
    };
  inherit (config.jitsusama.pineapple) sites gateway;
  # A word systemd reads as itself in ExecStart, which expands `%` and `$`
  # even inside quotes, as night-shift's units say.
  literal = lib.replaceStrings [ "%" "$" ] [ "%%" "$$" ];
in
{
  imports = [ ../quickshell/providers.nix ];

  # What Joel's own stop on everything pauses beside its record
  # (src/pause.rs): agents' timers, and what's never frozen. Every machine
  # mycelium lists a session on stops with this one, so none is named.
  options.jitsusama.pineapple.pause = {
    timers = lib.mkOption {
      description = "The agents' timers it stops, as names or systemd's patterns.";
      type = lib.types.listOf lib.types.str;
      default = [
        "cadence*.timer"
        "pain-hear.timer"
        "pain-points.timer"
        "night-shift-*.timer"
        "pi-packages.timer"
        "pi-measure.timer"
      ];
    };
    keep = lib.mkOption {
      description = ''
        The units of agents' work it never freezes, as names or patterns
        with `*`: the compile cache Joel's own builds read, the checks a
        merge to main runs, which the stop holds no merge for and which a
        freeze would time out into a revert, and anything started for a
        demo itself.
      '';
      type = lib.types.listOf lib.types.str;
      default = [
        "sccache.service"
        "merge-net.service"
        "changed-checks-keeps-main.service"
        "*demo*"
      ];
    };
    machines = lib.mkOption {
      description = "Other machines that stop with this one, beside each running a session.";
      type = lib.types.listOf lib.types.str;
      default = [ ];
    };
  };

  options.jitsusama.pineapple.sites = lib.mkOption {
    description = ''
      Other sites speaking GitHub's API to read for the safe word, by a name
      of lowercase letters, digits and dashes, each read as pineapple-NAME.
    '';
    default = { };
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          origin = lib.mkOption {
            description = "Where the site's API answers, such as https://code.example.com/api/v3.";
            type = lib.types.str;
          };
          authoredBy = lib.mkOption {
            description = "Joel's address there, whose pull requests are read.";
            type = lib.types.str;
          };
          login = lib.mkOption {
            description = "Joel's login there, as a tag of him reads.";
            type = lib.types.str;
          };
          repos = lib.mkOption {
            description = "Each repository read, as OWNER/NAME, since such a site searches one at a time.";
            type = lib.types.nonEmptyListOf lib.types.str;
          };
          signIn = lib.mkOption {
            description = "A program printing Joel's sign-in there, run each round.";
            type = lib.types.path;
          };
          live = lib.mkOption {
            description = ''
              Whether the site's stops act, once Joel has read its shadow;
              until then each is only written down.
            '';
            type = lib.types.bool;
            default = false;
          };
        };
      }
    );
  };

  options.jitsusama.pineapple.gateway = lib.mkOption {
    description = ''
      A tool gateway reading Slack for Joel, heard by a second reader in
      shadow beside the one asking Slack itself, so the two can be compared
      before the gateway's is trusted; none unless a machine names one.
    '';
    default = null;
    type = lib.types.nullOr (
      lib.types.submodule {
        options = {
          url = lib.mkOption {
            description = "Where the gateway answers MCP, such as https://gateway.example.com/mcp.";
            type = lib.types.str;
          };
          signIn = lib.mkOption {
            description = ''
              A program printing Joel's sign-in to the gateway, run when
              asked, that fetches it as it runs, never one holding it, since
              the program sits in the store where anyone can read it.
            '';
            type = lib.types.path;
          };
        };
      }
    );
  };

  config = lib.mkMerge [
    (lib.mkIf (gateway != null) {
      # The Slack reader again, hearing Slack through the gateway
      # (src/ears.rs), only ever in shadow, as the reader refuses --live
      # with --gateway: it keeps to pineapple-gateway beside the live
      # reader's state, its row of what waits included, however it's run,
      # so a gateway it can't hear never reaches Joel while the reader he
      # trusts goes on. The environment is left alone, for the sign-in
      # program it runs.
      systemd.user.services.pineapple-slack-gateway = {
        Unit = {
          Description = "Reads Slack through a tool gateway for the safe word, in shadow";
          X-SwitchMethod = "restart";
        };
        Service = {
          Slice = "background.slice";
          ExecStart = lib.escapeShellArgs [
            "@${pineapple}/bin/pineapple-slack"
            "pineapple-slack-gateway"
            "run"
            "--gateway"
            (literal gateway.url)
            "--gateway-sign-in"
            (literal "${gateway.signIn}")
          ];
          Restart = "on-failure";
          RestartSec = 30;
        };
        Install.WantedBy = [ "default.target" ];
      };
    })
    {
      # A name is a unit's and a file's, and GitHub's and Slack's are taken.
      # A site allows 30 searches a minute and a round spends one a
      # repository, so the reader refuses more than 20; refused here, a
      # switch fails rather than the reader restarting on it for good.
      # The gateway is given Joel's sign-in with every request.
      assertions =
        lib.optional (gateway != null) {
          assertion = lib.hasPrefix "https://" gateway.url;
          message = "jitsusama.pineapple.gateway.url is sent Joel's sign-in, so it's https.";
        }
        ++ lib.concatLists (
          lib.mapAttrsToList (name: site: [
            {
              assertion =
                builtins.match "[a-z0-9-]+" name != null
                && !lib.elem name [
                  "github"
                  "slack"
                ];
              message = "jitsusama.pineapple.sites.${name}: a site's name is lowercase letters, digits and dashes, and neither github nor slack.";
            }
            {
              assertion = lib.length site.repos <= 20;
              message = "jitsusama.pineapple.sites.${name}: a site reads at most 20 repositories, since each round spends one of its 30 searches a minute on each.";
            }
          ]) sites
        );
      systemd.user.services = lib.mapAttrs' siteReader sites;
    }
    {

      home.packages = [ pineapple ];

      # Read by pineapple stop all and by serve's Pause All Agents.
      xdg.configFile."pineapple/pause.json".text = builtins.toJSON {
        systemctl = lib.getExe' pkgs.systemd "systemctl";
        # mycelium as the machine installs it, which isn't this module's.
        mycelium = "${config.home.profileDirectory}/bin/mycelium";
        inherit (config.jitsusama.pineapple.pause) timers keep machines;
        inherit realms;
      };

      # Its rows in For You (SafeWord.qml), read from `pineapple serve` below;
      # on a machine without the shell nothing reads it. Beside Sound's, since
      # both only give For You what to do.
      jitsusama.quickshell.launcher.providers.pineapple = {
        order = 46;
        source = ./SafeWord.qml;
      };

      systemd.user.services.pineapple-slack = {
        Unit = {
          Description = "Reads Slack for the safe word that stops Joel's agents";
          # Restarted after a switch reloads the units, it comes back as the
          # new one.
          X-SwitchMethod = "restart";
        };
        Service = {
          Slice = "background.slice";
          # Shown in ps by its name, not its store path (docs/daemons.md).
          # Without --live it only writes down what it would do; taking the
          # word out, with its check's, is the way back to shadow.
          ExecStart = "@${lib.getExe pineapple} pineapple-slack run --live";
          # The keyring's reader, run each round, so a sign-in made or renewed
          # while it runs is read without a restart. With none, it says so in
          # the journal and reads nothing until there is one. And the file it
          # tells For You in, the one its stop removes, quoted as systemd
          # reads a setting, as the stop's paths are.
          Environment = [
            "SIGN_IN_KEYRING=${keyring}"
            ''"SAFE_WORD_WAITING=${waiting}"''
          ];
          Restart = "on-failure";
          RestartSec = 30;
          # Its row in For You saying it can't hear Slack (src/deaf.rs) goes
          # however it stops, as slack-unreads' does, so a reader stopped or
          # taken away leaves nothing standing that nothing would clear.
          ExecStopPost = lib.concatStringsSep " " [
            "${pkgs.coreutils}/bin/rm -f"
            (lib.escapeShellArg waiting)
            (lib.escapeShellArg "${dirOf waiting}/.writing/${baseNameOf waiting}")
          ];
        };
        # Started at login rather than once a sign-in file appears, since pi
        # keeps its sign-in in the keyring now and the file may never come.
        Install.WantedBy = [ "default.target" ];
      };

      # The same reader for GitHub (src/bin/pineapple-github/): every minute,
      # the pull requests and issues Joel takes part in that changed, and
      # what was said on them, only asked. Its stops go where the
      # Slack reader's do, so the launcher, the harness and his phone take
      # them alike; it answers nobody on GitHub.
      systemd.user.services.pineapple-github = {
        Unit = {
          Description = "Reads GitHub for the safe word that stops Joel's agents";
          X-SwitchMethod = "restart";
        };
        Service = {
          Slice = "background.slice";
          # Live, where only the repository's own stop him (src/github.rs);
          # taking --live out, with its check's, is the way back to shadow.
          ExecStart = "@${pineapple}/bin/pineapple-github pineapple-github run --live";
          # Run each round, so a sign-in renewed while it runs is read without
          # a restart; with none, it says so and reads nothing.
          Environment = [ "SIGN_IN_KEYRING=${githubSignIn}" ];
          Restart = "on-failure";
          RestartSec = 30;
        };
        Install.WantedBy = [ "default.target" ];
      };

      # The safe word's daemon for the launcher (src/serve.rs): it serves the
      # stops, alerts and resume requests as Needs You's rows over
      # %t/daemons/pineapple.sock, takes Joel's presses only from his shell,
      # and knocks as each first appears. In shadow it serves nothing and only
      # writes down what it would have told. It only reads what pineapple-slack
      # writes, and writes only under ~/.local/state/pineapple.
      systemd.user.services.pineapple = {
        Unit = {
          Description = "Shows Joel the safe word's stops in the launcher";
          X-SwitchMethod = "restart";
          # Its socket is systemd's, handed to it as it starts.
          Requires = [ "pineapple.socket" ];
          After = [ "pineapple.socket" ];
        };
        Service = {
          Slice = "background.slice";
          # Started as @<program> pineapple, so ps reads its name rather than a
          # store path (docs/daemons.md).
          ExecStart = lib.escapeShellArgs [
            "@${pineapple}/bin/pineapple"
            "pineapple"
            "serve"
            "--socket"
            "%t/daemons/pineapple.sock"
            "--vocabulary"
            "${(import ../quickshell/launcher/words.nix).file}"
            "--notify-send"
            (lib.getExe' pkgs.libnotify "notify-send")
            # A notice is closed once its stop no longer stands, so the alarm
            # goes with what it was about.
            "--busctl"
            (lib.getExe' pkgs.systemd "busctl")
          ];
          Restart = "on-failure";
          RestartSec = 5;
        };
        # Started at login rather than when the launcher first asks, so a stop
        # knocks even before Joel opens it.
        Install.WantedBy = [ "default.target" ];
      };

      systemd.user.sockets.pineapple = {
        Unit.Description = "Where the launcher asks for the safe word's rows";
        Socket = {
          ListenStream = "%t/daemons/pineapple.sock";
          SocketMode = "0600";
          DirectoryMode = "0700";
        };
        Install.WantedBy = [ "sockets.target" ];
      };
    }
  ];
}
