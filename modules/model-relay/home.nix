# model-relay carries the agents' model requests to their one origin over a
# few long-lived HTTP/2 connections, rather than one connection for each
# request in flight (src/lib.rs says why: on a ten-megabit uplink, ninety
# flows each lose about a segment in five).
#
# The origin is a machine's to name, in jitsusama.model-relay.origin; until
# one is named nothing is installed, and pi reaches its origin directly.
#
# pi finds the relay through PI_PROXY_RELAY, a folder in the runtime folder
# holding agents.sock. A pi sends a request there only while no turn a
# person typed is under way in it, which pi says in $XDG_RUNTIME_DIR/pi/typed
# for the shaper: a typed turn goes directly, so the shaper still lifts it
# ahead of the agents, which it couldn't once the request was among the
# relay's. A pi that finds no socket there, or one nothing answers, or a 421
# from a relay still naming an origin the machine no longer does, connects
# directly, as before. Those rules are pi's own, held where its fetch is
# wrapped for the proxy; nothing here can make a pi keep them.
#
# The relay trusts the public roots built into it, so an origin whose
# certificate chains to a private root isn't reached: each request is
# answered as a 421, which sends pi direct, and the journal says why. A
# request the origin resets before its answer begins, saying its stream was
# refused or a protocol error, goes once more, on another connection, and
# the one that reset it takes no more; so does one left out of a goodbye
# saying nothing was wrong, as a drain before a deploy or a connection's
# rotation is, or a protocol error, as that promises the origin did nothing
# with it. Reset or
# left out again the same way, lost on its path when sent again, or with no
# other connection to go on, it's the relay's own 421, so pi sends it direct
# at once. Reset or said goodbye to saying anything else, first or again,
# it's a 502, for pi's own retry. It never stands down from the origin as a
# whole: each request it can't carry goes direct alone, so a failing burst
# never sends every agent past it at once.
#
# The relay runs in background.slice, which the shaper sends in the agents'
# lane, and its connections are the account's own, so they ride the work VPN
# as pi's did. It holds no credential: each request's own goes as it came.
# A fence keeps the runtime folder from what it runs, so a fenced command
# can't reach the socket, and the folder mustn't be bound into one: it is a
# way to the origin with whatever key the asker holds. A unit of the account
# kept off the network with PrivateNetwork still sees the runtime folder,
# so it can reach the origin through the socket, though with no key of its
# own the origin answers it nothing.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.model-relay;
  package = pkgs.callPackage ./package.nix { };
  folder = "model-relay";
  # How long a stopped relay waits on the answers it streams before it
  # cuts them, in seconds. The stop timeout gives it this and ten more:
  # five for those it sends direct to finish their 421 (its source's
  # SAYS_GO_DIRECT_WITHIN, which the check reads), five to spare, so
  # it leaves on its own and says its counts rather than being killed.
  drainsWithin = 80;
  # How many connections it keeps to the origin: the most it takes, not
  # its own default of four. Over the work VPN, where a lost packet costs
  # a long round trip, one connection carries only so much at once, and
  # bodies sharing it arrive late enough for the origin to reset them.
  # Measured on 26 November with twenty requests of 50 KiB each: over
  # one connection they took 10 to 17 s and some were reset, over four
  # 7 to 9 s, and over ten 3 to 4 s with none reset.
  connections = 16;
in
{
  options.jitsusama.model-relay.origin = lib.mkOption {
    # A host and maybe a port alone, as the relay itself reads an origin;
    # none of systemd's % or $ expansions can be in one.
    type = lib.types.nullOr (lib.types.strMatching "https://[A-Za-z0-9.-]+(:[0-9]{1,5})?/?");
    default = null;
    example = "https://models.example";
    description = ''
      The https origin pi's model requests go to, which the relay forwards
      to and nothing else: a request naming any other host is refused.
    '';
  };

  config = lib.mkIf (cfg.origin != null && pkgs.stdenv.hostPlatform.isLinux) {
    # Every unit and every pi takes this from the user manager, as it
    # starts; a pi already running keeps reaching its origin directly.
    systemd.user.sessionVariables.PI_PROXY_RELAY = "\${XDG_RUNTIME_DIR}/${folder}";

    systemd.user.sockets.model-relay-agents = {
      Unit.Description = "Where the agents' model requests are relayed from";
      # Only the account may ask.
      Socket = {
        ListenStream = "%t/${folder}/agents.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        # A socket left with nothing behind it would be asked and refuse;
        # one removed is passed by, and pi connects directly.
        RemoveOnStop = true;
      };
      Install.WantedBy = [ "sockets.target" ];
    };

    systemd.user.services.model-relay-agents = {
      Unit = {
        Description = "Relays the agents' model requests over a few shared connections";
        Requires = [ "model-relay-agents.socket" ];
        After = [ "model-relay-agents.socket" ];
        # A restart lets each answer the relay is streaming, each a turn
        # some agent is taking, end first, sending every new request direct
        # meanwhile, and cuts one still going after drainsWithin. A second
        # SIGTERM cuts them at once: systemctl --user kill model-relay-agents
        # sends one, while a second stop or restart sends none, joining the
        # stop already going. A shutdown, or a logout where the account
        # doesn't linger, gives it only the user manager's own stop timeout,
        # 5 s under this repository's systemd module, so a busy relay is
        # killed mid-drain, with what it cuts and its last counts unsaid. A
        # lingering account's logout leaves it running. A restart sends a busy machine's
        # agents past the relay for as long as its longest answer, so a
        # switch leaves it be. It leaves on its own once it has forwarded
        # nothing for two minutes,
        # and its socket starts the new build and origin at the next
        # request. Agents busy enough never to leave it two quiet minutes
        # keep the old one until they do, or until it's restarted by hand
        # with systemctl --user restart model-relay-agents.
        X-SwitchMethod = "keep-old";
      };
      Service = {
        ExecStart = lib.escapeShellArgs [
          (lib.getExe package)
          "--connections"
          (toString connections)
          "--drain-within"
          (toString drainsWithin)
          "--origin"
          cfg.origin
        ];
        TimeoutStopSec = drainsWithin + 10;
        # The agents' lane, never app.slice, where it would ride Joel's.
        Slice = "background.slice";
        Restart = "on-failure";
        # It holds every agent's key while it carries a request, so it sees
        # no more of the machine than reaching the origin needs: no home,
        # no shared /tmp, nothing written, no capability, no namespace of
        # its own making. A user manager takes these by putting it in a
        # user namespace of its own, so a kernel refusing unprivileged ones
        # fails the unit, and its socket, after a few tries, refuses pi, who
        # then goes direct. Its socket comes from systemd, so the runtime
        # folder it hides isn't needed.
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        CapabilityBoundingSet = "";
        RestrictNamespaces = true;
        SystemCallFilter = "@system-service";
        UMask = "0077";
        RestrictRealtime = true;
        NoNewPrivileges = true;
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];
        SystemCallArchitectures = "native";
      };
    };
  };
}
