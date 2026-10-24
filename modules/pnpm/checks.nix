# The checks of pnpm, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # pnpm 10 and pnpm 12 each keep packages in the account's one store with the
  # files the account gets. Each reads its own file and ignores the other's,
  # so the check runs both, from a project on another device than the store
  # would be in, which is where pnpm would otherwise make a store of its own.
  pnpm-keeps-one-store =
    let
      home = account workstation;
      files = home.xdg.configFile;
      store = "${home.xdg.dataHome}/pnpm/store";
    in
    pkgs.runCommandLocal "pnpm-keeps-one-store" { } ''
      export HOME=$PWD XDG_CONFIG_HOME=$PWD/.config
      mkdir -p .config/pnpm project
      cp ${files."pnpm/rc".source} .config/pnpm/rc
      cp ${files."pnpm/config.yaml".source} .config/pnpm/config.yaml
      echo '{"name":"project"}' > project/package.json
      cd project
      for pnpm in ${pkgs.pnpm_10}/bin/pnpm ${pkgs.pnpm}/bin/pnpm; do
        said=$($pnpm store path)
        case $said in
          ${store}/v*) ;;
          *) echo "$pnpm keeps its store at $said, not under ${store}"; exit 1 ;;
        esac
      done
      touch $out
    '';
}
