# The checks of a Mac's local name, which checks.nix gathers with every other
# folder's; each says what it proves.
{
  bareDarwin,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # A local name of its own, apart from the host name nix-darwin would
  # derive it from, so a job reading the host name instead is caught.
  itsOwnName = {
    networking.hostName = "a-mac";
    networking.localHostName = "its-own-name";
  };
  noName = {
    networking.hostName = "a-mac";
    networking.localHostName = null;
  };
  # Each Mac evaluated once, with the module and without it, since an
  # evaluation is most of what a check costs.
  macWith = modules: (bareDarwin modules).config;
  # The same Mac without the module, whose packages no overlay of the
  # module's can reach.
  bareNamed = bareDarwin [ itsOwnName ];
  namedSystem = bareDarwin [
    self.darwinModules.local-name
    itsOwnName
  ];
  namedMac = namedSystem.config;
  unnamedMac = macWith [
    self.darwinModules.local-name
    noName
  ];
  # The one file the module puts in /etc, which has newsyslog move the jobs'
  # logs aside. nix-darwin hashes every file it links there into its
  # activation's checks, which the activate-system job carries whole, so
  # adding it changes both; the same Mac with it turned off shows what else
  # the module changes, and the file is held on its own.
  rotation = "newsyslog.d/local-name.conf";
  quietMac = macWith [
    self.darwinModules.local-name
    itsOwnName
    { environment.etc.${rotation}.enable = lib.mkForce false; }
  ];
  # Every launchd job a Mac's system writes, as the file it links, by file
  # name: nix-darwin lowers its launchd options into these, and takes raw
  # plists here too, a forced source winning over the text. Beside them,
  # the agents home-manager gives the Mac's one account and any file it
  # puts in the account's LaunchAgents; whether Remote Login is turned on,
  # which writes no plist; the environment launchd gives every job, which
  # nix-darwin's launchd activation sets, so a BASH_ENV there shows; every
  # file linked in /etc; and the activation scripts but nix-darwin's
  # launchd ones, whose text names every plist.
  jobsOf =
    config:
    let
      plists = lib.mapAttrs (
        _: file:
        builtins.toJSON {
          inherit (file) enable target;
          source = "${file.source}";
        }
      );
      account = config.home-manager.users.someone;
      launchAgentFiles = lib.filterAttrs (
        _: file: lib.hasPrefix "Library/LaunchAgents/" file.target
      ) account.home.file;
    in
    {
      daemons = plists config.environment.launchDaemons;
      agents = plists config.environment.launchAgents;
      userAgents = plists config.environment.userLaunchAgents;
      accountAgents = lib.mapAttrs (_: agent: builtins.toJSON agent.config) (
        lib.filterAttrs (_: agent: agent.enable) account.launchd.agents
      );
      accountFiles = lib.mapAttrs' (
        _: file:
        lib.nameValuePair file.target (
          builtins.toJSON [
            file.enable
            "${file.source}"
          ]
        )
      ) launchAgentFiles;
      remoteLogin."Remote Login" = builtins.toJSON (config.services.openssh.enable or null);
      environment = {
        "launchd's environment" = builtins.toJSON config.launchd.envVariables;
        "launchd's account environment" = builtins.toJSON config.launchd.user.envVariables;
      };
      etc = lib.mapAttrs (
        _: file:
        builtins.toJSON {
          inherit (file) target;
          source = "${file.source}";
        }
      ) (lib.filterAttrs (_: file: file.enable) config.environment.etc);
      activation = lib.mapAttrs (_: script: script.text) (
        removeAttrs config.system.activationScripts [
          "launchd"
          "userLaunchd"
          "script"
        ]
      );
    };
  # What the module adds or changes on a Mac, beside the same Mac without
  # it, so a job added any of those ways, or a key added to one the Mac
  # already has, shows by name. Any of them could advertise for the host,
  # through dns-sd, a script of its own, launchd's own Bonjour key or
  # Remote Login, and the module's header says why none may.
  changedBy =
    without: mac:
    let
      before = jobsOf without;
      after = jobsOf mac;
    in
    lib.concatLists (
      lib.mapAttrsToList (
        kind: files:
        lib.filter (file: before.${kind}.${file} or null != files.${file}) (lib.attrNames files)
      ) after
    );
  changedNamed = changedBy bareNamed.config quietMac;
  changedUnnamed = changedBy (macWith [ noName ]) unnamedMac;
  # The files in /etc the module adds or changes, which must be the rotation
  # alone, linked where its name says, written from its own text as held,
  # which names the two logs and sends no signal. nix-darwin names a file's
  # text for the last part of its name.
  etcChanged =
    let
      before = (jobsOf bareNamed.config).etc;
    in
    lib.filter (file: before.${file} or null != (jobsOf namedMac).etc.${file}) (
      lib.attrNames (jobsOf namedMac).etc
    );
  rotates =
    let
      written = namedMac.environment.etc.${rotation} or { };
      held = ''
        # logfilename                    mode count size(KB) when flags
        /var/log/local-name.log          644  3     512      *    N
        /var/log/local-name-answers.log  644  3     512      *    N
      '';
    in
    etcChanged == [ rotation ]
    && (written.target or null) == rotation
    && (written.text or null) == held
    && "${written.source or ""}" == "${bareNamed.pkgs.writeText "etc-local-name.conf" held}";
  # The Mac's own bash, by its store path on the Mac without the module, so
  # a wrapper named bash that sets BASH_ENV or runs anything first isn't
  # excused as the interpreter, an overlay's included. The message names
  # it, since a wrapper's path differs from it by its hash alone.
  bash = lib.getExe bareNamed.pkgs.bash;
  # A job of the module's, held whole beside the plist it writes, as
  # written, not as its options say. Each way the plist could differ from
  # the job is told apart, so a failure says which: turned off, which
  # launchd never loads, since nix-darwin links only plists enabled; text
  # other than what the job's options lower to, as text forced beside them;
  # or a file other than that text's own, made with the Mac's packages
  # without the module, so an overlay of the module's on writeText can't
  # make both alike; or linked as a file of another name, which could
  # stand in for another job's plist. The lowering is nix-darwin's own, its launchd module's
  # toEnvironmentText, so a bump that renders differently fails here as
  # other text, and that function says what it now is; the message says so.
  # Every setting is held, its label included, so a Program, an environment
  # variable such as BASH_ENV, a user or any key launchd reads shows, not
  # only those the check thought of. The plist's file is named for the label
  # nix-darwin gives, not the one the job has: launchd enables a job by its
  # label, so a job labelled as Remote Login's would turn Remote Login on.
  plistOf =
    name:
    let
      job = namedMac.launchd.daemons.${name}.serviceConfig or { };
      file = "org.nixos.${name}.plist";
      written = namedMac.environment.launchDaemons.${file} or { };
      fromText = bareNamed.pkgs.writeText file written.text or "";
    in
    {
      inherit file;
      held = lib.filterAttrs (_: value: value != null) job;
      enabled = written.enable or false;
      target = (written.target or null) == file;
      text = (written.text or null) == lib.generators.toPlist { escape = true; } job;
      source = "${written.source or ""}" == "${fromText}";
      socket =
        job.Sockets or null != null
        || lib.hasInfix "Sockets" written.text or ""
        || lib.hasInfix "Bonjour" written.text or "";
    };
  keeping = plistOf "local-name";
  answering = plistOf "local-name-answers";
  keeps =
    keeping.held == {
      Label = "org.nixos.local-name";
      ProgramArguments = [
        bash
        "${./keep.sh}"
        "its-own-name"
        "/var/db/local-name.last"
      ];
      EnvironmentVariables.PATH = "/usr/sbin:/usr/bin:/bin";
      WatchPaths = [ "/Library/Preferences/SystemConfiguration/preferences.plist" ];
      StartInterval = 60;
      RunAtLoad = true;
      StandardErrorPath = "/var/log/local-name.log";
    };
  answers =
    answering.held == {
      Label = "org.nixos.local-name-answers";
      ProgramArguments = [
        bash
        "${./answer.sh}"
        "its-own-name"
        "15"
      ];
      EnvironmentVariables.PATH = "/usr/sbin:/usr/bin:/bin";
      KeepAlive = true;
      RunAtLoad = true;
      StandardErrorPath = "/var/log/local-name-answers.log";
    };
  # The ways a script could advertise the host that the check knows:
  # dns-sd; launchctl, which loads Remote Login's or Screen Sharing's job by
  # its plist alone; systemsetup's remote switches; Remote Management's
  # kickstart; and the sharing command, by its name alone, since every one
  # of its flags changes what the Mac shares. Each script is read in any
  # case. Another way, or a disguised spelling, isn't seen.
  advertisingIn =
    script:
    lib.filter (word: lib.hasInfix word (lib.toLower (builtins.readFile script))) [
      "dns-sd"
      "launchctl"
      "setremotelogin"
      "setremoteappleevents"
      "com.openssh.sshd"
      "ssh.plist"
      "screensharing"
      "kickstart"
      "ardagent"
      "remotemanagement"
      "sharing"
    ];
  # keep.sh names none of them; answer.sh names only dns-sd, which it runs
  # in one line alone, answering the name as a host of its own (-P), so no
  # service names the Mac's own host, on one link (-i). dns-sd -R, or any
  # service without a host of its own, would bring the Mac's records back on
  # every interface. Its lines are read with its comments left out.
  keepAdvertises = advertisingIn ./keep.sh;
  answerAdvertises = lib.remove "dns-sd" (advertisingIn ./answer.sh);
  answerRunsDnsSd = lib.filter (line: lib.hasInfix "dns-sd" (lib.toLower line)) (
    lib.filter (line: !lib.hasPrefix "#" (lib.trim line)) (
      lib.splitString "\n" (builtins.readFile ./answer.sh)
    )
  );
  answersAsItsOwnHost =
    answerRunsDnsSd == [
      ''dns-sd -i "''${link}" -P "''${name}" _workstation._tcp local 9 "''${name}.local" "''${address}" >/dev/null &''
    ];
  namesOr = words: if words == [ ] then "none of them" else lib.concatStringsSep ", " words;
  filesOr = files: if files == [ ] then "nothing" else lib.concatStringsSep ", " files;
  yesNo = holding: if holding then "yes" else "no";
  # Each job's plist, as the message says it.
  told =
    plist:
    "on: ${yesNo plist.enabled}; linked as its own file: ${yesNo plist.target}; the text its options lower to: ${yesNo plist.text}; written from that text: ${yesNo plist.source}; opening a socket: ${yesNo plist.socket}";
  writtenAsHeld = plist: plist.enabled && plist.target && plist.text && plist.source && !plist.socket;
  # Only the two jobs' own plists, on a Mac with a local name, beside the
  # rotation, and nothing on one without; each linked as its own file and
  # written from its own text. Holding each job whole and its text already
  # keeps out launchd's Sockets and Bonjour keys; the socket test says so
  # apart, in case either hold is loosened.
  mayAdvertise =
    lib.sort lib.lessThan changedNamed != lib.sort lib.lessThan [
      keeping.file
      answering.file
    ]
    || changedUnnamed != [ ]
    || !rotates
    || !writtenAsHeld keeping
    || !writtenAsHeld answering
    || keepAdvertises != [ ]
    || answerAdvertises != [ ]
    || !answersAsItsOwnHost;
in
{
  # A Mac with a local name runs two root jobs, each labelled as nix-darwin
  # labels it, under the Mac's own bash, finding its tools only in macOS's
  # own folders, saying what it does in its log and given nothing else:
  # keep.sh with that name and its stamp, from its start, whenever macOS
  # writes its network settings and once a minute; and answer.sh with that
  # name, reading every fifteen seconds, started again whenever it ends. A
  # Mac with a host name but no local name runs neither. The module adds or
  # changes no other launchd job, nix-darwin's or the account's, nor Remote
  # Login, launchd's environment, an activation script or a file in /etc
  # but the rotation of the jobs' logs, held as written; each plist is on,
  # is linked as its own file, is what its options lower to, is written
  # from that text and opens no socket; keep.sh names, in bash's lowercase,
  # none of the ways to advertise the host listed above, and answer.sh only
  # dns-sd, run in one line alone answering the name as a host of its own
  # on one link. It doesn't see another module's jobs, a way that list
  # doesn't know or a deliberately disguised spelling, or a line added to
  # nix-darwin's own launchd activation or to home-manager's.
  # Its message quotes the jobs' store paths as text alone, so a red check
  # on Linux needn't build the Mac's packages it names to say so.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh, and answered by one running answer.sh, each under bash with its local name, finding its tools only in macOS's own folders, saying what it does in its log and given nothing else, or a Mac with no local name runs one, or the module adds or changes another launchd job, Remote Login, launchd's environment, an activation script or a file in /etc other than the logs' rotation as held, turns a plist off, links it as another file, writes it from elsewhere or from other text, opens a socket, names in keep.sh a way to advertise the host, or has answer.sh run dns-sd other than as a host of its own on one link, any of which could advertise for the host. What it adds or changes with a local name, its rotation turned off: ${filesOr changedNamed}; without one: ${filesOr changedUnnamed}. The files in /etc it adds or changes: ${filesOr etcChanged}, the rotation as held: ${yesNo rotates}. Its keeping job is as held: ${yesNo keeps}, given ${builtins.unsafeDiscardStringContext (builtins.toJSON keeping.held)}; its answering job: ${yesNo answers}, given ${builtins.unsafeDiscardStringContext (builtins.toJSON answering.held)}; their bash to be ${builtins.unsafeDiscardStringContext bash}. A text is lowered here as nix-darwin did when this was written, so a no for a job or its text after a nix-darwin update may be nix-darwin rendering it differently or adding a key of its own. The keeping job's plist is ${told keeping}. The answering job's plist is ${told answering}. Of the ways to advertise the host, keep.sh names ${namesOr keepAdvertises}, and answer.sh, beside dns-sd, ${namesOr answerAdvertises}. answer.sh runs dns-sd only as a host of its own on one link: ${yesNo answersAsItsOwnHost}, in ${namesOr answerRunsDnsSd}."
      (!mayAdvertise && keeps && answers);
}
# What follows runs the jobs' scripts under Linux's bash, with a stand-in
# scutil and dns-sd, since a Mac's sandbox can't set its own name or answer
# for it.
// onLinux {
  # A renamed Mac's name is set back and the rename said once, at most once
  # in five minutes by the time its stamp holds; a name already right is
  # left unwritten, one that can't be read is left alone, and what it
  # couldn't read or set is said at most once in five minutes
  # (tests/local-name-comes-back.sh).
  local-name-comes-back = pkgs.runCommandLocal "local-name-comes-back" { } ''
    ${lib.getExe pkgs.bash} ${./tests/local-name-comes-back.sh} ${./keep.sh} ${lib.getExe pkgs.bash}
    touch $out
  '';
  # The name answers the first wired or Wi-Fi link's address, past a tunnel
  # or a bridge, on that link alone and as a host of its own; a moved
  # address, another link or none ends it and its answer; with no link it
  # waits, saying so once; and an answer that ends by itself ends it
  # (tests/local-name-answers-on-its-link.sh).
  local-name-answers-on-its-link = pkgs.runCommandLocal "local-name-answers-on-its-link" { } ''
    ${lib.getExe pkgs.bash} ${./tests/local-name-answers-on-its-link.sh} \
      ${./answer.sh} ${lib.getExe pkgs.bash}
    touch $out
  '';
}
