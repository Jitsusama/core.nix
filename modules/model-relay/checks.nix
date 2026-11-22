# The checks of model-relay, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
onLinux {
  # model-relay's tests run as it builds, against an HTTP/2 origin over TLS
  # of their own: a request forwarded whole, a TE of trailers passed as h2
  # sends it, another host refused, many
  # requests at once on a few connections, an answer streamed as it comes, a
  # connection the origin lets go opened again, a request the origin reset
  # before its answer, or left out of a goodbye saying nothing was wrong or
  # a protocol error, sent once more on another connection, one already open, even beside one
  # kept, or one opened for a burst of them together, a single attempt heard
  # by all when it fails, and sent direct with the relay's own 421 when
  # reset or left out again, or lost, when its connection was kept with no
  # other open or when none would open, but said as a 502 when a reset,
  # first or again, said to calm down, with its connection retired, a body too large or left
  # unfinished going nowhere, another origin's 421 said once the body is
  # read, even a body too large, a body pi left counted as its cancel, never
  # a 421, one past its deadline told as late, the counts said as it leaves,
  # answers broken off among them, by a fallen connection or the origin,
  # the requests pi gave up on counted on each connection and no answer read whole or broken off, a
  # request the origin reset told from one a fallen
  # connection lost, an origin out of reach said as a 502 or a 504 without a
  # queue, a connection that won't open not
  # asked for on every request, a failure that ends as the origin comes
  # back, an origin it can't trust sending pi direct, a request answered
  # beside an answer longer than the idle time, the relay leaving once idle
  # even while refusing other hosts, held open by a kept connection or
  # knocked on as it goes, a relay stopped finishing each answer it streams
  # and sending what comes meanwhile direct past the idle leave's ten
  # seconds, then giving those a few seconds, even one pi says nothing on,
  # cutting its answers after its drain time or at a second stop and saying
  # what it waited on and its counts, a socket the account's alone, never
  # taken from a relay answering it nor from a file or a link that isn't a socket, and
  # nothing of a header or body logged, a failure included.
  model-relay-carries-requests-whole = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in model-relay, its tests included.
  model-relay-passes-clippy = passesClippy "model-relay-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A workstation that names no origin has no relay and tells pi of none;
  # one that names an origin listens where only the account may ask, in the
  # agents' lane, forwards to that origin alone, stops before systemd
  # would kill it, and tells every pi the folder its sockets are in.
  model-relay-runs-in-background-slice =
    let
      named = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.sharedModules = [ { jitsusama.model-relay.origin = "https://models.example"; } ];
        }
      ]);
      bare = account workstation;
      listens = named.systemd.user.sockets.model-relay-agents.Socket;
      unit = named.systemd.user.services.model-relay-agents.Unit;
      service = named.systemd.user.services.model-relay-agents.Service;
    in
    holds "model-relay-runs-in-background-slice"
      "model-relay is installed where no origin is named, lets other accounts ask, leaves its socket behind, runs without its socket or outside background.slice, forwards somewhere other than the origin named, sees more of the machine than its origin needs, is restarted by a switch, waits on its answers once stopped past what its stop timeout leaves for its goodbyes, or doesn't tell pi where it is."
      (
        !(bare.systemd.user.sockets ? model-relay-agents)
        && !(bare.systemd.user.sessionVariables ? PI_PROXY_RELAY)
        && listens.ListenStream == "%t/model-relay/agents.sock"
        && listens.SocketMode == "0600"
        && listens.DirectoryMode == "0700"
        && listens.RemoveOnStop
        && unit.Requires == [ "model-relay-agents.socket" ]
        && unit.X-SwitchMethod == "keep-old"
        && service.Slice == "background.slice"
        && lib.hasSuffix " --drain-within 80 --origin https://models.example" (
          lib.concatStringsSep " " (lib.toList service.ExecStart)
        )
        # Its drain, then five seconds of goodbyes, end inside the stop
        # timeout, so it leaves on its own and says its counts.
        && service.TimeoutStopSec or null == 90
        # It holds every agent's key while it carries a request, so it
        # sees no more of the machine than reaching the origin needs.
        && service.ProtectSystem or null == "strict"
        && service.ProtectHome or null == true
        && service.PrivateTmp or null == true
        && service.CapabilityBoundingSet or null == ""
        && service.RestrictNamespaces or null == true
        && service.SystemCallFilter or null == "@system-service"
        && service.UMask or null == "0077"
        && service.RestrictRealtime or null == true
        && service.NoNewPrivileges or null == true
        && service.LockPersonality or null == true
        && service.MemoryDenyWriteExecute or null == true
        &&
          service.RestrictAddressFamilies or null == [
            "AF_UNIX"
            "AF_INET"
            "AF_INET6"
          ]
        && service.SystemCallArchitectures or null == "native"
        && named.systemd.user.sessionVariables.PI_PROXY_RELAY == "\${XDG_RUNTIME_DIR}/model-relay"
      );
}
