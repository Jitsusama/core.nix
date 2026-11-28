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
  # queue, and one whose name can't be looked up said as that, a connection
  # that won't open not
  # asked for on every request, a failure that ends as the origin comes
  # back, an origin it can't trust sending pi direct, a request answered
  # beside an answer longer than the idle time, the relay leaving once idle
  # even while refusing other hosts, held open by a kept connection or
  # knocked on as it goes, a relay stopped finishing each answer it streams
  # and sending what comes meanwhile direct past the idle leave's ten
  # seconds, then giving those a few seconds, even one pi says nothing on,
  # cutting its answers when its drain time runs out or at a second stop,
  # ending what it sends direct when stopped as it leaves idle, at once if
  # stopped again, and saying what it waited on and its counts, its cuts
  # apart from pi's cancels, a drain of
  # none and the flags it no longer takes refused, a socket the account's
  # alone, never taken from a relay answering it nor from a file or a link
  # that isn't a socket, and nothing of a header or body logged, a failure
  # included.
  model-relay-carries-requests-whole = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in model-relay, its tests included.
  model-relay-passes-clippy = passesClippy "model-relay-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A workstation that names no origin has no relay and tells pi of none;
  # one that names an origin listens where only the account may ask, in the
  # agents' lane, forwards to that origin alone, stops before systemd
  # would kill it, keeps more connections than its default and no more
  # than it takes, and tells every pi the folder its sockets are in. The
  # lend's socket beside pi's is the account's alone too, handed the same
  # relay under the name its source tells it apart by, and so is the one
  # it's asked to hand over on. A switch reloads it, and the reload runs
  # the same build and flags as its start, asking it to hand over.
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
      lends = named.systemd.user.sockets.model-relay-lend.Socket;
      controls = named.systemd.user.sockets.model-relay-control.Socket;
      # The names the relay finds the lend's and the control's sockets by,
      # read from its source so neither side moves alone.
      nameOf =
        constant:
        lib.pipe ./src/main.rs [
          builtins.readFile
          (lib.splitString "\n")
          (lib.findFirst (lib.hasPrefix "const ${constant}:") "")
          (lib.match ".*= \"([a-z]+)\";")
        ];
      lendName = nameOf "LEND";
      controlName = nameOf "CONTROL";
      sockets = [
        "model-relay-agents.socket"
        "model-relay-lend.socket"
        "model-relay-control.socket"
      ];
      # The reload is the start's build and flags, asking on the control
      # socket to be handed over to.
      words = command: lib.splitString " " (lib.concatStringsSep " " (lib.toList command));
      started = words service.ExecStart;
      reloaded = words (service.ExecReload or "");
      reloadsAsAHandOver =
        lib.length reloaded == lib.length started + 2
        && lib.head reloaded == lib.head started
        && lib.elemAt reloaded 1 == lib.escapeShellArg "--hand-over"
        && lib.elemAt reloaded 2 == lib.escapeShellArg controls.ListenStream
        && lib.drop 3 reloaded == lib.tail started;
      unit = named.systemd.user.services.model-relay-agents.Unit;
      service = named.systemd.user.services.model-relay-agents.Service;
      drain = lib.match ".* --drain-within ([0-9]+) --origin https://models\\.example" (
        lib.concatStringsSep " " (lib.toList service.ExecStart)
      );
      # The seconds a stopped relay gives those it sends direct once its
      # answers end, read from its source so neither side moves alone.
      goodbyes = lib.pipe ./src/lib.rs [
        builtins.readFile
        (lib.splitString "\n")
        (lib.findFirst (lib.hasPrefix "const SAYS_GO_DIRECT_WITHIN:") "")
        (lib.match ".*from_secs\\(([0-9]+)\\);")
      ];
      # The connections it's told to keep, against its source's default and
      # the most it takes: a count past that fails the relay at its start,
      # and pi then goes direct for every request. Both are read from the
      # source, so neither side moves alone.
      kept = lib.match ".* --connections ([0-9]+) .*" (
        lib.concatStringsSep " " (lib.toList service.ExecStart)
      );
      sourced =
        name:
        lib.pipe ./src/main.rs [
          builtins.readFile
          (lib.splitString "\n")
          (lib.findFirst (lib.hasPrefix "const ${name}:") "")
          (lib.match ".*= ([0-9]+);")
        ];
      default = sourced "CONNECTIONS";
      most = sourced "MOST_CONNECTIONS";
      # Left past those for the relay to end its connections and say its
      # counts.
      spare = 2;
      # Read only once both are known to be there.
      needs = lib.toInt (lib.head drain) + lib.toInt (lib.head goodbyes) + spare;
    in
    holds "model-relay-runs-in-background-slice"
      "model-relay is installed where no origin is named, lets other accounts ask, leaves its socket behind, hands the relay no lend's or control's socket by its source's name, runs without its socket or outside background.slice, forwards somewhere other than the origin named, sees more of the machine than its origin needs, is restarted by a switch, isn't reloaded as a hand-over to the build and flags it starts with, can't tell systemd its new main process, waits on its answers once stopped past what its stop timeout leaves for its goodbyes, keeps no more connections than its source's default or more than it takes, or doesn't tell pi where it is."
      (
        !(bare.systemd.user.sockets ? model-relay-agents)
        && !(bare.systemd.user.sockets ? model-relay-lend)
        && !(bare.systemd.user.sockets ? model-relay-control)
        && !(bare.systemd.user.sessionVariables ? PI_PROXY_RELAY)
        && listens.ListenStream == "%t/model-relay/agents.sock"
        && listens.SocketMode == "0600"
        && listens.DirectoryMode == "0700"
        && listens.RemoveOnStop
        && listens.FileDescriptorName == "agents"
        && lends.ListenStream == "%t/model-relay/lend.sock"
        && lends.SocketMode == "0600"
        && lends.DirectoryMode == "0700"
        && lends.RemoveOnStop
        && lendName != null
        && lends.FileDescriptorName == lib.head lendName
        && lends.Service == "model-relay-agents.service"
        && controls.ListenStream == "%t/model-relay/control.sock"
        && controls.SocketMode == "0600"
        && controls.DirectoryMode == "0700"
        && controls.RemoveOnStop
        && controlName != null
        && controls.FileDescriptorName == lib.head controlName
        && controls.Service == "model-relay-agents.service"
        && unit.Requires == sockets
        && service.Sockets == sockets
        # A switch hands it over to the new build rather than leaving it
        # be or cutting what it streams.
        && unit.X-SwitchMethod == "reload"
        && reloadsAsAHandOver
        && service.NotifyAccess or null == "main"
        && service.TimeoutStartSec or 0 >= 35
        && service.Slice == "background.slice"
        # Its drain, then its goodbyes, end inside the stop timeout, so it
        # leaves on its own and says its counts.
        && drain != null
        && goodbyes != null
        && service.TimeoutStopSec or 0 >= needs
        # It keeps more connections than its default, but no more than
        # it takes.
        && kept != null
        && default != null
        && most != null
        && lib.toInt (lib.head kept) > lib.toInt (lib.head default)
        && lib.toInt (lib.head kept) <= lib.toInt (lib.head most)
        # It holds every agent's key while it carries a request, so it
        # sees no more of the machine than reaching the origin needs.
        && service.ProtectSystem or null == "strict"
        # Of the runtime folder it sees only systemd's notify socket and
        # its control socket.
        && service.ProtectHome or null == "tmpfs"
        &&
          service.BindPaths or null == [
            "%t/systemd/notify"
            controls.ListenStream
          ]
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
