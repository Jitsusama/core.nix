# ssh-tpm-agent holds SSH keys sealed in the TPM, so no private key ever sits
# on the disk, and it serves every SSH login and SSH-signed commit. When a key
# wants its PIN or a confirmation, the agent runs SSH_ASKPASS, which the
# desktop's shell answers (modules/quickshell). The keys are made on the
# machine itself: jitsusama.tpmKeys names them, which the guide makes on a
# machine it installed (modules/guide/src/installed/keys.rs), and
# docs/installing.md says how to make them by hand. The login key's spare
# lives on each YubiKey, for when the TPM is wiped or the laptop is gone,
# which the guide keeps there too (modules/guide/src/installed/spares.rs).
{
  config,
  lib,
  pkgs,
  ...
}:
{
  imports = [ ../yubikey-spare/home.nix ];

  options.jitsusama.tpmKeys = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          file = lib.mkOption {
            type = lib.types.str;
            description = "Where the key is kept, without the .tpm and .pub ssh-tpm-keygen adds.";
          };
          label = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "signing";
            description = "What its comment says it's for, after the account and the machine.";
          };
          pin = lib.mkOption {
            type = lib.types.bool;
            description = "Whether it asks for a PIN before it's used.";
          };
          github = lib.mkOption {
            type = lib.types.nullOr (
              lib.types.enum [
                "authentication"
                "signing"
              ]
            );
            default = null;
            description = "What `guide finish` adds it to GitHub as: a key that signs in, or one that signs commits.";
          };
        };
      }
    );
    default = [ ];
    description = ''
      The SSH keys this account keeps in the TPM, each named by the module
      that uses it. The guide makes each one missing on a machine it
      installed, as the account, with one PIN for every key that has one.
    '';
  };

  config = {
    services.ssh-tpm-agent.enable = true;

    # The agent keeps the login key's PIN only for as long as it runs, so a
    # switch that restarted it would ask for the PIN again, and every SSH login
    # and signed commit would wait until someone typed it, the person often
    # away. A changed agent starts at the next login instead, when they're
    # there to type it.
    systemd.user.services.ssh-tpm-agent.Unit.X-SwitchMethod = "keep-old";

    # So the switch that first brings the gate, and the core limit below,
    # leaves the agent already running without either until its next start.
    # systemctl shows the new unit's environment straight after the switch,
    # so the agent's journal says at each start which gate it judges by,
    # and /proc/<its pid>/environ says what the running one has.
    # zsh's socket text below waits on nothing: each shell opened after the
    # switch keeps the agent it was given.
    #
    # The gate is read again for each connection, so a switch changes it
    # without the restart that would ask for the PIN again. Only this agent,
    # the one holding Joel's keys, has one; the agents' own signing agents
    # run the same program without it. It sits in his own config, which
    # anything running as him may rewrite, so it catches accidents rather
    # than escapes. One the agent never read whole, emptied, unreadable,
    # refusing nowhere or gone, refuses everyone, since it looks just like
    # one that works, until the link is back, which serves Joel at the next
    # connection with no restart of the agent; the journal says so. A switch
    # that changes nothing leaves Home Manager's unit as it is, so the link
    # comes back with a restart of home-manager-<user>.service instead. A
    # signing key added while the gate refused is missing until its unit,
    # tpm-signing's signing-key.service, is started again. One the agent
    # read that is gone, unreadable or garbled since, as rolling back to
    # before the gate or a later gate in a format this agent predates
    # leaves it, is judged by what it last said, so Joel isn't shut out
    # until a restart asks for the PIN again and agents' work stays refused.
    # So reverting the gate leaves its last rules in force until a restart;
    # to stop it refusing without one, switch to a gate.conf whose first line
    # is watch, which journals each refusal and serves it, keeping its
    # refuse lines, since a gate refusing no place refuses everyone.
    # A file put in the link's place is replaced at the next switch, rather
    # than stopping the switch that puts the gate back.
    xdg.configFile."ssh-tpm-agent/gate" = {
      source = ./gate.conf;
      force = true;
    };
    systemd.user.services.ssh-tpm-agent.Service.Environment = [
      "SSH_TPM_AGENT_GATE=${config.xdg.configHome}/ssh-tpm-agent/gate"
    ];

    # The agent holds the login key's PIN, and a core of it would put that
    # PIN in a file, wherever the kernel's core pattern sends it: so it
    # keeps none, with a hard limit no process of his can raise with
    # prlimit. It closes only that way out; anything running as Joel that
    # may trace his processes can still read the agent's memory.
    systemd.user.services.ssh-tpm-agent.Service.LimitCORE = 0;

    # Home Manager has every shell not reached over SSH point SSH_AUTH_SOCK
    # at this agent over whatever it was given, so a zsh an agent's command
    # started took Joel's agent back in place of the agents' own proxy its
    # session was given. A zsh keeps the agent it was given, and only one
    # given none points here. bash's text stays Home Manager's, since its
    # set-SSH_AUTH_SOCK service runs it at login to point the account's
    # services here whatever the user manager already held.
    sshAuthSock.initialization.zsh = lib.mkForce ''
      export SSH_AUTH_SOCK="''${SSH_AUTH_SOCK:-$XDG_RUNTIME_DIR/ssh-tpm-agent.sock}"
    '';

    # The login key, which the agent loads from ~/.ssh when it starts and
    # which asks for its PIN once a session.
    jitsusama.tpmKeys = [
      {
        file = "${config.home.homeDirectory}/.ssh/id_ecdsa";
        pin = true;
        github = "authentication";
      }
    ];

    # The agent starts serving before it holds the keys it read from ~/.ssh, so
    # the request that wakes it through its socket finds none, and the first
    # signature after login failed. The patch is the change upstream needs; take
    # it out once a release carries it.
    #
    # Each signature with a key that has a PIN also locked a page of memory it
    # never gave back, so after a few hundred the agent refused every signature
    # until it restarted. The second patch frees it; upstream's own fix (pull
    # request 135) isn't in a release yet, so take it out once one carries it.
    #
    # A PIN prompt nobody answered also held up every request to the agent, for
    # every key, since the PIN was asked for inside locks every signature
    # shares: one program using a key whose PIN wasn't typed this session froze
    # every SSH login and signed commit for 21 minutes one night, and a
    # confirmation does the same. The third patch asks for both outside them,
    # stops a prompt nobody answers after a minute, and refuses that key for ten
    # minutes without another, naming it.
    #
    # Patched, it builds here rather than coming from the cache, and four of its
    # tests check which error the kernel's keyring gives for a missing key. That
    # depends on the kernel: nixpkgs rewrites them to expect ENOENT, and some
    # kernels give ENOKEY, so they fail there whatever the agent does. They test
    # the kernel, not the agent, so they're skipped. The pattern has no
    # parentheses or question marks: the build's shell reads those as a glob and
    # drops the flag without a word.
    #
    # Agents run as Joel's own account, so the agent's socket, open to the
    # account, served them every key it holds: his GitHub login, his cloud
    # one and his way into his work Mac among them. The fourth patch refuses
    # each connection from where agents' work runs, which ./gate.conf names
    # and ./gate-cases.txt proves, by the cgroups of the process asking and its
    # parents; its build runs those cases. It catches an agent's mistake,
    # never one set on getting round it: a process of the account's can
    # start itself somewhere unnamed, and keeping it in is the fence's work.
    services.ssh-tpm-agent.package = pkgs.ssh-tpm-agent.overrideAttrs (old: {
      patches = (old.patches or [ ]) ++ [
        ./keys-before-serving.patch
        ./free-cached-pin.patch
        ./locked-key-blocks-nothing.patch
        ./refuses-agents.patch
      ];
      preCheck = (old.preCheck or "") + ''
        mkdir -p agent/testdata
        cp ${./gate.conf} agent/testdata/gate
        cp ${./gate-cases.txt} agent/testdata/gate-cases
      '';
      checkFlags = (old.checkFlags or [ ]) ++ [
        "-skip=^TestNoKey$|^TestRemoveKey$|^TestNoKeyThreaded$|^TestRemoveKeyThreaded$"
      ];
    });
  };
}
