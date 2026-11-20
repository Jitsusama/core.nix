# repos lists every repository on the machine once, with every checkout of
# it and every tree cut from each (src/lib.rs says how). It reads git's own
# files as it runs, and merges in what GitHub last held, which a timer asks
# every five minutes through the GitHub broker (src/github.rs says why).
# Where it looks differs between machines: a machine whose trees lie in a
# folder of their own, under no checkout, adds that folder. So do the few
# repositories the App can't see that a machine has read through gh's
# sign-in instead (src/gh.rs says how narrowly). A second timer counts how
# far each head is from the branch it follows, with git, and keeps the
# counts for every reading to merge in too (src/upstreams.rs says why).
#
# On Linux it's also a daemon, holding the record ready over
# $XDG_RUNTIME_DIR/daemons/repos.sock in the shell's daemon interface, read
# again as git writes, for the local sites' Code page (src/serve.rs says
# how).
#
# Each refresh also says to the machine's realm, as its GitHub receptor, how
# many open changes the owners declared for that realm hold, how many ask
# the declared login's review and which, each as owner/name#N. A sound read
# names those owners too, so mycelium can clear what it kept; a failed read
# names neither (src/receptor.rs). The login and the owners come from here
# alone, never from a file repos reads, and a machine declaring no realm or
# no login says nothing.
{ mycelium }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  repos = pkgs.callPackage ./package.nix { };
  inherit (config.jitsusama.repos)
    roots
    throughGh
    receptorLogin
    receptorOwners
    ;
  realm = config.jitsusama.mycelium.realm or null;
  # What the refresh's unit gives it to say the receptor with, set in the
  # unit alone rather than as the command's defaults, so what is counted is
  # what the machine declares. Every realm's owners are given, so an owner
  # declared for two is counted in neither, joined by ';' into one word,
  # since home-manager writes each item unquoted and systemd splits an
  # unquoted one at whitespace, which would lose a realm's owners.
  myceliumProgram = lib.getExe mycelium.packages.${pkgs.stdenv.hostPlatform.system}.default;
  owner = lib.types.strMatching "[A-Za-z0-9._-]+";
  owned = name: owners: "${name}=${lib.concatStringsSep "," owners}";
  receptor = lib.optionals (realm != null && receptorLogin != null) [
    "MYCELIUM=${myceliumProgram}"
    "MYCELIUM_REALM=${realm}"
    "REPOS_RECEPTOR_LOGIN=${receptorLogin}"
    "REPOS_RECEPTOR_OWNERS=${lib.concatStringsSep ";" (lib.mapAttrsToList owned receptorOwners)}"
  ];
  # The command's settings, as defaults a caller may still set for one
  # run. gh is named only where a machine lists repositories for it, so
  # it's never asked elsewhere. The login whose review the receptor counts
  # is the one the collection leads with what waits on.
  settings = {
    REPOS_ROOTS = lib.concatStringsSep ":" roots;
    REPOS_GIT = lib.getExe config.programs.git.package;
  }
  // lib.optionalAttrs (receptorLogin != null) {
    REPOS_LOGIN = receptorLogin;
  }
  // lib.optionalAttrs (throughGh != [ ]) {
    REPOS_THROUGH_GH = lib.concatStringsSep " " throughGh;
    REPOS_GH = lib.getExe config.programs.gh.package;
  };
  wrapped = pkgs.symlinkJoin {
    name = "repos-${repos.version}";
    paths = [ repos ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/repos ${
        lib.concatStringsSep " " (
          lib.mapAttrsToList (name: value: "--set-default ${name} ${lib.escapeShellArg value}") settings
        )
      }
    '';
    inherit (repos) meta;
  };
in
{
  options.jitsusama.repos.roots = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ "${config.home.homeDirectory}/src" ];
    description = ''
      The folders repos looks under for checkouts, four folders deep. A
      tree is found from the checkout it was cut from, wherever it lies,
      so only a folder of trees whose checkout is under no root needs
      naming.
    '';
  };

  options.jitsusama.repos.throughGh = lib.mkOption {
    type = lib.types.listOf (lib.types.strMatching "github:[^/ ]+/[^/ ]+");
    default = [ ];
    example = [ "github:someone/private" ];
    description = ''
      Repositories, by their keys, whose open changes are read through
      gh's own sign-in rather than the GitHub App, which can't see them.
      Read-only and fixed: one query, to GitHub's GraphQL endpoint alone,
      with the token held in memory for each refresh. None by default.
    '';
  };

  options.jitsusama.repos.receptorLogin = lib.mkOption {
    type = lib.types.nullOr (lib.types.strMatching "[A-Za-z0-9](-?[A-Za-z0-9])*");
    default = null;
    example = "someone";
    description = ''
      The GitHub login whose asked reviews the GitHub receptor counts and
      names to mycelium, as GitHub matches logins, case aside, and whose
      waiting changes `repos` leads with. None by default, and with none
      the receptor is never said and `repos` leads with nothing.
    '';
  };

  options.jitsusama.repos.receptorOwners = lib.mkOption {
    type = lib.types.attrsOf (lib.types.listOf owner);
    default = { };
    example = {
      work = [ "some-company" ];
      personal = [ "someone" ];
    };
    description = ''
      The GitHub owners whose repositories each realm's receptor counts and
      names changes of, by realm; mycelium is told them too, and takes no
      other owner's change. A machine counts only its own realm's, and an
      owner listed under two realms is counted in neither.
    '';
  };

  config = {
    home.packages = [ wrapped ];
    # How agents are to ask it, and the Code words to answer in.
    home.file.".pi/agent/skills/repos/SKILL.md".source = ./SKILL.md;

    # A realm's name, like an owner's, holds no whitespace, ';' or '=', so
    # it can't break the one word every realm's owners are given in.
    assertions = [
      {
        assertion = lib.all (name: builtins.match "[A-Za-z0-9._-]+" name != null) (
          lib.attrNames receptorOwners
        );
        message = "jitsusama.repos.receptorOwners names a realm with more than letters, digits, '.', '_' or '-'.";
      }
    ];

    # The refresh asks only through the broker's socket, so a machine
    # without one skips it rather than failing every five minutes.
    systemd.user.services.repos-refresh = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Asks GitHub after each repository's open changes, for repos";
        After = [ "github-broker.socket" ];
        Wants = [ "github-broker.socket" ];
        ConditionPathExists = "%t/github-broker.sock";
        # Three tries in two minutes before it counts as failed, as set
        # out under RestartForceExitStatus.
        StartLimitIntervalSec = "2min";
        StartLimitBurst = 3;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe wrapped} refresh";
        Slice = "background.slice";
        # Exit 69 (unavailable) is GitHub or the broker out of reach, which
        # is mostly a moment's blip: on 16 November one failed name lookup
        # failed the unit, and the next run, five minutes on, was clean.
        # So it's tried again twice, 30 seconds apart, and an outage that
        # lasts still fails the unit within a minute. Any other failure,
        # such as GitHub refusing, fails it at once, as before.
        RestartForceExitStatus = "69";
        RestartSec = "30s";
        Environment = receptor;
      };
    };
    systemd.user.timers.repos-refresh = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Refreshes repos's GitHub changes every five minutes";
      Timer = {
        OnBootSec = "1min";
        OnUnitActiveSec = "5min";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # The counting needs only git, so it runs apart from the refresh, which
    # waits on the broker. Its first run counts every head apart from its
    # branch, about a minute on a machine with hundreds of trees; each
    # after counts only the pairs of commits that moved.
    systemd.user.services.repos-count = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Counts how far each head is from its branch, for repos";
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe wrapped} count";
        Slice = "background.slice";
      };
    };
    systemd.user.timers.repos-count = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Counts how far repos's heads are from their branches every five minutes";
      Timer = {
        OnBootSec = "2min";
        OnUnitActiveSec = "5min";
      };
      Install.WantedBy = [ "timers.target" ];
    };

    # In background.slice: it reads the disk as git writes, which agents
    # do all day, and a view a moment behind costs Joel nothing.
    systemd.user.services.repos = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit = {
        Description = "Holds every repository ready for its views, read again as git writes";
        # Restarted after a switch reloads the units, it comes back as the
        # new one.
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "repos.socket" ];
        After = [ "repos.socket" ];
      };
      Service = {
        # Run by its name, so ps -ef reads repos rather than a store path
        # (docs/daemons.md, Shown). The wrapper would exec it under its own
        # path, so it runs bare, given the wrapper's settings here.
        ExecStart = "@${lib.getExe repos} repos serve";
        Environment = lib.mapAttrsToList (name: value: ''"${name}=${value}"'') settings;
        Restart = "on-failure";
        RestartSec = 5;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # Its socket, held by systemd whether or not the daemon runs, so a view
    # asking while it starts waits on it rather than finding no socket.
    systemd.user.sockets.repos = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
      Unit.Description = "Where the views ask the repos daemon";
      Socket = {
        ListenStream = "%t/daemons/repos.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
  };
}
