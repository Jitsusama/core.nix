# The checks of elsewhere, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
onLinux {
  # elsewhere's tests run as it builds, against a stand-in realm: a crowded
  # machine runs a job in a far copy of its folder, after its kind's setup,
  # and brings back what the kind names; the far job's code is its own; a
  # machine with room keeps the job; an unreached machine ends 69 and a
  # program its kind doesn't allow is refused before anything runs.
  elsewhere-sends-only-from-a-crowded-machine = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in elsewhere, its tests included.
  elsewhere-passes-clippy = passesClippy "elsewhere-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # The machines and kinds a machine repository declares reach elsewhere's
  # settings in the words the program reads, and an account that declares
  # none still has it on its path, sending nothing.
  elsewhere-reads-what-a-machine-declares =
    let
      declared = account (bareNixos [
        {
          home-manager.users.someone = {
            imports = [ self.homeModules.elsewhere ];
            jitsusama.elsewhere = {
              machines = [ "far" ];
              kinds.model-check = {
                programs = [ "tla-rs" ];
                setup = "PATH=/far/bin:$PATH";
                bringsBack = [ "states" ];
              };
            };
          };
        }
      ]);
      none = account (bareNixos [
        { home-manager.users.someone.imports = [ self.homeModules.elsewhere ]; }
      ]);
      read = machine: builtins.fromJSON machine.xdg.configFile."elsewhere/settings.json".text;
      settings = read declared;
      check = settings.kinds.model-check;
    in
    holds "elsewhere-reads-what-a-machine-declares"
      "A machine's declared machines or kinds don't reach elsewhere's settings as it reads them, or an account declaring none sends somewhere or lacks the program."
      (
        settings.machines == [ "far" ]
        && settings.scratch == ".cache/elsewhere"
        && check.programs == [ "tla-rs" ]
        && check.copies == "folder"
        && check.brings_back == [ "states" ]
        && pkgs.lib.hasPrefix "PATH=/far/bin" check.setup
        && (read none).machines == [ ]
        && (read none).kinds == { }
        && pkgs.lib.any (package: package.pname or "" == "elsewhere") none.home.packages
      );
}
