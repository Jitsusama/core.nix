# The checks of pipewire, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  graphical,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
{
  # A call's audio thread asks for SCHED_RR 8 in Chrome and Electron apps,
  # and gets it only if the account's limit reaches 8. At 11 or more,
  # PipeWire takes its priority from the limit rather than rtkit's 20, so
  # anything an app sets could rank beside it. With no time limit, a
  # real-time thread that spins is never stopped. None of these says so.
  call-audio-runs-in-real-time =
    let
      inherit (graphical.config) systemd;
      manager = systemd.services."user@".serviceConfig;
      units = systemd.user.settings.Manager;
      priority = lib.toInt manager.LimitRTPRIO;
    in
    holds "call-audio-runs-in-real-time"
      "The account can't take real-time priority 8, can take 11 or more, or has no real-time time limit."
      (
        priority >= 8
        && priority < 11
        && (manager.LimitRTTIME or "infinity") != "infinity"
        && (units.DefaultLimitRTPRIO or null) == manager.LimitRTPRIO
        && (units.DefaultLimitRTTIME or null) == manager.LimitRTTIME
      );

  # The sound services sound-services.nix chooses from stand-in units, each
  # half of its choice alone: a NixOS unit or home-manager's that runs a
  # PipeWire of its own, wherever it sits, as one command, one line of
  # several after a reset, or with a prefix systemd takes, and one PipeWire
  # wants in session.slice, by a list or, as home-manager allows, one name,
  # while one PipeWire wants in app.slice, one that runs another program
  # whose path begins the same or names PipeWire past its program, and a
  # NixOS unit switched off are none, and a home's are its account's alone.
  # Read as sets, since the order they're chosen in grants nothing.
  sound-services-are-what-runs-a-pipewire-or-sits-beside-it =
    let
      pipewire = "/nix/store/x-pipewire/bin/pipewire";
      nixos = start: wantedBy: slice: {
        serviceConfig = {
          ExecStart = start;
        }
        // lib.optionalAttrs (slice != null) { Slice = slice; };
        inherit wantedBy;
      };
      home = start: wantedBy: slice: {
        Service = {
          ExecStart = start;
        }
        // lib.optionalAttrs (slice != null) { Slice = slice; };
        Install.WantedBy = wantedBy;
      };
      chosen = import ./sound-services.nix { inherit lib; } {
        inherit pipewire;
        system = {
          chain = nixos "${pipewire} -c chain.conf" [ ] "app.slice";
          reset = nixos [ "" "${pipewire} -c reset.conf" ] [ ] "app.slice";
          prefixed = nixos "-${pipewire} -c prefixed.conf" [ ] "app.slice";
          passed = nixos "/bin/env ${pipewire} -c passed.conf" [ ] "app.slice";
          beside = nixos "/bin/beside" [ "pipewire.service" ] "session.slice";
          watcher = nixos "/bin/watcher" [ "pipewire.service" ] "app.slice";
          unsliced = nixos "/bin/unsliced" [ "pipewire.service" ] null;
          pulse = nixos "${pipewire}-pulse" [ ] "session.slice";
          off = nixos "${pipewire} -c off.conf" [ ] "session.slice" // {
            enable = false;
          };
        };
        homes = {
          one = {
            own = home "${pipewire} -c own.conf" [ ] null;
            cancelled = home "/bin/cancelled" [ "pipewire.service" ] "session.slice";
            named = home "/bin/named" "pipewire.service" "session.slice";
            paused = home "/bin/paused" [ "pipewire.service" ] null;
          };
          other = { };
        };
      };
    in
    holds "sound-services-are-what-runs-a-pipewire-or-sits-beside-it"
      "sound-services.nix chose ${builtins.toJSON chosen}, not by what each runs and where."
      (
        lib.mapAttrs (_: lib.sort lib.lessThan) chosen.homes == {
          one = [
            "cancelled"
            "named"
            "own"
          ];
          other = [ ];
        }
        &&
          lib.sort lib.lessThan chosen.system == [
            "beside"
            "chain"
            "pipewire"
            "pipewire-pulse"
            "prefixed"
            "reset"
            "wireplumber"
          ]
      );

  # nixos.nix chooses from what the machine runs: the PipeWire it's given,
  # so a stand-in's program is a sound service where nixpkgs' isn't, and a
  # home by the login name polkit asks with, where it differs from the
  # home's attribute.
  sound-services-are-read-from-the-machine =
    let
      standIn = pkgs.runCommandLocal "a-pipewire" { } "mkdir -p $out/bin";
      machine = bareNixos [
        self.nixosModules.pipewire
        {
          services.pipewire.package = standIn;
          systemd.user.services = {
            ours.serviceConfig.ExecStart = "${standIn}/bin/pipewire -c ours.conf";
            nixpkgs.serviceConfig.ExecStart = "${pkgs.pipewire}/bin/pipewire -c theirs.conf";
          };
          # The account logs in as joel, which its home takes for its own.
          users.users.someone.name = "joel";
          home-manager.users.someone.systemd.user.services.own.Service.ExecStart =
            "${standIn}/bin/pipewire -c own.conf";
        }
      ];
      rules = machine.config.security.polkit.extraConfig;
      found = lib.filter lib.isList (builtins.split "var soundServices = ([^\n]*);" rules);
      chosen = if found == [ ] then null else builtins.fromJSON (lib.head (lib.head found));
    in
    holds "sound-services-are-read-from-the-machine"
      "nixos.nix chose ${builtins.toJSON chosen}, not by the machine's PipeWire and login names."
      (
        chosen != null
        && lib.elem "ours" chosen.system
        && !lib.elem "nixpkgs" chosen.system
        &&
          chosen.homes == {
            joel = [ "own" ];
          }
      );

  # The graphical role's polkit rules, with its tuning chains and
  # noise-cancelled, as polkitd assembles them, run in duktape, polkit's own
  # engine, after polkit's init.js, with an account's manager asking before
  # the seat has a session and after: PipeWire's three units, each chain and
  # each service PipeWire wants in session.slice, by name in session.slice
  # or app.slice, are given real time and a high priority from rtkit, a
  # home's service for its own account alone, and PipeWire's grant is logged
  # with what it runs, one ps can't read as unknown, while anything else of
  # the account's, a watcher in app.slice among them, keeps rtkit's own
  # answer, leaving no grant's line, and agents' places are still refused
  # (realtime-test.rules). The slices' stand-in for
  # polkitd serves here too. No chain's name holds a dot today, so one is put
  # among the names, to show a dot stands for itself. Every placeholder is
  # filled, since a cat left unnamed would refuse sound in silence.
  sound-takes-real-time-before-the-seat =
    let
      assembled = graphical.config.environment.etc."polkit-1/rules.d/10-nixos.rules".text;
      dotted = ''"wireplumber","tuning.v2"'';
      rules = builtins.replaceStrings [ ''"wireplumber"'' ] [ dotted ] assembled;
      unfilled = lib.concatMap (lib.take 1) (
        builtins.filter builtins.isList (builtins.split "(@[A-Za-z]+@)" assembled)
      );
    in
    assert lib.assertMsg (
      unfilled == [ ]
    ) "polkit's 10-nixos.rules holds ${lib.concatStringsSep ", " unfilled} unfilled";
    assert lib.assertMsg (rules != assembled) "realtime.rules no longer names its services as JSON";
    pkgs.runCommandLocal "sound-takes-real-time-before-the-seat" { } ''
      ${lib.getExe' pkgs.duktape "duk"} ${../slices/agents-ask-polkit.rules} \
        ${pkgs.polkit.src}/src/polkitbackend/init.js \
        ${pkgs.writeText "10-nixos.rules" rules} ${./realtime-test.rules}
      touch $out
    '';
}
