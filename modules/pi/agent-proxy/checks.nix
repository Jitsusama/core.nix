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
  # The fence's SSH agent runs only where it's given hosts or the
  # builders' key, started by the first ssh that asks, on a socket in a
  # folder only the account can reach, linked from where fenced commands
  # found it before, told of those hosts and of each agent behind it, or
  # of the builders' key and root's file of the builders lent; a Mac
  # gets none.
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
      lending = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.sharedModules = [
            {
              jitsusama.pi.agentProxy = {
                builderKey = "/home/someone/.local/state/nix-offload/store_ed25519";
                lentFiles = [ "/run/nix-offload-lines/lent" ];
                upstreams = [ ];
              };
            }
          ];
        }
      ]);
      lends = lib.head (lib.toList lending.systemd.user.services.pi-agent-proxy.Service.ExecStart);
      plain = account workstation;
      mac = account macWorkstation;
      socket = home.systemd.user.sockets.pi-agent-proxy.Socket;
      start = lib.head (lib.toList home.systemd.user.services.pi-agent-proxy.Service.ExecStart);
    in
    holds "pi-agent-proxy-runs-only-with-machines"
      "pi's SSH agent proxy runs with no hosts or builders' key, or on a Mac, listens elsewhere than %t/pi-agent/agent.sock or to more than the account, isn't linked from %t/pi-agent.sock or doesn't clear what stands there first, or isn't told of its hosts, the agents behind it, or the builders' key and the builders lent."
      (
        lib.head (lib.toList socket.ListenStream) == "%t/pi-agent/agent.sock"
        && lib.head (lib.toList socket.SocketMode) == "0600"
        && lib.head (lib.toList socket.DirectoryMode) == "0700"
        && lib.toList socket.Symlinks == [ "%t/pi-agent.sock" ]
        && lib.hasSuffix "/bin/rm -f %t/pi-agent.sock" (lib.head (lib.toList socket.ExecStartPre))
        && !(lib.hasInfix "--key" start)
        && lib.hasInfix "--key /home/someone/.local/state/nix-offload/store_ed25519" lends
        && lib.hasInfix "--lent /run/nix-offload-lines/lent" lends
        && !(lib.hasInfix "--upstream" lends)
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
  # forwarded agent, or adding, removing and locking keys; no more than
  # 64 clients answered at once, a place given back as each goes; the
  # builders' key listed first and signing a login as nix-offload to a
  # builder lent now, and no other: not as another account, not to a host
  # that isn't lent, not to a standing builder, not once a builder leaves
  # root's file, and no agent's key for a lent builder; an agent
  # behind that never answers given up on; and each login signed and each
  # refusal said in one line of a fixed shape, with its host, its reason
  # as a word from a fixed set, and the session the client claims.
  pi-agent-proxy-signs-only-logins = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-agent-proxy, its tests included.
  pi-agent-proxy-passes-clippy = passesClippy "pi-agent-proxy-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
