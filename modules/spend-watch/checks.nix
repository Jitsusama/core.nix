# The checks of spend-watch, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # spend-watch only watches, so it yields to whatever the account runs, and
  # comes back when it fails.
  spend-watch-stays-in-the-background =
    let
      service = (account workstation).systemd.user.services.spend-watch.Service;
    in
    holds "spend-watch-stays-in-the-background"
      "spend-watch runs outside background.slice, isn't restarted when it fails, or takes its status with it when it stops."
      (
        service.Slice == "background.slice"
        && service.Restart == "on-failure"
        && service.RuntimeDirectoryPreserve == "yes"
      );
}
// onLinux {
  # spend-watch reads the config an account gets: it's refused at build by
  # the watch's own check-config, which builds it and runs its tests on the
  # way, and the service starts the watch with that file.
  spend-watch-accepts-its-configuration =
    let
      home = account workstation;
      file = home.xdg.configFile."spend-watch/config.json".source;
      # Every option set, so each shape the module renders is one the
      # daemon reads: a feed, a cap of the account's own, a zone.
      fed = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone.jitsusama.spend-watch = {
            feeds.provider = {
              label = "the provider";
              command = [
                "provider-feed"
                "run"
              ];
              environment.REGION = "north";
            };
            cap.allowance = 4000;
            zone = "America/Toronto";
            premium = [ "big-model" ];
            detectors.notesOnly = [ "spikes" ];
          };
        }
      ]);
    in
    pkgs.runCommandLocal "spend-watch-accepts-its-configuration"
      {
        inherit file;
        fed = fed.xdg.configFile."spend-watch/config.json".source;
        service = home.systemd.user.services.spend-watch.Service.ExecStart;
      }
      ''
        case "$service" in
          *"--config $file run") ;;
          *)
            echo "The service doesn't start spend-watch with the account's config: $service"
            exit 1
            ;;
        esac
        touch $out
      '';

  # clippy finds nothing in spend-watch, its tests included.
  spend-watch-passes-clippy = passesClippy "spend-watch-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}
