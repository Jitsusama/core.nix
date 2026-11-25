# pi-agent-proxy, the one SSH agent the agent fence lets in, and the OS
# Login key it holds for agents' gcloud compute ssh: its options, its
# socket and service, and the units that put the key in the person's
# login profile and renew it there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The fence's SSH agent, run only where it has hosts to sign logins to.
  # It depends on the hosts alone, since the upstreams' default looks at
  # which sockets are declared, these among them.
  agentProxy = pkgs.callPackage ./package.nix { };
  proxied =
    pkgs.stdenv.hostPlatform.isLinux
    && (
      config.jitsusama.pi.agentProxy.machines != [ ] || config.jitsusama.pi.agentProxy.builderKey != null
    );

  # What puts the proxy's OS Login key in the person's profile and takes
  # it out (os-login.sh), where the proxy makes one.
  osLoginKnownHosts = config.jitsusama.pi.agentProxy.osLoginKnownHosts;
  osLogin = proxied && osLoginKnownHosts != null;
  osLoginKey = "%t/pi-agent/os-login.pub";
  osLoginRegister = pkgs.writeShellApplication {
    name = "pi-agent-proxy-os-login";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.openssh
    ];
    # The proxy lists its own key only once every agent behind it has
    # answered or had its 5 s, so the script waits that out (os-login.sh).
    text = ''
      asking=${toString (5 * lib.length config.jitsusama.pi.agentProxy.upstreams + 3)}
    ''
    + builtins.readFile ./os-login.sh;
  };
in
{
  options.jitsusama.pi.agentProxy = {
    machines = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI... builder" ];
      description = ''
        The host keys, as OpenSSH public keys, of the machines a fenced pi
        command may log in to through pi-agent-proxy (agent-proxy/src).
        With none, and no builderKey, no proxy runs and fenced commands
        reach no SSH agent.
      '';
    };
    builderKey = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "/home/me/.local/state/nix-offload/store_ed25519";
      description = ''
        The private key nix-offload logs in to the builders it's lent with,
        which the proxy holds for fenced commands so the key itself stays
        hidden from them. It signs with it only a login as nix-offload to a
        builder a file in lentFiles names now, never any other.
      '';
    };
    lentFiles = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      example = [ "/run/nix-offload-lines/lent" ];
      description = ''
        The files root writes of the builders it vouches for now, read
        afresh at each login, one builder a line as `<lender> <instance>
        <key-type> <base64-key>`: a nix-offload lender's ed25519 host key
        is lent to builderKey's logins. Never a file the account can write,
        or a command could lend itself a host of its own.
      '';
    };
    osLoginKnownHosts = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = "%h/.ssh/google_compute_known_hosts";
      example = null;
      description = ''
        The known-hosts file gcloud keeps of the instances it reaches,
        `%h` for the home folder, whose `compute.<id>` hosts an agent's
        `gcloud compute ssh` may log in to as the person through OS Login.
        The proxy makes a key for them as it starts, written to no file
        and kept out of swap, short of a hibernation image, and the account's gcloud puts it in the
        person's login profile for twelve hours at a time, renewed while
        the proxy runs and taken out as it stops (agent-proxy/src/os_login.rs).
        So agents reach, as the person, every instance the file names,
        which their fence keeps read-only, by IAP or by its address; where
        gcloud-broker gives a fenced agent's gcloud a service account, a
        login naming the person's own user still goes past that account's
        one project. Null for no such key, where the
        person's own key serves agents too.
      '';
    };
    upstreams = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = lib.optional (config.systemd.user.sockets ? agent-signing) "%t/agent-signing.sock";
      defaultText = lib.literalExpression ''[ "%t/agent-signing.sock" ], where the agents' key is served, else [ ]'';
      description = ''
        The agents' sockets, `%t` for the runtime folder, whose keys the
        proxy lists and signs logins with. Only the agents' own by default:
        the person's key asks a confirmation on every signature, so an agent
        that offered it while they were away would hold every other
        signature until they came back.
      '';
    };
  };

  config = {
    assertions = [
      {
        assertion =
          config.jitsusama.pi.agentProxy.machines != [ ] -> config.jitsusama.pi.agentProxy.upstreams != [ ];
        message = "jitsusama.pi.agentProxy.machines names machines, but no agent's socket is given in jitsusama.pi.agentProxy.upstreams to sign their logins with.";
      }
      {
        assertion =
          config.jitsusama.pi.agentProxy.builderKey != null
          -> config.jitsusama.pi.agentProxy.lentFiles != [ ];
        message = "jitsusama.pi.agentProxy.builderKey is given, but no file in jitsusama.pi.agentProxy.lentFiles names the builders it may sign for.";
      }
    ];

    # The one SSH agent the agent fence lets in, at %t/pi-agent/agent.sock:
    # it signs, with the agents' key, only logins to the hosts named in
    # jitsusama.pi.agentProxy.machines, and with the builders' key only
    # nix-offload's logins to the builders lent now, each proved by the
    # machine's own signature over the session (src/bind.rs).
    # Started by the first ssh that asks; every refusal says why in the
    # journal. The socket sits in a folder of its own so the fence can let
    # the folder in, which still holds the socket systemd makes anew when
    # the socket unit restarts; %t/pi-agent.sock links to it meanwhile.
    # systemd never makes a link over a file already there, and a machine
    # switched from the socket's old place keeps its old, dead socket at
    # that path until it boots, so the old path refused every client on
    # 14 November 2026 until it was cleared by hand. Clearing it before
    # the socket starts lets systemd make the link.
    systemd.user.sockets.pi-agent-proxy = lib.mkIf proxied {
      Unit.Description = "The SSH agent pi's fenced commands reach, signing only logins to the hosts it's told of";
      Socket = {
        ListenStream = "%t/pi-agent/agent.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
        ExecStartPre = "${pkgs.coreutils}/bin/rm -f %t/pi-agent.sock";
        Symlinks = [ "%t/pi-agent.sock" ];
        Service = "pi-agent-proxy.service";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
    systemd.user.services.pi-agent-proxy = lib.mkIf proxied {
      Unit = {
        Description = "The SSH agent pi's fenced commands reach, signing only logins to the hosts it's told of";
        Requires = [ "pi-agent-proxy.socket" ];
        After = [ "pi-agent-proxy.socket" ];
      };
      Service.ExecStart = lib.escapeShellArgs (
        [
          (lib.getExe agentProxy)
          "--hosts"
          "${pkgs.writeText "pi-agent-proxy-hosts" (lib.concatLines config.jitsusama.pi.agentProxy.machines)}"
        ]
        ++ lib.concatMap (socket: [
          "--upstream"
          socket
        ]) config.jitsusama.pi.agentProxy.upstreams
        ++ lib.optionals (config.jitsusama.pi.agentProxy.builderKey != null) (
          [
            "--key"
            config.jitsusama.pi.agentProxy.builderKey
          ]
          ++ lib.concatMap (file: [
            "--lent"
            file
          ]) config.jitsusama.pi.agentProxy.lentFiles
        )
        ++ lib.optionals osLogin [
          "--os-login"
          "%t/pi-agent"
          "--os-login-hosts"
          osLoginKnownHosts
        ]
      );
      # Taken out of the person's profile as the proxy stops, since the
      # key's private half goes with it.
      Service.ExecStopPost = lib.mkIf osLogin "-${lib.getExe osLoginRegister} remove ${osLoginKey}";
      # The OS Login key is written to no file, so no crash writes the
      # proxy's memory out, and none of it goes to swap; the proxy also
      # makes itself undumpable. A hibernation image still holds it,
      # inside the disk's encryption, as it holds all of memory.
      Service.LimitCORE = lib.mkIf osLogin 0;
      Service.MemorySwapMax = lib.mkIf osLogin 0;
      # Started with the session rather than by the first ssh, where it
      # makes an OS Login key, so the key's public half is there for an
      # agent's first ssh to offer.
      Install.WantedBy = lib.mkIf osLogin [ "default.target" ];
    };

    # The proxy's OS Login key goes into the person's profile each time it
    # makes one, and its twelve hours are renewed each quarter hour while
    # it runs. The renewal is by the wall clock, whose timers fire on
    # waking, since the profile's twelve hours run while a laptop sleeps;
    # a registration that failed, as on waking before the network is back,
    # is tried again half a minute on, then less often, to a quarter hour,
    # never giving up, since a unit at its start limit would leave the key
    # to lapse. It runs again as the desktop starts, by which time the
    # session's PATH, where the person's gcloud may be, is the user
    # manager's; the proxy starts before it. The path and the timer are
    # bound to the proxy, so neither puts back the key its stop took out;
    # the path is watching before the proxy writes its first key. The
    # timer isn't ordered after the proxy, which a timer's own ordering
    # before timers.target would make a cycle of at login.
    systemd.user.services.pi-agent-proxy-os-login = lib.mkIf osLogin {
      Unit = {
        Description = "Put pi's agent proxy's OS Login key in the person's login profile";
        Requisite = [ "pi-agent-proxy.service" ];
        After = [ "pi-agent-proxy.service" ];
        PartOf = [ "pi-agent-proxy.service" ];
        StartLimitIntervalSec = 0;
      };
      Service = {
        Type = "oneshot";
        ExecStart = "${lib.getExe osLoginRegister} register ${osLoginKey}";
        # A few seconds for the key, then three calls of a minute each.
        TimeoutStartSec = "4min";
        Restart = "on-failure";
        RestartSec = "30s";
        RestartSteps = 5;
        RestartMaxDelaySec = "15min";
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
    systemd.user.paths.pi-agent-proxy-os-login = lib.mkIf osLogin {
      Unit = {
        Description = "Register each OS Login key pi's agent proxy makes";
        BindsTo = [ "pi-agent-proxy.service" ];
        Before = [ "pi-agent-proxy.service" ];
      };
      Path.PathChanged = osLoginKey;
      Install.WantedBy = [ "pi-agent-proxy.service" ];
    };
    systemd.user.timers.pi-agent-proxy-os-login = lib.mkIf osLogin {
      Unit = {
        Description = "Renew pi's agent proxy's OS Login key before it lapses";
        BindsTo = [ "pi-agent-proxy.service" ];
      };
      Timer = {
        OnCalendar = "*:0/15";
        Persistent = true;
      };
      Install.WantedBy = [ "pi-agent-proxy.service" ];
    };
  };
}
