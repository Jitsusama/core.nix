# The checks of pi-agent-proxy, the SSH agent pi's fenced commands reach,
# beside pi's own in ../checks.nix.
{
  account,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # The fence's SSH agent runs only where it's given hosts, started by the
  # first ssh that asks, on a socket only the account can reach, told of
  # those hosts and of each agent behind it; a Mac gets none.
  pi-agent-proxy-runs-only-with-machines =
    let
      key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
      home = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.sharedModules = [
            {
              jitsusama.pi.agentProxy = {
                machines = [ key ];
                upstreams = [ "%t/agents.sock" ];
              };
            }
          ];
        }
      ]);
      plain = account workstation;
      mac = account macWorkstation;
      socket = home.systemd.user.sockets.pi-agent-proxy.Socket;
      start = lib.head (lib.toList home.systemd.user.services.pi-agent-proxy.Service.ExecStart);
    in
    holds "pi-agent-proxy-runs-only-with-machines"
      "pi's SSH agent proxy runs with no hosts or on a Mac, listens elsewhere than %t/pi-agent.sock or to more than the account, or isn't told of its hosts or the agents behind it."
      (
        lib.head (lib.toList socket.ListenStream) == "%t/pi-agent.sock"
        && lib.head (lib.toList socket.SocketMode) == "0600"
        && lib.hasInfix "--upstream" start
        && lib.hasInfix "%t/agents.sock" start
        && lib.hasInfix "--hosts" start
        && lib.hasInfix "-pi-agent-proxy-hosts" start
        && !(plain.systemd.user.services ? pi-agent-proxy)
        && !(plain.systemd.user.sockets ? pi-agent-proxy)
        && !(mac.systemd.user.services ? pi-agent-proxy)
      );

  # pi-agent-proxy builds with its tests run (tests/agent_proxy.rs):
  # the keys behind it listed, a login signed once its allowed host signed
  # the session, and nothing else reaching the agent behind it: no session,
  # another host, a forged binding, a commit, another session or key, a
  # forwarded agent, or adding, removing and locking keys; and no more than
  # 64 clients answered at once, a place given back as each goes.
  pi-agent-proxy-signs-only-logins = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-agent-proxy, its tests included.
  pi-agent-proxy-passes-clippy = passesClippy "pi-agent-proxy-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
