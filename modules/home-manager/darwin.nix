# home-manager on a Mac, configuring each account with the system.
{ home-manager }:
{
  imports = [ home-manager.darwinModules.home-manager ];

  # The account's configuration builds with the system's packages and their
  # settings, such as the unfree ones they allow, and installs into the account's
  # own profile, so one rebuild applies the machine and the account together.
  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
  };
}
