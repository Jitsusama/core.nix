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
  # of their own: a request forwarded whole, another host refused, many
  # requests at once on a few connections, an answer streamed as it comes, a
  # connection the origin lets go opened again, an origin out of reach said
  # as a 502 or a 504 without a queue, a connection that won't open not
  # asked for on every request, a failure that ends as the origin comes
  # back, the relay leaving once idle but never mid-answer, even while
  # refusing other hosts, a socket the account's alone and never taken from
  # a relay answering it, and nothing of a header or body logged, a failure
  # included.
  model-relay-carries-requests-whole = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in model-relay, its tests included.
  model-relay-passes-clippy = passesClippy "model-relay-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A workstation that names no origin has no relay and tells pi of none;
  # one that names an origin listens where only the account may ask, in the
  # agents' lane, forwards to that origin alone, and tells every pi the
  # folder its sockets are in.
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
      "model-relay is installed where no origin is named, lets other accounts ask, leaves its socket behind, runs without its socket or outside background.slice, forwards somewhere other than the origin named, is restarted by a switch, or doesn't tell pi where it is."
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
        && lib.hasSuffix " --origin https://models.example" service.ExecStart
        && named.systemd.user.sessionVariables.PI_PROXY_RELAY == "\${XDG_RUNTIME_DIR}/model-relay"
      );
}
