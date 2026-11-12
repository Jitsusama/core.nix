# The checks of which places the launcher turns on, which the folder's
# checks.nix gathers with every other place's; its opening comment says why
# each kind keeps its own.
{
  account,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  ...
}:
let
  # A machine holding every module and all each of them reads.
  everything = bareNixos [
    self.nixosModules.workstation
    self.nixosModules.graphical
    {
      home-manager.sharedModules = [ self.homeModules.eero ];
      services.fwupd.enable = true;
      environment.systemPackages = [
        (pkgs.writeShellApplication {
          name = "warp-cli";
          text = "";
        })
      ];
    }
  ];
  home = account everything;
  providers = home.jitsusama.quickshell.launcher.providers;
  on = lib.attrNames (lib.filterAttrs (_: provider: provider.enable) providers);
in
{
  # A place is on wherever what it reads from is there. A few places turn on
  # by another module's unit, option or package, named by a string in
  # home.nix (eero, waits, firmware, warp and ask), and one name that
  # changed left Home Network off on every machine, while the VM test forced
  # it on and every check passed (eero-watch, renamed eero-daemon, found
  # 2026-11-11). So a machine holding every module and all each of them
  # reads has every provider on, and one that's off names a source that
  # doesn't exist any more.
  every-place-is-on-where-its-source-is =
    let
      off = lib.attrNames (lib.filterAttrs (_: provider: !provider.enable) providers);
    in
    holds "every-place-is-on-where-its-source-is" (
      "On a machine holding all they read from, these places are off, so their condition "
      + "names something that isn't there: ${lib.concatStringsSep ", " off}."
    ) (off == [ ]);

  # Cadence reads which places are on from a file the shell's build writes,
  # never from the shell, so it names each place on, sorted, and nothing else.
  the-places-on-file-names-each-place-on =
    let
      said = builtins.fromJSON home.xdg.configFile."quickshell/launcher/places-on.json".text;
      wrong = "places-on.json says ${builtins.toJSON said}, not ${builtins.toJSON on}.";
    in
    holds "the-places-on-file-names-each-place-on" wrong (said == { inherit on; });
}
