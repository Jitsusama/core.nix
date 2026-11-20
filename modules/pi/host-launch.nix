# What starts a realm's pi host, the same under Linux's systemd (host.nix) and
# a Mac's launchd (darwin.nix): the program node runs, the words node is
# given before it, the variables its start clears, and the environment the
# host and its conversations' commands share. Each system adds its own around
# it: how PATH is found, the agents' SSH agent, and the runtime folder its
# socket goes in.
{ lib }:
rec {
  # How long the system gives a host to stop at a restart or a switch, in
  # seconds. The host is told the same number (PI_HOST_STOP_USEC), which
  # parks within three quarters of it, so its own limit comes first there.
  stopSeconds = 150;

  kinds = {
    attended = "the pi conversations Joel watches";
    background = "the pi conversations that run in the background";
  };

  # The harness's host program, in the version pi-packages calls current, in
  # the account `home`. node follows the link as it loads it, so a running
  # host keeps the version it started with through an update, and the next
  # start takes the new one.
  entry =
    home:
    "${home.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-host.ts";

  # What would run code as the host as its node starts, so a host's start
  # clears each and a realm giving one is refused: NODE_OPTIONS, whose
  # --require or --import runs a script and whose --inspect opens the
  # inspector; NODE_PATH, where a require that finds nothing beside it looks,
  # as a library's optional require does; NODE_COMPILE_CACHE, whose folder
  # wins over the host's own and whose compiled code node runs as its
  # modules' (host/service/compile-cache.ts); the loader's LD_PRELOAD, LD_AUDIT
  # and LD_LIBRARY_PATH, which load a library; and OpenSSL's OPENSSL_CONF and
  # OPENSSL_MODULES, whose configuration loads a module. mycelium keeps the
  # same list in its src/node/asker.rs, once its lock here carries the change
  # that proves a hosted session by its spawner: it then takes no host's word
  # whose environment gives any of them, but for a blank NODE_OPTIONS. The two
  # lists are kept alike by hand: nothing here reads mycelium's yet. These are
  # node's, the loader's and OpenSSL's doors only. pi's own variables, such
  # as PI_CODING_AGENT_DIR, which moves the extensions it loads, are the
  # account's doors, like HOME: a process of the account can as well write
  # the host's own code where pi-packages keeps it, or a module in node's
  # home folders (~/.node_modules), which no variable names. A Mac's own
  # loader variables, DYLD_*, are dropped by macOS as its start's sh and
  # env run, while System Integrity Protection is on, but env then hands
  # node a realm's own, so a Mac refuses a realm giving one (darwin.nix).
  loadsCode = [
    "NODE_OPTIONS"
    "NODE_PATH"
    "NODE_COMPILE_CACHE"
    "LD_PRELOAD"
    "LD_AUDIT"
    "LD_LIBRARY_PATH"
    "OPENSSL_CONF"
    "OPENSSL_MODULES"
  ];

  # What a host's own restart waits out in the account `home` (the harness's
  # host/service/quiet.ts), as JSON: each probe's words and its command,
  # which exits 0 while its reason holds and 1 while it doesn't.
  restartWaitsOn =
    home:
    builtins.toJSON (
      map (probe: {
        "while" = probe.why;
        run = probe.command;
      }) home.jitsusama.pi.hostRestartWaitsWhile
    );

  # The words after node, for a process with a heap of `heapMb`: the
  # harness's program, then `rest`, such as a host's kind.
  words =
    home: host: heapMb: rest:
    [
      # Any process of the account may send SIGUSR1, which would open node's
      # inspector and let it run code as the host. mycelium takes the host's
      # word for which conversation each pi-spawner serves only where this
      # is the first option on the host's command line and no option there
      # opens the inspector; once its lock here carries its list, also only
      # where the host's environment gives none of loadsCode, which the
      # start clears. It refuses the word of a host started otherwise, and
      # logs why.
      "--disable-sigusr1"
    ]
    # After the flag above, which must come first; asyncContextFrame's
    # description says why a realm sets it.
    ++ lib.optional host.asyncContextFrame "--experimental-async-context-frame"
    ++ [
      # The harness's package.json names no module type, on purpose.
      "--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"
      "--max-old-space-size=${toString heapMb}"
      (entry home)
    ]
    ++ rest;

  # The environment of a process of `realm`'s host in the account `home`:
  # what the system adds first (`before`), which the realm's own may
  # replace, and what it adds last (`own`), which nothing does. `many` is
  # whether the process holds many conversations, as a host does, and so
  # wants the pools sized for them; a conversation's own process keeps the
  # defaults a plain pi has.
  environment =
    home: realm: host: many: before: own:
    let
      homeRealm = lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null home;
    in
    lib.optionalAttrs many {
      # The host reads its conversations' mesh folders off its loop, on
      # libuv's pool, which is four threads by default. On 16 November a
      # stalled disk held those reads for up to 17 s, and four stalled reads
      # would leave every other conversation's reads, lookups and hashing
      # waiting behind them. libuv sizes the pool when it first starts,
      # before any of the host's code runs, so it is set here. A machine's
      # own value still wins.
      UV_THREADPOOL_SIZE = "16";
      # pi gives a process two codemode workers, which suits one conversation
      # but is shared here by every conversation the host holds. After the
      # reset of 17 November the background host's forty conversations had
      # one cell's six-minute script and one other holding both, and every
      # other script failed after waiting a minute. A worker starts only for
      # a script, so a bigger limit costs nothing while idle. pi reads it at
      # each script.
      PI_CODEMODE_MAX_WORKERS = "16";
    }
    // before
    // host.environment
    // {
      MYCELIUM_REALM = realm;
      # pi reads its own files from here, and a subagent a hosted
      # conversation starts runs this pi too.
      PI_PACKAGE_DIR = host.pi;
      # The host checks it against its pi as it starts.
      PI_HOST_PROOF = "${host.proof}";
    }
    // lib.optionalAttrs (homeRealm != null) { PI_HOME_REALM = homeRealm; }
    // own;

  # What a host itself adds last in the account `home`, with its restart's
  # waits as `waits` (restartWaitsOn's JSON, as the system needs it written).
  hostOwn = waits: {
    # The stop limit, which the host parks within.
    PI_HOST_STOP_USEC = toString (stopSeconds * 1000000);
    # A host restarts itself onto a new pi or harness once it's installed,
    # or for its heap. At 16:38 on 19 November the attended host did so
    # mid-call as its install moved, and every window on it went with it,
    # so it waits until no call is on and nothing a machine adds holds.
    PI_HOST_RESTART_WAITS_ON = waits;
  };
}
