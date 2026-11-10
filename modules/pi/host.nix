# Shared pi hosts: for each realm a machine names in jitsusama.pi.hosts, two
# long-lived processes, one for the conversations Joel watches and one for
# those that run in the background, each holding many pi conversations
# behind a socket of its own. The harness's host program is the process,
# run by the node the host's pi was built with; the pi it hosts carries the
# host's patches (checks.nix, pi-hosted-keeps-turns-its-own), and every pi
# an account starts by hand stays unpatched. A machine that names no realm
# gets no host. Linux only, for now.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.pi.hosts;

  # The harness's host program, in the version pi-packages calls current.
  # node follows the link as it loads it, so a running host keeps the version
  # it started with through an update, and the next start takes the new one.
  entry = "${config.jitsusama.pi-packages.root}/github.com/Jitsusama/agentic-harness.pi/current/scripts/pi-host.ts";

  homeRealm = lib.attrByPath [ "jitsusama" "mycelium" "realm" ] null config;

  kinds = {
    attended = "the pi conversations Joel watches";
    background = "the pi conversations that run in the background";
  };

  hostOf = realm: host: kind: {
    name = "pi-host-${realm}-${kind}";
    value = {
      Unit = {
        Description = "Holds ${kinds.${kind}} in the ${realm} realm";
        # Until the harness ships the program there is nothing to run. systemd
        # judges this only as the unit starts, so a realm named before then
        # waits for the next login.
        ConditionPathExists = entry;
        # A restart parks every conversation the host holds and ends the turns
        # they are taking, so a switch leaves a running host be. A new pi or a
        # new harness reaches it at its next start: a login, a crash, or a
        # restart its bump gate chooses.
        X-SwitchMethod = "keep-old";
      };
      Service = {
        # The host says it's ready once its socket listens, which is when a
        # conversation can be sent to it; it says so from a child process.
        Type = "notify";
        NotifyAccess = "all";
        ExecStart = lib.escapeShellArgs [
          host.node
          # The harness's package.json names no module type, on purpose.
          "--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"
          entry
          kind
        ];
        # Quoted, as systemd splits an unquoted entry at its spaces.
        Environment = lib.mapAttrsToList (name: value: ''"${name}=${lib.escape [ "\\" "\"" ] value}"'') (
          host.environment
          // {
            MYCELIUM_REALM = realm;
            # pi reads its own files from here, and a subagent a hosted
            # conversation starts runs this pi too.
            PI_PACKAGE_DIR = host.pi;
          }
          // lib.optionalAttrs (homeRealm != null) { PI_HOME_REALM = homeRealm; }
        );
        # The host parks its conversations and keeps their records when told
        # to stop, so only its own failure restarts it; a crash's next start
        # takes back what it held.
        Restart = "on-failure";
        RestartSec = "5s";
        # Parking ends each turn in flight, the tools it runs with it; what is
        # still running after that is ended with it.
        KillMode = "mixed";
        TimeoutStopSec = "60s";
        # Its conversations are agents' sessions, so they run among the
        # sessions rather than among Joel's programs (modules/slices).
        Slice = "agents.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };
  };
in
{
  options.jitsusama.pi.hosts = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          pi = lib.mkOption {
            type = lib.types.path;
            description = ''
              The package directory of a pi built with the host's patches, where
              its dist and node_modules are. The host refuses a pi without them.
            '';
          };
          node = lib.mkOption {
            type = lib.types.path;
            description = ''
              The node that pi was built with, which its native modules are
              built for; the host runs on it.
            '';
          };
          environment = lib.mkOption {
            type = lib.types.attrsOf lib.types.str;
            default = { };
            example = {
              PROXY_ORIGIN = "https://proxy.example.com";
            };
            description = ''
              What the realm's own pi launcher would give every pi it starts,
              as the host never runs that launcher. No credential: the host
              resolves its own, and anything set for one launch belongs to the
              conversation, not here.
            '';
          };
        };
      }
    );
    default = { };
    description = ''
      The realms whose pi conversations run in shared hosts, by realm name, each
      with an attended and a background host started with the account's session.
    '';
  };

  config.systemd.user.services = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    lib.listToAttrs (
      lib.concatLists (
        lib.mapAttrsToList (realm: host: map (hostOf realm host) (lib.attrNames kinds)) cfg
      )
    )
  );
}
