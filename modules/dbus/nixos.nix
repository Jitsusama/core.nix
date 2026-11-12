# The message buses find each service they can start once. NixOS hands them
# every package in services.dbus.packages, the system's profile among them,
# and links each package's bus files into that profile too; the account's bus
# also searches its standard folders, which take in the system's profile again
# through XDG_DATA_DIRS. So every service file in the profile was found two or
# three times, and the bus logged each as an error at every start and every
# switch: 29 lines each time on a laptop, 1,710 in one boot on a desktop.
#
# Instead both buses read one tree of every package's bus files, the system's
# profile first, so its higher-priority files win as they did (gnome-keyring's
# activation relies on that), and the account's bus searches only the
# account's own profiles beside it. The tree takes effect at the next reload,
# which a switch brings; the account's search, at its bus's next start.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.dbus;

  # Every package's bus files in one tree, the first of each name kept.
  together = pkgs.buildEnv {
    name = "dbus-files";
    paths = [ config.system.path ] ++ cfg.packages;
    pathsToLink = [
      "/etc/dbus-1"
      "/share/dbus-1"
    ];
    ignoreCollisions = true;
  };

  # The profiles a session searches, as the account's unit names them, all but
  # the system's, which the tree above holds. One named through a variable
  # systemd can't fill is left out.
  accountProfiles = lib.pipe config.environment.profiles [
    (lib.remove "/run/current-system/sw")
    (map (lib.replaceStrings [ "$HOME" "$USER" ] [ "%h" "%u" ]))
    (lib.filter (profile: !lib.hasInfix "$" profile))
  ];

  accountBus = if cfg.implementation == "broker" then "dbus-broker" else "dbus";
in
{
  environment.etc."dbus-1".source = lib.mkForce (
    pkgs.makeDBusConf.override {
      inherit (cfg) apparmor;
      dbus = cfg.dbusPackage;
      suidHelper = "${config.security.wrapperDir}/dbus-daemon-launch-helper";
      serviceDirectories = [ together ];
    }
  );

  systemd.user.services.${accountBus}.environment.XDG_DATA_DIRS = lib.concatMapStringsSep ":" (
    profile: "${profile}/share"
  ) accountProfiles;
}
