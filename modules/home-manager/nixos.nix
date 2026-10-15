{ home-manager }:
{
  config,
  lib,
  utils,
  ...
}:
let
  # home-manager sets up each account in a unit of its own at boot.
  homes = lib.mapAttrsToList (
    _: account: "home-manager-${utils.escapeSystemdPath account.home.username}.service"
  ) config.home-manager.users;
in
{
  imports = [ home-manager.nixosModules.home-manager ];

  # The account's configuration builds with the system's packages and their
  # settings, such as allowing unfree ones, and installs into the account's
  # own profile, so one rebuild applies the machine and the account together.
  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
  };

  # home-manager holds every login until each account is set up, by starting
  # before systemd-user-sessions, which the greeter waits for, so the greeter
  # shows about a second later at every boot. What needs the account set up
  # is its own service manager, which a lingering account starts at boot and
  # every login waits for, so that waits instead. The greeter shows while
  # the accounts are set up, a login still finds its files in place, and the
  # manager no longer reads the account's units while they're being linked.
  systemd.services = lib.mkMerge [
    (lib.genAttrs (map (lib.removeSuffix ".service") homes) (_: {
      before = lib.mkForce [ ];
    }))
    { "user@".after = homes; }
  ];
}
