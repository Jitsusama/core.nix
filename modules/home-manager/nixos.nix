# home-manager on NixOS, configuring each account with the system, holding
# every login until its account is set up, and switching an account's units
# however many change at once.
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

  # sd-switch, which home-manager switches an account's units with, asks
  # systemd to be told as each job ends but reads none of it until it has
  # sent every job. zbus keeps 64 unread messages of a kind and then stops
  # reading the bus, so in a switch of more than 64 jobs the next job's
  # reply never came: on 16 November two switches of about 75 hung for
  # their 5 minutes and the machine went back to what it ran. Patched, it
  # reads them on a thread of its own from the start, and its wait for the
  # last job ends at its timeout saying how many were left. Through
  # useGlobalPkgs, each account's activation uses the system's patched one.
  nixpkgs.overlays = [
    (_: prev: {
      sd-switch = prev.sd-switch.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [ ./sd-switch-reads-as-it-sends.patch ];
      });
    })
  ];

  # home-manager holds every login until each account is set up, by starting
  # before systemd-user-sessions, which the greeter waits for, so the greeter
  # shows about a second later at every boot. What needs the account set up
  # is its own service manager, which a lingering account starts at boot and
  # every login waits for, so that waits instead. The greeter shows while
  # the accounts are set up, a login still finds its files in place, and the
  # manager no longer reads the account's units while they're being linked.
  #
  # A switch restarts some changed units and waits on them before it starts
  # the ones it stopped, and home-manager asks to be restarted that way. So
  # an account's setup hanging on its own units kept every service the switch
  # had stopped down: on 14 November, a keyboard's layout, the VPN and Wi-Fi,
  # for minutes. Set up with the ones the switch starts instead, the account
  # is set up beside them, and they run again in seconds however long it
  # takes. Stopping its unit stops nothing, since it only says the account
  # was set up.
  systemd.services = lib.mkMerge [
    (lib.genAttrs (map (lib.removeSuffix ".service") homes) (_: {
      before = lib.mkForce [ ];
      stopIfChanged = lib.mkForce true;
    }))
    { "user@".after = homes; }
  ];
}
