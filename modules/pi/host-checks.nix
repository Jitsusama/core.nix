# The checks of pi's shared hosts, which pi's checks.nix gathers beside its
# own: the units host.nix renders for each realm and the proof each realm's pi
# needs, kept apart since they change with host.nix and nothing else.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # A runtime's proof as the harness's host-prove writes it, written without
  # building anything, so reading it back needs no build at evaluation.
  proofOf =
    fields:
    builtins.toFile "pi-host-proof.json" (
      builtins.toJSON (
        {
          kind = "pi-host-runtime-proof";
          version = 1;
          harness = "0000000000000000000000000000000000000000";
          files = 14;
          cases = 71;
          provenAt = "2026-11-10T11:00:00.000Z";
        }
        // fields
      )
    );
in
{
  # A realm a machine names gets two hosts, attended and background, started
  # with the account's session once the harness is installed, run by the
  # host's own pi and node, said ready by the host, among the agents'
  # sessions, left running through a switch, and said to be down when left
  # down; a machine that names none, and a Mac, get no host.
  pi-hosts-start-with-the-session =
    let
      pi = "${pkgs.emptyDirectory}/pi";
      node = "${pkgs.emptyDirectory}/node";
      proof = proofOf { runtime = builtins.unsafeDiscardStringContext "${pkgs.emptyDirectory}"; };
      hosts = {
        one = {
          inherit pi node proof;
          environment = {
            PROXY_ORIGIN = "https://proxy.example.com";
            GREETING = ''say "hi" there'';
          };
        };
        two = {
          inherit pi node proof;
          path = [
            "/first/bin"
            "/second/bin"
            "/100%/$HOME"
          ];
        };
      };
      naming =
        bare: role:
        account (bare [
          role
          { home-manager.users.someone.jitsusama.pi.hosts = hosts; }
        ]);
      services = (naming bareNixos self.nixosModules.workstation).systemd.user.services;
      mac = naming bareDarwin self.darwinModules.workstation;
      # home-manager keeps a unit's settings as lists.
      first = value: lib.head (lib.toList value);
      starts =
        kind:
        let
          unit = services."pi-host-one-${kind}";
          environment = lib.toList unit.Service.Environment;
        in
        unit.Service.Type == "notify"
        && unit.Service.NotifyAccess == "all"
        && unit.Service.Slice == "agents.slice"
        && first unit.Service.RestartPreventExitStatus == "78"
        && unit.Service.WatchdogSec or null == "2min"
        && unit.Unit.StartLimitIntervalSec or null == 600
        && unit.Unit.StartLimitBurst or null == 3
        && unit.Unit.OnFailure or null == [ "pi-host-down@%N.service" ]
        && unit.Unit.X-SwitchMethod == "keep-old"
        && unit.Install.WantedBy == [ "default.target" ]
        && lib.hasPrefix "${node} " (first unit.Service.ExecStart)
        && lib.hasSuffix "/scripts/pi-host.ts ${kind}" (first unit.Service.ExecStart)
        && lib.hasSuffix "/scripts/pi-host.ts" (first unit.Unit.ConditionPathExists)
        && lib.elem ''"MYCELIUM_REALM=one"'' environment
        && lib.elem ''"PI_PACKAGE_DIR=${pi}"'' environment
        && lib.elem ''"PI_HOST_PROOF=${proof}"'' environment
        && lib.elem ''"PROXY_ORIGIN=https://proxy.example.com"'' environment
        # systemd keeps a quoted entry whole, spaces and escaped quotes in it.
        && lib.elem ''"GREETING=say \"hi\" there"'' environment;
      # A realm naming folders runs node through env, which puts them before
      # the PATH systemd expands, quoted so it stays one word, with a folder's
      # % and $ doubled so systemd takes them as they are.
      findsItsPath =
        kind:
        lib.hasPrefix
          "${pkgs.coreutils}/bin/env 'PATH=/first/bin:/second/bin:/100%%/$$HOME:\${PATH}' ${node} "
          (first services."pi-host-two-${kind}".Service.ExecStart);
      # The notice a host left down starts is a template, not a host.
      hostsIn =
        services:
        lib.filter (name: lib.hasPrefix "pi-host-" name && name != "pi-host-down@") (
          lib.attrNames services
        );
      # It runs once, apart from the agents, on the host's name as %i gives it.
      saysDown =
        let
          unit = services."pi-host-down@" or { };
        in
        unit.Service.Type or null == "oneshot"
        && unit.Service.Slice or null == "background.slice"
        && lib.hasSuffix "/bin/pi-host-down %i" (first (unit.Service.ExecStart or ""));
    in
    holds "pi-hosts-start-with-the-session"
      "A realm's pi host isn't started with the session, runs another pi or node, says ready before it listens, runs outside agents.slice, restarts after a refusal or at a switch, isn't watched for a wedged loop, restarts without limit, isn't said to be down once left down, misses its realm's environment or the folders it puts first on PATH, or a machine naming no realm, or a Mac, gets one."
      (
        starts "attended"
        && starts "background"
        && findsItsPath "attended"
        && findsItsPath "background"
        && saysDown
        && !((account workstation).systemd.user.services ? "pi-host-down@")
        &&
          hostsIn services == [
            "pi-host-one-attended"
            "pi-host-one-background"
            "pi-host-two-attended"
            "pi-host-two-background"
          ]
        && hostsIn (account workstation).systemd.user.services == [ ]
        && hostsIn mac.systemd.user.services == [ ]
        && lib.all (check: check.assertion) (naming bareNixos self.nixosModules.workstation).assertions
      );

  # A realm's pi reaches its hosts only with a proof that pnpm test:host passed
  # on the runtime it is in: a proof of another runtime, of one whose name
  # merely starts the same, or not a proof at all fails evaluation, naming the
  # realm and how to prove it.
  pi-hosts-refuse-an-unproven-runtime =
    let
      runtime = "/nix/store/00000000000000000000000000000000-pi-host-runtime-1.5.0";
      pi = "${runtime}/lib/node_modules/pi";
      node = "${pkgs.emptyDirectory}/node";
      realm = proof: { inherit pi node proof; };
      refusals = lib.concatMapStrings (check: check.message + "\n") (
        lib.filter (check: !check.assertion)
          (account (bareNixos [
            self.nixosModules.workstation
            {
              home-manager.users.someone.jitsusama.pi.hosts = {
                proven = realm (proofOf {
                  inherit runtime;
                });
                elsewhere = realm (proofOf {
                  runtime = "/nix/store/11111111111111111111111111111111-pi-host-runtime-1.5.1";
                });
                prefixed = realm (proofOf {
                  runtime = lib.removeSuffix "-1.5.0" runtime;
                });
                stranger = realm (proofOf {
                  inherit runtime;
                  kind = "something-else";
                });
                later = realm (proofOf {
                  inherit runtime;
                  version = 2;
                });
              };
            }
          ])).assertions
      );
    in
    holds "pi-hosts-refuse-an-unproven-runtime"
      "A realm's pi was taken without a proof of its own runtime, or the refusal didn't name the realm and how to prove it."
      (
        !lib.hasInfix "hosts.proven." refusals
        && lib.hasInfix "hosts.elsewhere.proof proves /nix/store/1111" refusals
        && lib.hasInfix "node scripts/host-prove.ts" refusals
        && lib.hasInfix "hosts.prefixed.proof proves" refusals
        && lib.hasInfix "hosts.stranger.proof isn't a pi-host-runtime-proof" refusals
        && lib.hasInfix "hosts.later.proof is version 2, not 1" refusals
      );
}
