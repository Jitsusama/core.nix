# The checks of mycelium, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
let
  # A workstation of either system whose account sets mycelium's options.
  linking =
    bare: role: settings:
    account (bare [
      role
      { home-manager.users.someone.jitsusama.mycelium = settings; }
    ]);
  linux = linking bareNixos self.nixosModules.workstation;
  mac = linking bareDarwin self.darwinModules.workstation;

  links = {
    realm = "personal";
    links = [
      "one"
      "two"
    ];
  };
  ssh = [
    "--ssh"
    "one"
    "--ssh"
    "two"
  ];
  # home-manager keeps ExecStart as a list of its lines.
  linuxNode = toString (linux links).systemd.user.services.mycelium.Service.ExecStart;
  linuxAgent = (linux links).systemd.user.services.mycelium.Service.Environment;
  # The home realm's node for a machine naming one via, forced whole so a
  # value the option's type refuses fails here.
  viaNode =
    via:
    let
      words =
        toString
          (linux (links // { vias = [ via ]; })).systemd.user.services.mycelium.Service.ExecStart;
    in
    builtins.deepSeq words words;
  # The same for a machine naming one provider's program.
  providerNode =
    program:
    let
      words =
        toString
          (linux (links // { providers = [ program ]; })).systemd.user.services.mycelium.Service.ExecStart;
    in
    builtins.deepSeq words words;
  macNode = (mac links).launchd.agents.mycelium.config.ProgramArguments;
  # A Mac workstation signs its commits from the Secure Enclave, so its node
  # signs its links in with the same key, each option after its own flag.
  enclaveOptions = lib.concatMap (option: [
    "--ssh-option"
    option
  ]) (mac links).jitsusama.signing.signer.agentSshOptions;
  answering = linux { realm = "personal"; };
  apart = linux { };

  both = {
    realm = "home";
    links = [ "one" ];
    alsoJoins.other.links = [ "two" ];
  };
  bothLinux = (linux both).systemd.user.services;
  otherNode = toString bothLinux.mycelium-other.Service.ExecStart;
  otherApart = [
    "--state"
    "/home/someone/.local/state/mycelium/other"
    "--mesh"
    "/home/someone/.local/state/pi/agentic-harness.pi/realms/other/mesh-workflow"
    "--steer"
    "/home/someone/.local/state/pi/agentic-harness.pi/realms/other/steer"
  ];

  declaring = {
    reach = [ "internet" ];
    probes.home = "192.0.2.1:53";
    probes.office = [
      "test"
      "-e"
      "/run/office"
    ];
    taints = [
      {
        key = "person-works";
        effect = "prefer_not";
        why = "its person works here";
      }
    ];
  };
  # What a declaration says, read as mycelium reads it. A fence's launcher
  # names its program in the store, which fromJSON refuses to read unless
  # the text forgets it was built from that path; the words are the same.
  read = text: builtins.fromJSON (builtins.unsafeDiscardStringContext text);
  declared = bare: read (bare { declares = declaring; }).xdg.configFile."mycelium/declared.json".text;
  # A machine within another, and the host it runs in, each naming the other.
  declaredAs =
    extra: read (linux { declares = declaring // extra; }).xdg.configFile."mycelium/declared.json".text;
  inVm = declaredAs {
    within = {
      machine = "host-machine";
      file = "/run/host/said.json";
    };
  };
  # A machine whose strictly fenced sessions build remotely, and no other.
  strictly = declaredAs {
    fence.strict = {
      builders = [ "remote" ];
      forges = [ ];
      secrets = [ ];
    };
  };
  # A machine fenced every way.
  launching =
    bare:
    read
      (bare {
        declares = declaring // {
          fence = lib.genAttrs [ "none" "fence" "strict" ] (_: {
            builders = [ ];
            forges = [ ];
            secrets = [ ];
          });
        };
      }).xdg.configFile."mycelium/declared.json".text;
  # pi's own fence, strict, which an agent's run starts through on Linux.
  launcher = [
    (lib.getExe (pkgs.callPackage ../pi/fence-run/package.nix { }))
    "--kind"
    "strict"
    "--"
  ];
  hosting = declaredAs {
    hosts = [
      {
        machine = "guest-machine";
        file = "/var/host/said.json";
      }
    ];
  };
  # A machine at home naming the command its sessions there start with, or
  # one in a realm it doesn't join.
  saying =
    agents:
    linux {
      realm = "home";
      declares = declaring;
      inherit agents;
    };
  refusesAgents =
    home:
    lib.any (
      check: !check.assertion && lib.hasInfix "jitsusama.mycelium.agents" check.message
    ) home.assertions;
  # A machine in both realms, and one in the sending realm alone, keeping a
  # flow from other into home; and flows naming no realm a machine joins.
  flowing = {
    from = "other";
    to = "home";
  };
  bothFlowing = (linux (both // { flows = [ flowing ]; })).systemd.user.services;
  sendingOnly =
    toString
      (linux {
        realm = "other";
        flows = [ flowing ];
      }).systemd.user.services.mycelium.Service.ExecStart;
  refusesFlows =
    flows:
    lib.any (check: !check.assertion && lib.hasInfix "jitsusama.mycelium.flows" check.message)
      (linux (both // { inherit flows; })).assertions;
in
{
  # A flow is kept by both realms' nodes, one way: the sending realm's node
  # hands text into the other, and the receiving realm's takes it from the
  # first, neither naming the reverse. A machine in the sending realm alone
  # keeps it too, so what it sends waits for a link to carry it, and one
  # naming no flow keeps every realm apart. A flow into its own realm, or
  # between realms the machine joins neither of, is refused.
  mycelium-flows-one-way =
    holds "mycelium-flows-one-way"
      "A realm flow isn't kept one way by both realms' nodes, a machine in the sending realm alone doesn't keep it, a node names a flow nobody declared, or a flow into its own realm or between two realms the machine doesn't join is taken."
      (
        lib.hasInfix "node ${lib.escapeShellArgs otherApart} --flow-to home --ssh two" (
          toString bothFlowing.mycelium-other.Service.ExecStart
        )
        && lib.hasInfix "--realm home node --flow-from other --ssh one" (
          toString bothFlowing.mycelium.Service.ExecStart
        )
        && !lib.hasInfix "--flow-from" (toString bothFlowing.mycelium-other.Service.ExecStart)
        && !lib.hasInfix "--flow-to" (toString bothFlowing.mycelium.Service.ExecStart)
        && lib.hasInfix "--realm other node --flow-to home --serves" sendingOnly
        && !lib.hasInfix "--flow" (toString bothLinux.mycelium.Service.ExecStart)
        && !lib.hasInfix "--flow" (toString bothLinux.mycelium-other.Service.ExecStart)
        && !refusesFlows [ flowing ]
        && refusesFlows [
          {
            from = "home";
            to = "home";
          }
        ]
        && refusesFlows [
          {
            from = "third";
            to = "fourth";
          }
        ]
      );

  # A workstation in a realm runs the node in it, dialling each machine it
  # names, over SSH or through a provider's program named by its path, in
  # its home realm and in each further one, and asks each provider's
  # program it names what it could make: a user service on Linux, an
  # agent on a Mac. A program named any other way, or a place it could take
  # for a flag or systemd would expand, doesn't evaluate. One that names
  # none runs it dialling nothing, since its node still holds its sessions
  # and starts what others ask of it, and one in no realm has neither a
  # realm nor a node. On Linux the node signs in with the agents' key,
  # which asks nothing, unless its realm names a key of its own; on a Mac
  # what it starts signs commits with that key, since nobody at another
  # machine can touch this one's sensor. A Mac's node finds the system's
  # ssh first and the account's programs after it, among them the
  # harness's pi-carry, which a carry runs by name.
  mycelium-links-what-a-machine-names =
    holds "mycelium-links-what-a-machine-names"
      "A machine's links, over SSH or through a provider's program, don't each get dialled in its realm, its providers' programs aren't each asked, a via or a provider naming its program by no path or a place it could misread evaluates, Linux's node signs in with a key that may ask for a PIN or not the one its realm names, a Mac's node starts sessions whose commits wait on its sensor or finds the system's ssh after the account's programs or none of them, a machine in a realm naming none runs no node, or one in none runs one."
      (
        lib.hasInfix "--realm personal node --ssh one --ssh two" linuxNode
        && lib.hasInfix "--ssh one --ssh two --via /bin/cell:cell-1 --serves" (viaNode "/bin/cell:cell-1")
        && lib.all (wrong: !(builtins.tryEval (viaNode wrong)).success) [
          "cell:cell-1"
          "/bin/cell:"
          "/bin/cell:-c"
          "/bin/my cell:cell-1"
          "/bin/cell:cell 1"
          "/bin/cell:cell-%h"
          "/bin/cell:cell-$HOME"
        ]
        && lib.hasInfix "--ssh one --ssh two --provider /bin/cell --serves" (providerNode "/bin/cell")
        && lib.all (wrong: !(builtins.tryEval (providerNode wrong)).success) [
          "cell"
          "/bin/my cell"
          "/bin/cell:make"
          "/bin/cell-%h"
        ]
        && lib.hasInfix "--ssh two --via /bin/cell:cell-2 --provider /bin/cell --serves" (
          toString
            (linux (
              both
              // {
                alsoJoins.other = {
                  links = [ "two" ];
                  vias = [ "/bin/cell:cell-2" ];
                  providers = [ "/bin/cell" ];
                };
              }
            )).systemd.user.services.mycelium-other.Service.ExecStart
        )
        && linuxAgent == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        &&
          (linux (links // { signsInWith = "personal-link"; }))
          .systemd.user.services.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/personal-link.sock" ]
        &&
          lib.drop 1 macNode == [
            "--realm"
            "personal"
            "node"
          ]
          ++ enclaveOptions
          ++ ssh
          ++ [
            "--serves"
            "${(mac links).xdg.configHome}/mycelium/serves-personal.json"
          ]
        && (mac links).launchd.agents.mycelium.enable
        && (mac links).launchd.agents.mycelium.config.EnvironmentVariables.SIGN_AS == "agent"
        &&
          (mac links).launchd.agents.mycelium.config.EnvironmentVariables.PATH
          == "/usr/bin:/bin:${(mac links).home.profileDirectory}/bin"
        && lib.hasInfix "--realm personal node" (
          toString answering.systemd.user.services.mycelium.Service.ExecStart
        )
        && !(lib.hasInfix "--ssh" (toString answering.systemd.user.services.mycelium.Service.ExecStart))
        && (mac { realm = "personal"; }).launchd.agents.mycelium.enable
        && !(apart.systemd.user.services ? mycelium)
        && answering.home.sessionVariables.MYCELIUM_REALM == "personal"
        && !(apart.home.sessionVariables ? MYCELIUM_REALM)
        && lib.any (package: (package.pname or "") == "mycelium") apart.home.packages
      );

  # A machine in a realm reads each minute how evenly the home realm's
  # machines carry its work, on Linux in the agents' slice, on a Mac as an
  # agent; only the home realm is read, and a machine in none reads nothing.
  mycelium-watches-its-balance =
    holds "mycelium-watches-its-balance"
      "A machine in a realm doesn't read its home realm's balance each minute, reads it outside the agents' slice on Linux, reads a further realm's, or one in no realm reads one."
      (
        let
          linuxBalance = bothLinux.mycelium-balance.Service;
          macBalance = (mac both).launchd.agents.mycelium-balance;
        in
        lib.hasSuffix "/bin/mycelium --realm home balance --follow" (toString linuxBalance.ExecStart)
        && linuxBalance.Slice == "agents.slice"
        && macBalance.enable
        &&
          lib.drop 1 macBalance.config.ProgramArguments == [
            "--realm"
            "home"
            "balance"
            "--follow"
          ]
        && !(bothLinux ? mycelium-other-balance)
        && !(apart.systemd.user.services ? mycelium-balance)
      );

  # The home realm's node on Linux shows the Nix builders nix-offload's
  # lists name beside the machines, the lent one among them, by the folder
  # its lender keeps its health in; a further realm's node shows none, as
  # they are the machine's, and a Mac's, with no lists, is given none.
  mycelium-shows-the-builders =
    holds "mycelium-shows-the-builders"
      "Linux's home realm node isn't given nix-offload's standing, beside and lent lists and the lent builder's health, a further realm's node is, or a Mac's node is."
      (
        lib.hasInfix (lib.escapeShellArgs [
          "--builders"
          "standing=/run/nix-offload-lines/machines-with-standing"
          "--builders"
          "beside=/run/nix-offload-lines/machines-beside"
          "--lent-builders"
          "lent=/run/nix-offload-lines/machines-for-anything"
          "--lent-health"
          "/run/{host}/health.json"
        ]) linuxNode
        && !(lib.hasInfix "--builders" otherNode)
        && !(lib.elem "--builders" macNode)
        && !(lib.elem "--lent-builders" macNode)
      );

  # A Mac's node signs its links in with the agents' key from the Secure
  # Enclave, through that key's own library and no agent, since the login
  # session's one may ask for an approval nobody is there to give; one
  # without that key's module passes no such options, and Linux's node never
  # does.
  mycelium-signs-a-macs-links-with-the-enclave =
    holds "mycelium-signs-a-macs-links-with-the-enclave"
      "A Mac's node signs its links in through whatever agent the login holds, or not with the agents' key from the Secure Enclave, a Mac without that key's module is given options for it, or Linux's node is given a Mac's options."
      (
        let
          enclave = {
            home-manager.sharedModules = [ self.homeModules.secure-enclave-signing ];
          };
          withEnclave =
            (bareDarwin [
              self.darwinModules.workstation
              enclave
              { home-manager.users.someone.jitsusama.mycelium = links; }
            ]).config.home-manager.users.someone;
          words = withEnclave.launchd.agents.mycelium.config.ProgramArguments;
        in
        lib.elem "IdentityFile=/Users/someone/.ssh/agent-signing" words
        && lib.elem "SecurityKeyProvider=/usr/lib/ssh-keychain.dylib" words
        && lib.elem "IdentityAgent=none" words
        && lib.elem "IdentitiesOnly=yes" words
        && lib.elem "--ssh-option" words
        && !(lib.elem "--ssh-option" (lib.splitString " " (toString linuxNode)))
      );

  # A machine joining a further realm runs a node of its own for it, apart
  # as another machine's would be: its own state, and the mesh and steer
  # folders the harness keeps for a realm that isn't home. Its links sign in
  # with that realm's own key, never the agents' key every session reaches,
  # and the home realm's node is as it was. Its pis know which realm is home.
  mycelium-keeps-each-realm-apart =
    holds "mycelium-keeps-each-realm-apart"
      "A further realm's node shares the home realm's state, mesh, steer folders or key, the home realm's node changes, or a pi isn't told its home realm."
      (
        lib.hasInfix (lib.escapeShellArgs (
          [
            "--realm"
            "other"
            "node"
          ]
          ++ otherApart
          ++ [
            "--ssh"
            "two"
          ]
        )) otherNode
        && bothLinux.mycelium-other.Service.Environment == [ "SSH_AUTH_SOCK=%t/other-link.sock" ]
        && bothLinux.mycelium-other.Unit.Wants == [ "other-link.socket" ]
        && lib.hasInfix "--realm home node --ssh one --serves /home/someone/.config/mycelium/serves-home.json" (
          toString bothLinux.mycelium.Service.ExecStart
        )
        && bothLinux.mycelium.Service.Environment == [ "SSH_AUTH_SOCK=%t/agent-signing.sock" ]
        && lib.hasInfix "/.local/state/pi/agentic-harness.pi/realms/other/steer --ssh-option " (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && lib.hasInfix " --ssh two --serves " (
          toString (mac both).launchd.agents.mycelium-other.config.ProgramArguments
        )
        && (linux both).home.sessionVariables.PI_HOME_REALM == "home"
        && (linux both).home.sessionVariables.MYCELIUM_REALM == "home"
      );

  # Each realm's node serves the harness's files of its realm, as the
  # harness keeps them: the home realm's in the harness's usual folders, a
  # further realm's in its own, and the machine's waiting and verdicts on
  # the home realm's node alone, so nothing is heard twice.
  # The node wakes with every session it carries, so it runs in their
  # slice, every realm's alike, rather than among Joel's programs.
  # The swarm's teams complete in a real zsh, asked of a stand-in node
  # (tests/completion.zsh).
  mycelium-completes-its-teams-in-zsh =
    pkgs.runCommandLocal "mycelium-completes-its-teams-in-zsh" { nativeBuildInputs = [ pkgs.zsh ]; }
      ''
        export HOME=$PWD
        zsh -f ${./tests/completion.zsh} ${./completion.zsh}
        touch $out
      '';

  mycelium-runs-among-the-sessions =
    holds "mycelium-runs-among-the-sessions"
      "A mycelium node runs outside agents.slice, where its wakeups preempt Joel's programs."
      (
        lib.all (node: (node.Service.Slice or null) == "agents.slice") [
          bothLinux.mycelium
          bothLinux.mycelium-other
        ]
      );

  mycelium-serves-the-harness-files =
    holds "mycelium-serves-the-harness-files"
      "A realm's node doesn't serve its realm's open and ended quests, restore list, runs, landings and talks from that realm's folders, the machine's waiting, verdicts, queue and the void's answers aren't served by the home realm's node alone, or a node doesn't restart when what it serves changes."
      (
        let
          state = "/home/someone/.local/state";
          serves = realm: builtins.fromJSON (linux both).xdg.configFile."mycelium/serves-${realm}.json".text;
        in
        serves "home" == {
          quest = {
            file = "${state}/pi/agentic-harness.pi/quest-workflow/quests.json";
            records = "quests";
            key = "id";
          };
          ended-quest = {
            file = "${state}/pi/agentic-harness.pi/quest-workflow/ended.json";
            records = "quests";
            key = "id";
          };
          restore = {
            file = "${state}/pi/restore.json";
            records = "sessions";
            key = "id";
          };
          run = {
            folder = "${state}/pi/agentic-harness.pi/mesh-workflow/runs";
            records = "runs";
            key = "id";
          };
          landing = {
            file = "${state}/pi/agentic-harness.pi/mesh-workflow/landings.json";
            records = "landings";
            key = "key";
          };
          talk.talks = "${state}/pi/agentic-harness.pi/steer";
          waiting = {
            folder = "${state}/mission/waiting";
            records = "items";
            key = "id";
            by = "quest";
          };
          verdict = {
            file = "${state}/mission/verdicts.json";
            records = "items";
          };
          job = {
            file = "jobs/queue.json";
            within = "runtime";
            records = "jobs";
            key = "id";
          };
          queue = {
            file = "jobs/queue.json";
            within = "runtime";
            whole = true;
          };
          void-answers = {
            file = "${state}/void/answers.jsonl";
            whole = true;
          };
        }
        &&
          serves "other" == {
            quest = {
              file = "${state}/pi/agentic-harness.pi/realms/other/quest-workflow/quests.json";
              records = "quests";
              key = "id";
            };
            ended-quest = {
              file = "${state}/pi/agentic-harness.pi/realms/other/quest-workflow/ended.json";
              records = "quests";
              key = "id";
            };
            restore = {
              file = "${state}/pi/agentic-harness.pi/realms/other/restore.json";
              records = "sessions";
              key = "id";
            };
            run = {
              folder = "${state}/pi/agentic-harness.pi/realms/other/mesh-workflow/runs";
              records = "runs";
              key = "id";
            };
            landing = {
              file = "${state}/pi/agentic-harness.pi/realms/other/mesh-workflow/landings.json";
              records = "landings";
              key = "key";
            };
            talk.talks = "${state}/pi/agentic-harness.pi/realms/other/steer";
          }
        && lib.hasSuffix "--serves /home/someone/.config/mycelium/serves-other.json" otherNode
        && !(apart.xdg.configFile ? "mycelium/serves-home.json")
        # The node reads its serves file only as it starts, so a switch
        # changing what it serves restarts it.
        &&
          lib.all
            (
              one:
              one.node.Unit.X-Restart-Triggers == [
                (builtins.hashString "sha256" (linux both).xdg.configFile."mycelium/serves-${one.realm}.json".text)
              ]
            )
            [
              {
                node = bothLinux.mycelium;
                realm = "home";
              }
              {
                node = bothLinux.mycelium-other;
                realm = "other";
              }
            ]
      );

  # A machine that declares what its node can't measure has it written where
  # mycelium reads it, in mycelium's words; Linux's held sessions outlive
  # their window, the node and a suspend, and a Mac claims none yet. A fenced
  # or strict fence on Linux starts an agent's run through the fence's
  # launcher; an unfenced one and a Mac's say none, so mycelium refuses it.
  # One that declares nothing has no file, which mycelium reads as unknown.
  mycelium-declares-what-a-machine-says =
    holds "mycelium-declares-what-a-machine-says"
      "What a machine declares, its probes of either kind included, isn't written in mycelium's shape, a held session's outlives isn't each system's, a fence it doesn't say is written, an agent's run in a fenced or strict fence on Linux isn't said to start through the fence's launcher, an unfenced one or a Mac's is, or a machine declaring nothing gets a file anyway."
      (
        declared linux == {
          reach = [ "internet" ];
          probes.home.connect = "192.0.2.1:53";
          probes.office.run = declaring.probes.office;
          outlives = [
            "window_close"
            "node_restart"
            "sleep"
          ];
          costs_money = false;
          inherit (declaring) taints;
        }
        && (declared mac).outlives == [ ]
        && !((declared linux) ? within)
        && !((declared linux) ? hosts)
        && !((declared linux) ? fence)
        && (
          strictly.fence or null == {
            strict = {
              builders = [ "remote" ];
              forges = [ ];
              secrets = [ ];
              runs = launcher;
            };
          }
        )
        && (launching linux).fence.fence.runs or null == launcher
        && (launching linux).fence.strict.runs or null == launcher
        && !((launching linux).fence.none ? runs)
        && !((launching mac).fence.strict ? runs)
        && (
          inVm.within or null == {
            machine = "host-machine";
            file = "/run/host/said.json";
          }
        )
        && (
          hosting.hosts or null == [
            {
              machine = "guest-machine";
              file = "/var/host/said.json";
            }
          ]
        )
        && !(apart.xdg.configFile ? "mycelium/declared.json")
        &&
          (builtins.fromJSON (saying { home = "an-agent"; }).xdg.configFile."mycelium/declared.json".text)
          .agents == {
            home = "an-agent";
          }
        && !refusesAgents (saying {
          home = "an-agent";
        })
        && refusesAgents (saying {
          elsewhere = "an-agent";
        })
      );

  # A machine's system pins each key that links to it, on either system, to
  # its own realm and machine, with a terminal for an attach; one naming
  # none authorizes nothing of mycelium's.
  mycelium-lets-a-link-key-only-link =
    holds "mycelium-lets-a-link-key-only-link"
      "A link key a machine names isn't pinned to `mycelium ssh` as the realm and machine it names, on NixOS or nix-darwin, can't have a terminal for an attach, or a machine naming none authorizes a key."
      (
        let
          key = "ecdsa-sha2-nistp256 AAAAone";
          pinning = {
            account = "someone";
            linkedFrom.personal.one = key;
          };
          keys =
            bare: role: settings:
            (bare [
              role
              { jitsusama.mycelium = settings; }
            ]).config.users.users.someone.openssh.authorizedKeys.keys;
          pinnedOn =
            bare: role:
            let
              given = keys bare role pinning;
            in
            builtins.length given == 1
            && lib.hasPrefix ''restrict,pty,command="/nix/store/'' (lib.head given)
            && lib.hasSuffix ''/bin/mycelium --realm personal ssh --from one" ${key}'' (lib.head given);
        in
        pinnedOn bareNixos self.nixosModules.workstation
        && pinnedOn bareDarwin self.darwinModules.workstation
        && keys bareNixos self.nixosModules.workstation { } == [ ]
      );
}
