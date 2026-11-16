# system says how the whole system stands at once: the machines of the
# realm, the mycelium, the home network and the code (src/lib.rs says how).
# It asks each part's own command, mycelium, net and repos, as they're found
# on the account's path, so it keeps no daemon and no copy of theirs, and a
# machine without one of them says that part wasn't read.
#
# Two readers more than the terminal. An agent reads it through the system
# skill beside this file. The local sites' system/ page reads its record,
# written there each minute by a timer and put in place by a rename, so the
# page never reads half of one; the page itself is the sites' own
# (../local-sites says why), and only its record is declared here.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.callPackage ./package.nix { };
  site = "${config.home.homeDirectory}/.local/share/jitsusama/sites/system";
  # The account's own tools, where mycelium, net and repos are, then the
  # machine's.
  path = lib.concatStringsSep ":" [
    "/etc/profiles/per-user/${config.home.username}/bin"
    "${config.home.homeDirectory}/.nix-profile/bin"
    "/run/current-system/sw/bin"
    (lib.makeBinPath [ pkgs.coreutils ])
  ];
  write = pkgs.writeShellScript "system-site" ''
    set -eu
    mkdir -p ${site}
    ${lib.getExe system} --json > ${site}/system.json.new
    mv -f ${site}/system.json.new ${site}/system.json
  '';
in
{
  home.packages = [ system ];

  # So an agent asked how everything stands answers from the owners, in
  # words, worst first, as the page and the terminal do.
  home.file.".pi/agent/skills/system/SKILL.md".source = ./SKILL.md;

  systemd.user = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    services.system-site = {
      Unit.Description = "Writes the system site's record from what each part's owner says now";
      Service = {
        Type = "oneshot";
        Environment = [ "PATH=${path}" ];
        ExecStart = "${write}";
        Slice = "background.slice";
      };
    };

    # Every minute, as the page says it's read; a run takes under a second.
    timers.system-site = {
      Unit.Description = "Rewrites the system site's record every minute";
      Timer = {
        OnActiveSec = "5s";
        OnUnitActiveSec = "1min";
        AccuracySec = "5s";
      };
      Install.WantedBy = [ "timers.target" ];
    };
  };
}
