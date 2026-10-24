# The checks of pnpm, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  lib,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # pnpm 10 and pnpm 12 each resolve the account's one store from the
  # variables the account gets, in its shells and from its user manager alike.
  # Each reads its own variable and ignores the other's, so the check runs
  # both. The sandbox's project and store share one device, so this proves the
  # store is named and read, not the clone into a subvolume.
  pnpm-keeps-one-store =
    let
      home = account workstation;
      store = "${home.xdg.dataHome}/pnpm/store";
      # Only the stores', so a variable missing shows as a pnpm finding
      # another store.
      exported =
        variables:
        lib.concatStrings (
          lib.mapAttrsToList (name: value: "export ${name}=${lib.escapeShellArg value}\n") (
            lib.filterAttrs (name: _: lib.hasSuffix "_config_store_dir" name) variables
          )
        );
      resolves = place: variables: ''
        (
          ${exported variables}
          for pnpm in ${pkgs.pnpm_10}/bin/pnpm ${pkgs.pnpm}/bin/pnpm; do
            said=$($pnpm store path)
            case $said in
              ${store}/v*) ;;
              *) echo "$pnpm in ${place} keeps its store at $said, not under ${store}"; exit 1 ;;
            esac
          done
        )
      '';
    in
    pkgs.runCommandLocal "pnpm-keeps-one-store" { } ''
      export HOME=$PWD XDG_CONFIG_HOME=$PWD/.config
      mkdir -p project
      echo '{"name":"project"}' > project/package.json
      cd project
      ${resolves "a shell" home.home.sessionVariables}
      ${resolves "the user manager" home.systemd.user.sessionVariables}
      touch $out
    '';
}
