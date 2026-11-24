# The checks of pi-agent-proxy, the SSH agent pi's fenced commands reach,
# beside pi's own in ../checks.nix.
{
  account,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  onLinux,
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

  # Where the proxy runs, it makes a key for the OS Login hosts gcloud
  # knows, starting with the session so the key is there for an agent's
  # first ssh, and never dumping its memory; the account's gcloud registers
  # each key it writes, renews it each quarter hour by the wall clock, so a
  # night asleep doesn't leave it lapsed, bounded so none hangs, and takes
  # it out as the proxy stops; and the machine's ssh offers it to every
  # compute.<id> host, and its block ends so what follows reaches every
  # host. Null known hosts turns it off.
  # The registering script is run against a stand-in gcloud
  # (os-login-test.sh).
  pi-agent-proxy-reaches-os-login-hosts =
    let
      key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
      proxying = osLoginHosts: {
        home-manager.sharedModules = [
          {
            jitsusama.pi.agentProxy = {
              machines = [ key ];
              upstreams = [ "%t/agents.sock" ];
              inherit osLoginHosts;
            };
          }
        ];
      };
      machine = bareNixos [
        self.nixosModules.workstation
        (proxying "%h/.ssh/google_compute_known_hosts")
      ];
      home = account machine;
      off = account (bareNixos [
        self.nixosModules.workstation
        (proxying null)
      ]);
      proxy = home.systemd.user.services.pi-agent-proxy;
      start = lib.head (lib.toList proxy.Service.ExecStart);
      stop = lib.head (lib.toList proxy.Service.ExecStopPost);
      register = lib.head (
        lib.toList home.systemd.user.services.pi-agent-proxy-os-login.Service.ExecStart
      );
      offStart = lib.head (lib.toList off.systemd.user.services.pi-agent-proxy.Service.ExecStart);
      wired =
        holds "pi-agent-proxy-reaches-os-login-hosts-wired"
          "pi's SSH agent proxy makes no OS Login key from gcloud's known hosts, or one with them null, isn't started with the session, its key isn't registered when written, renewed each quarter hour by the wall clock within a bound, or removed as it stops, may dump its memory, or the machine's ssh doesn't offer it to compute.* hosts alone."
          (
            lib.hasInfix "--os-login %t/pi-agent --os-login-hosts %h/.ssh/google_compute_known_hosts" start
            && lib.toList proxy.Install.WantedBy == [ "default.target" ]
            && lib.hasPrefix "-" stop
            && lib.hasSuffix "/bin/pi-agent-proxy-os-login remove %t/pi-agent/os-login.pub" stop
            && lib.hasSuffix "/bin/pi-agent-proxy-os-login register %t/pi-agent/os-login.pub" register
            &&
              lib.toList home.systemd.user.paths.pi-agent-proxy-os-login.Path.PathChanged == [
                "%t/pi-agent/os-login.pub"
              ]
            && home.systemd.user.timers.pi-agent-proxy-os-login.Timer.OnCalendar == "*:0/15"
            && home.systemd.user.timers.pi-agent-proxy-os-login.Timer.Persistent
            && home.systemd.user.services.pi-agent-proxy-os-login.Service.TimeoutStartSec == "3min"
            && proxy.Service.LimitCORE == 0
            && lib.hasInfix "Host compute.*\n  IdentityFile /run/user/%i/pi-agent/os-login.pub\nHost *\n" machine.config.programs.ssh.extraConfig
            && !(lib.hasInfix "--os-login" offStart)
            && !(off.systemd.user.services ? pi-agent-proxy-os-login)
            && !(off.systemd.user.paths ? pi-agent-proxy-os-login)
            && !(off.systemd.user.timers ? pi-agent-proxy-os-login)
          );
    in
    pkgs.runCommandLocal "pi-agent-proxy-reaches-os-login-hosts"
      { nativeBuildInputs = [ pkgs.openssh ]; }
      ''
        cat ${wired}
        $BASH ${./os-login-test.sh} ${lib.removeSuffix " register %t/pi-agent/os-login.pub" register}
        touch $out
      '';
}
# The proxy knows its caller by SO_PEERCRED, which only Linux has.
// onLinux {
  # pi-agent-proxy builds with its tests run (tests/agent_proxy.rs):
  # the keys behind it listed, a login signed once its allowed host signed
  # the session, and nothing else reaching the agent behind it: no session,
  # another host, a forged binding, a commit, another session or key, a
  # forwarded agent, or adding, removing and locking keys; no more than
  # 64 clients answered at once, a place given back as each goes; the
  # builders' key listed first and signing a login as nix-offload to a
  # builder lent now, and no other: not as another account, not to a host
  # that isn't lent, not to a standing builder, not once a builder leaves
  # root's file, and no agent's key for a lent builder; a key made in
  # memory listed last and signing only for an OS Login host gcloud's
  # known hosts name, neither told nor lent, and no other key for one,
  # the proxy going on without it when it can't be written; an agent
  # behind that never answers given up on; and each login signed and each
  # refusal said in one line of a fixed shape, with its host, its reason
  # as a word from a fixed set, and the session the client claims.
  pi-agent-proxy-signs-only-logins = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in pi-agent-proxy, its tests included.
  pi-agent-proxy-passes-clippy = passesClippy "pi-agent-proxy-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
