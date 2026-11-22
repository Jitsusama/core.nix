# The checks of pineapple, which checks.nix gathers with every other
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
  ...
}:
onLinux {
  # Its tests run as it builds: every saying in sayings.json, the real
  # messages of 13 November and 20 October among them, is judged the safe
  # word, a resume, an ask to stop or a near miss (no tag, a quote, talked
  # about, a joke, a bot) as the table says, and reaches what its words
  # and the place it was said in name, or everything, said so; and Slack's
  # answers become messages, oldest first, a bot's told apart, with the
  # thread each is in, and a stop's record is written whole and once; and
  # pineapple, Joel's own command, lists the stops standing, refuses a
  # resume where pi says it runs, and otherwise marks what a scope names
  # resumed, which clears its thread's reply; and pineapple serve serves
  # a live stop and alert as rows in the launcher's words, writes a shadow
  # one down only, and refuses a press from anything but Joel's shell;
  # and pineapple stop all, refused under pi, stops everything, refusing
  # agents' starts, stopping their timers and freezing their work but
  # never Joel's, here and on each machine running a session, until a
  # resume undoes exactly that.
  pineapple-judges-each-saying = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pineapple, its tests included.
  pineapple-passes-clippy = passesClippy "pineapple-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # pineapple-slack starts at login, reads pi's Slack sign-in from the
  # keyring as slack-unreads does, comes back as the new one after a
  # switch and after it fails, runs in the background slice, reads as its
  # own name in ps, runs live, and takes its row in For You with it as it
  # stops.
  pineapple-slack-starts-with-the-sign-in =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple-slack;
      waiting = "${home.xdg.stateHome}/mission/waiting/pineapple.json";
    in
    holds "pineapple-slack-starts-with-the-sign-in"
      "pineapple-slack lacks its login start, keyring, switch restart, restart, slice, name, --live, row or row cleanup."
      (
        lib.elem "default.target" unit.Install.WantedBy
        && !(home.systemd.user.paths ? pineapple-slack)
        && lib.any (lib.hasPrefix "SIGN_IN_KEYRING=/nix/store/") (lib.toList unit.Service.Environment)
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        # As home-manager takes ExecStart, each start is listed.
        && lib.any (lib.hasSuffix "/bin/pineapple-slack pineapple-slack run --live") (
          lib.toList unit.Service.ExecStart
        )
        # The program is handed the file its stop removes.
        && lib.elem ''"SAFE_WORD_WAITING=${waiting}"'' (lib.toList unit.Service.Environment)
        && lib.hasInfix "/bin/rm -f " unit.Service.ExecStopPost
        && lib.all (row: lib.hasInfix row unit.Service.ExecStopPost) [
          waiting
          "${dirOf waiting}/.writing/${baseNameOf waiting}"
        ]
      );
  # pineapple-github starts at login, reads Joel's GitHub sign-in through
  # gh, comes back as the new one after a switch and after it fails, runs
  # in the background slice, reads as its own name in ps, and runs live,
  # where only a repository's own stop Joel (src/github.rs).
  pineapple-github-starts-with-the-sign-in =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple-github;
    in
    holds "pineapple-github-starts-with-the-sign-in"
      "pineapple-github lacks its login start, sign-in, switch restart, restart, slice, name or --live."
      (
        lib.elem "default.target" unit.Install.WantedBy
        && lib.any (lib.hasPrefix "SIGN_IN_KEYRING=/nix/store/") (lib.toList unit.Service.Environment)
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        && lib.any (lib.hasSuffix "/bin/pineapple-github pineapple-github run --live") (
          lib.toList unit.Service.ExecStart
        )
      );
  # Each other site a machine names is read by its own pineapple-NAME,
  # the GitHub reader searching the repositories named for what Joel
  # opened, with that site's sign-in, started as GitHub's is; none is read
  # unless named, and a name GitHub's or Slack's reader has is refused.
  pineapple-reads-each-site-named =
    let
      withSites =
        sites:
        account (bareNixos [
          {
            home-manager.sharedModules = [
              self.homeModules.pineapple
              { jitsusama.pineapple.sites = sites; }
            ];
          }
        ]);
      code = {
        origin = "https://code.example.com/api/v3";
        authoredBy = "joel@example.com";
        login = "joel.example";
        repos = [
          "example/shop"
          "example/other"
        ];
        signIn = pkgs.writeShellScript "stand-in-sign-in" "echo stand-in-token";
      };
      home = withSites { inherit code; };
      unit = home.systemd.user.services.pineapple-code;
      command = lib.concatStringsSep " " (lib.toList unit.Service.ExecStart);
      live =
        (withSites {
          code = code // {
            live = true;
          };
        }).systemd.user.services.pineapple-code;
      liveCommand = lib.concatStringsSep " " (lib.toList live.Service.ExecStart);
      readers = home: lib.filter (lib.hasPrefix "pineapple-") (lib.attrNames home.systemd.user.services);
      refused = home: lib.any (check: !check.assertion) home.assertions;
    in
    holds "pineapple-reads-each-site-named"
      "A named site lacks its reader, search, sign-in, start or shadow, one said live isn't, one is read unnamed, or a taken name or more than 20 repositories pass."
      (
        lib.hasSuffix (
          "/bin/pineapple-github pineapple-code run --origin https://code.example.com/api/v3"
          + " --site code --authored-by joel@example.com --login joel.example"
          + " --repo example/shop --repo example/other"
        ) command
        && lib.hasSuffix ", in shadow" unit.Unit.Description
        && lib.hasInfix " pineapple-code run --live --origin " liveCommand
        && !(lib.hasInfix "in shadow" live.Unit.Description)
        && lib.elem "SIGN_IN_KEYRING=${code.signIn}" (lib.toList unit.Service.Environment)
        && lib.elem "default.target" unit.Install.WantedBy
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        && !(refused home)
        &&
          readers (withSites { }) == [
            "pineapple-github"
            "pineapple-slack"
          ]
        && refused (withSites {
          github = code;
        })
        && refused (withSites {
          code = code // {
            repos = map (n: "example/repo${toString n}") (lib.range 1 21);
          };
        })
      );
  # A machine naming a tool gateway hears Slack through it with a second
  # reader, started as Slack's is but only ever in shadow, keeping its
  # own state and telling For You in a file there that nothing reads;
  # and a machine naming none has no such reader.
  pineapple-slack-hears-a-gateway-in-shadow =
    let
      withGateway =
        gateway:
        account (bareNixos [
          {
            home-manager.sharedModules = [
              self.homeModules.pineapple
              { jitsusama.pineapple.gateway = gateway; }
            ];
          }
        ]);
      signIn = pkgs.writeShellScript "stand-in-sign-in" "echo stand-in-token";
      at = url: withGateway { inherit url signIn; };
      home = at "https://gateway.example.com/mcp";
      unit = home.systemd.user.services.pineapple-slack-gateway;
      command = lib.concatStringsSep " " (lib.toList unit.Service.ExecStart);
      percent = (at "https://gateway.example.com/mcp?x=a%2Fb").systemd.user.services;
      encoded = lib.concatStringsSep " " (
        lib.toList percent.pineapple-slack-gateway.Service.ExecStart
      );
      refused = (at "http://gateway.example.com/mcp").assertions;
    in
    holds "pineapple-slack-hears-a-gateway-in-shadow"
      "The gateway's reader lacks its command, shadow or start, moves the state its sign-in reads, mangles a URL's %, takes plain http, or runs unnamed."
      (
        lib.hasInfix (
          "/bin/pineapple-slack pineapple-slack-gateway run"
          + " --gateway https://gateway.example.com/mcp --gateway-sign-in /nix/store/"
        ) command
        && !(lib.hasInfix "--live" command)
        && lib.hasSuffix ", in shadow" unit.Unit.Description
        && !(unit.Service ? Environment)
        && lib.hasInfix "mcp?x=a%%2Fb" encoded
        && lib.any (said: !said.assertion && lib.hasInfix "gateway.url" said.message) refused
        && lib.elem "default.target" unit.Install.WantedBy
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        && !((withGateway null).systemd.user.services ? pineapple-slack-gateway)
      );
  # pineapple, the daemon showing stops in the launcher, is started at
  # login for its socket, which systemd holds where the launcher looks,
  # knocks with notify-send, and reads its words from the launcher's
  # vocabulary; and its Pause All Agents has a plan naming systemctl,
  # mycelium and the agents' timers, keeping the compile cache.
  pineapple-serves-the-launcher-from-its-socket =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple;
      socket = home.systemd.user.sockets.pineapple;
      command = lib.concatStringsSep " " (lib.toList unit.Service.ExecStart);
      # Read for its shape only, so the store paths it names needn't build.
      pause = builtins.fromJSON (
        builtins.unsafeDiscardStringContext home.xdg.configFile."pineapple/pause.json".text
      );
    in
    holds "pineapple-serves-the-launcher-from-its-socket"
      "pineapple lacks its socket, login start, serve command, notice, words or pause plan."
      (
        socket.Socket.ListenStream == "%t/daemons/pineapple.sock"
        && lib.elem "pineapple.socket" unit.Unit.Requires
        && lib.elem "default.target" unit.Install.WantedBy
        && lib.hasInfix "/bin/pineapple pineapple serve --socket" command
        && lib.hasInfix "/bin/notify-send" command
        && lib.hasInfix "words.json" command
        && lib.hasSuffix "/bin/systemctl" pause.systemctl
        && lib.hasSuffix "/bin/mycelium" pause.mycelium
        && lib.elem "cadence*.timer" pause.timers
        && lib.elem "sccache.service" pause.keep
        && lib.elem "merge-net.service" pause.keep
        && builtins.isList pause.realms
      );
}
