# The checks of repos, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # repos's tests run as it builds: checkouts and trees laid out by hand
  # as git lays them out, a reftable's included, read into one
  # repository for each remote, each checkout's store weighed and judged
  # due, every view of them fitting each width, and the GitHub receptor
  # said with its counts, the changes asking review and the realm's
  # owners, none of them while a read failed; and Code's homes, as the
  # daemon writes them.
  repos-reads-the-disk = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in repos, its tests included.
  repos-passes-clippy = passesClippy "repos-passes-clippy" (pkgs.callPackage ./package.nix { });
}
// onLinux {
  # The refresh's unit, as home-manager writes it, gives every realm's
  # owners as one word. systemd splits an unquoted Environment= at
  # whitespace, so owners joined by spaces lost a realm's list, and an
  # owner shared with it then counted in the other realm
  # (src/receptor.rs). So the check reads the unit of a machine declaring
  # two realms, one owner listed in both. The unit's program must also set
  # the declared login outright: a refresh keeps whose review each change
  # asks against it, and nothing else would show those reviews went
  # unmarked. tests/repos-is-told-every-realms-owners.sh reads both.
  repos-is-told-every-realms-owners =
    let
      home = account (bareNixos [
        self.nixosModules.workstation
        {
          home-manager.users.someone = {
            jitsusama.mycelium.realm = "personal";
            jitsusama.repos.receptorLogin = "jsama";
            jitsusama.repos.receptorOwners = {
              personal = [
                "jsama"
                "shared"
              ];
              work = [
                "kestrel-co"
                "shared"
              ];
            };
          };
        }
      ]);
      unit = home.xdg.configFile."systemd/user/repos-refresh.service".source;
      owners = "Environment=REPOS_RECEPTOR_OWNERS=personal=jsama,shared;work=kestrel-co,shared";
      login = "export REPOS_LOGIN='jsama'";
    in
    pkgs.runCommandLocal "repos-is-told-every-realms-owners" { } ''
      ${lib.getExe pkgs.bash} ${./tests/repos-is-told-every-realms-owners.sh} \
        ${unit} ${lib.escapeShellArg owners} ${lib.escapeShellArg login}
      touch $out
    '';
}
