# A Mac's shared pi hosts: for each realm the primary account names in its
# jitsusama.pi.hosts (host.nix), an attended and a background host, each a
# launchd daemon run as that account. An agent in the account's own domain
# would be simpler, but launchd ends those with the login session, and every
# conversation a host holds with it; a daemon starts at boot and outlives a
# log-out. Each is started as host-launch.nix says, as Linux's are.
#
# launchd knows nothing of what systemd gives Linux's hosts, so a Mac's go
# without it. No watchdog: the host pings only where systemd asks it to. No
# cap on restarts and no notice of a host left down, so the start keeps its
# own: a fourth start within ten minutes waits ten minutes first, as
# Linux's allows three. A start by hand counts, as does a host's own
# restart onto a new pi, so removing the log's .starts file beside it ends
# the wait within ten seconds. No systemctl for a host to restart itself
# through onto a new pi or harness, so it exits with a failure, which
# launchd starts again. No call heard elsewhere: a Mac's a-call-is-on hears
# only this Mac, so a host's own restart waits out a call here, not one on
# another of the person's machines. No exit the host is never restarted
# from: one it was refused with, 78, restarts as any failure does and says why in its
# log again each time, so a start past 10 MB of log moves it aside, keeping
# one old one. No journal: a running host writes into the same log until
# its next start, where Linux's background host logs about a megabyte a
# day. No mark a command
# can't shed: a Mac has no cgroup, so a command whose parent exits, as one
# put in the background does, is adopted by launchd at once, and mycelium
# then finds no pi-host above it and tells it as the person's; when a host
# exits, launchd kills what is left in its process group, but not one in a
# group of its own. Nothing here fences a hosted command, so from boot,
# with nobody logged in, one has whatever reach the machine's fence leaves
# the account. A reboot waits up to the 150 s a host parks within, where
# Linux's gives it 5 s, and so does a switch that changes a host's plist,
# since nix-darwin unloads each daemon in turn. Run in the system's domain,
# outside any login session, a host may be refused the account's keychain
# and the Secure Enclave keys its agents sign with, even while the account
# is logged in. No privacy grant a terminal holds: macOS refuses a hosted
# command ~/Documents, ~/Desktop, ~/Downloads and iCloud Drive without
# asking, and a grant given the host's program is lost with its store path.
#
# A log-out ends the realm's node, a login agent (mycelium/home.nix), so
# the hosts keep their conversations, but no other machine reaches them
# until the next login brings the node back. A host is restarted by hand
# with `sudo launchctl kickstart -k system/org.jitsusama.pi-host-<realm>-<kind>`.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  launch = import ./host-launch.nix { inherit lib; };

  account = config.system.primaryUser;
  home = if account == null then null else config.home-manager.users.${account} or null;
  hosts = if home == null then { } else home.jitsusama.pi.hosts or { };
  folder = config.users.users.${account}.home;

  # The realm's node as a file named pi-host. A Mac's ps names a process by
  # its program's file, not by what it's called, and mycelium tells a
  # process as an agent's by a forebear named pi or pi-host. launchd is the
  # host's parent and no pi runs above it, so under node's own name every
  # command a hosted conversation runs would be told as the person's. A
  # copy, since a link would be named for the file it leads to, and of
  # node's own program, since a script would run node under node's name.
  named =
    host:
    # Kept in Nix because it is a copy and one test of the realm's node,
    # whose path and message only Nix knows.
    pkgs.runCommandLocal "pi-host" { } ''
      if [ "$(head -c 2 ${host.node})" = "#!" ]; then
        echo "jitsusama.pi.hosts: ${host.node} is a script; a Mac's host needs node's own program, which it runs as pi-host." >&2
        exit 1
      fi
      mkdir -p $out/bin
      cp ${host.node} $out/bin/pi-host
    '';

  # Where a host's start is, a path no switch changes. A restart parks every
  # conversation the host holds and ends the turns they are taking, and
  # nix-darwin restarts a daemon whose plist changed, so the plist names
  # only this and the host's program, never a store path. A switch changes
  # what this link leads to and leaves a running host be, as Linux's
  # keep-old does; a new pi or a new harness reaches it at its next start.
  startAt = realm: kind: "/etc/pi-host/${realm}-${kind}";

  logOf = realm: kind: "${folder}/Library/Logs/pi-host-${realm}-${kind}.log";

  # A daemon's environment is launchd's alone, so the account's home and
  # language, which its login's shell sets, are put first, for a realm's own
  # to replace as on Linux, so commands read and print text as they do in a
  # terminal. Only LANG: the shell's LC_ALL would outrank a realm's LANG,
  # which Linux's hosts never get.
  accountsOwn = {
    HOME = folder;
    USER = account;
  }
  // lib.optionalAttrs (home.programs.zsh.sessionVariables ? LANG) {
    LANG = toString home.programs.zsh.sessionVariables.LANG;
  };

  ofTheAccount = lib.replaceStrings [ "$HOME" "$USER" ] [ folder account ];

  environmentOf =
    realm: host:
    launch.environment home realm host true accountsOwn (launch.hostOwn (launch.restartWaitsOn home))
    // {
      # The folders a login's shell finds commands in, in its order: the
      # realm's own, Homebrew's, the system's, then the account's session's,
      # since a command found in a terminal but not by a hosted
      # conversation is hard to trace to the host.
      PATH = lib.concatStringsSep ":" (
        host.path
        ++ lib.optionals config.homebrew.enable [
          "${config.homebrew.prefix}/bin"
          "${config.homebrew.prefix}/sbin"
        ]
        ++ [ (ofTheAccount config.environment.systemPath) ]
        ++ map ofTheAccount (home.home.sessionPath or [ ])
      );
    };

  # Run as the account, so its log is opened as the account and not as
  # root, who would follow a link left there to any file. It keeps the
  # times of its last three starts beside the log, and a fourth within ten
  # minutes waits ten minutes, so a harness that fails as it starts costs
  # a start every few minutes rather than six a minute. Until the harness
  # ships the program there is nothing to run, so it looks again ten
  # minutes on. The socket goes in the account's own cache folder, named
  # AGENTS_RUNTIME_DIR, where the harness and mycelium look first on a Mac,
  # and the scratch of the host's conversations in its temporary folder;
  # each is the account's with mode 700, unlike /tmp. The socket stays out
  # of the temporary folder because, by confstr(3), the system may remove
  # what sits there unaccessed for three days, and a host's socket lives as
  # long as the host; the cache folder it never cleans by itself, only a
  # safe boot empties it, which ends every host anyway. A host
  # already running keeps its socket where it was until it starts again.
  # The XDG_RUNTIME_DIR launchd gives is taken away, since disk-guard reads
  # it for a folder of its own; a realm naming one gets what it named.
  # Neither path is known until the account asks the system, so sh asks,
  # checks each is a whole path to a folder with no . or .. in it, not a
  # link, the account's with mode 700, and then becomes node, the process
  # launchd signals, through env, which takes any name a realm gives and
  # drops every variable that would run code as node starts
  # (host-launch.nix's loadsCode), as Linux's unit does.
  startOf = realm: host: kind: ''
    log=${lib.escapeShellArg (logOf realm kind)}
    /bin/mkdir -p "''${log%/*}"
    if [ -f "$log" ] && [ "$(/usr/bin/stat -f %z "$log")" -gt 10485760 ]; then
      /bin/mv -f "$log" "$log.old"
    fi
    exec >>"$log" 2>&1
    starts="$log.starts"
    now=$(/bin/date +%s)
    if [ -n "$(/usr/bin/sed -n 3p "$starts" 2>/dev/null)" ] &&
      [ "$(/usr/bin/sed -n 1p "$starts")" -gt "$((now - 600))" ]; then
      echo "pi-host: started three times in ten minutes; waiting ten minutes to start again; removing $starts ends the wait"
      waited=0
      while [ "$waited" -lt 600 ] && [ -e "$starts" ]; do
        /bin/sleep 10
        waited=$((waited + 10))
      done
      now=$(/bin/date +%s)
    fi
    { /usr/bin/tail -n 2 "$starts" 2>/dev/null; echo "$now"; } >"$starts.new"
    /bin/mv -f "$starts.new" "$starts"
    if [ ! -e ${lib.escapeShellArg (launch.entry home)} ]; then
      echo "pi-host: the harness has no host here yet; looking again in ten minutes"
      /bin/sleep 600
      exit 1
    fi
    temporary=$(/usr/bin/getconf DARWIN_USER_TEMP_DIR)
    cache=$(/usr/bin/getconf DARWIN_USER_CACHE_DIR)
    for folder in "$temporary" "$cache"; do
      bare=$folder
      while [ "''${bare%/}" != "$bare" ]; do bare=''${bare%/}; done
      dotted=
      case "$bare/" in */./* | */../*) dotted=yes ;; esac
      if [ -n "$dotted" ] || [ "''${folder#/}" = "$folder" ] || [ -L "$bare" ] ||
        [ ! -d "$folder" ] ||
        [ "$(/usr/bin/stat -f '%u %Lp' "$folder")" != "$(/usr/bin/id -u) 700" ]; then
        echo "pi-host: the account's own temporary or cache folder wasn't found, or isn't its alone: '$folder'"
        exit 1
      fi
    done
    exec /usr/bin/env ${
      lib.concatMapStringsSep " " (name: "-u ${name}") (launch.loadsCode ++ [ "XDG_RUNTIME_DIR" ])
    } \
    ${
      lib.concatMapStrings (word: "  ${word} \\\n") (
        lib.mapAttrsToList (name: value: lib.escapeShellArg "${name}=${value}") (environmentOf realm host)
        ++ [
          ''AGENTS_RUNTIME_DIR="$cache"''
          ''TMPDIR="$temporary"''
        ]
      )
    }  ${
      lib.escapeShellArgs (
        [ "${named host}/bin/pi-host" ] ++ launch.words home host host.heapMb [ ] [ kind ]
      )
    }
  '';

  hostOf = realm: kind: {
    name = "pi-host-${realm}-${kind}";
    value.serviceConfig = {
      Label = "org.jitsusama.pi-host-${realm}-${kind}";
      UserName = account;
      # At boot the start, a link into the store, isn't there until the
      # store's volume is mounted, and what fails before the start opens its
      # log goes nowhere, so sh waits for the store first, as nix-darwin's
      # own daemons do.
      ProgramArguments = [
        "/bin/sh"
        "-c"
        "/bin/wait4path /nix/store && exec /bin/sh ${lib.escapeShellArg (startAt realm kind)}"
      ];
      RunAtLoad = true;
      # The host parks what it holds when told to stop and then exits
      # cleanly, so only a failure starts it again, as on-failure does on
      # Linux; launchd starts nothing within ten seconds of its last start.
      KeepAlive.SuccessfulExit = false;
      ThrottleInterval = 10;
      # The time launchd gives a stop before it kills, which the host parks
      # within.
      ExitTimeOut = launch.stopSeconds;
      # launchd's own soft limit is 256 open files, and a host holds a
      # socket and files for each of dozens of conversations. Only the soft
      # limit rises: launchd's hard one is unlimited, and setting it would
      # lower it for everything the host starts.
      SoftResourceLimits.NumberOfFiles = 8192;
    };
  };

  pairs = lib.concatLists (
    lib.mapAttrsToList (
      realm: host:
      map (kind: {
        inherit realm host kind;
      }) (lib.attrNames launch.kinds)
    ) hosts
  );
in
{
  config = {
    # env hands a realm's environment to the host's node, which System
    # Integrity Protection doesn't guard, so a Mac's loader would read a
    # DYLD_ variable there and load a library into the host, as Linux's
    # refuses LD_PRELOAD for.
    assertions =
      lib.mapAttrsToList (
        realm: host:
        let
          given = lib.filter (lib.hasPrefix "DYLD_") (lib.attrNames host.environment);
        in
        {
          assertion = given == [ ];
          message = "jitsusama.pi.hosts.${realm}.environment gives ${lib.concatStringsSep ", " given}, which a Mac's loader reads to load a library into the pi host. Drop it; a command that needs one sets it itself.";
        }
      ) hosts
      ++ lib.mapAttrsToList (name: user: {
        assertion = name == account || (user.jitsusama.pi.hosts or { }) == { };
        message =
          if account == null then
            "home-manager.users.${name}.jitsusama.pi.hosts names a realm, but a Mac's pi hosts run only as its primary user, and system.primaryUser names none."
          else
            "home-manager.users.${name}.jitsusama.pi.hosts names a realm, but a Mac's pi hosts run only as its primary user, ${account}, whose launchd daemons they are.";
      }) (config.home-manager.users or { });

    launchd.daemons = lib.listToAttrs (map (pair: hostOf pair.realm pair.kind) pairs);

    environment.etc = lib.listToAttrs (
      map (pair: {
        name = lib.removePrefix "/etc/" (startAt pair.realm pair.kind);
        value.text = startOf pair.realm pair.host pair.kind;
      }) pairs
    );
  };
}
