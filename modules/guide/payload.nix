# What a reinstall stick carries for one machine, built from its NixOS
# configuration, for `guide write-stick --install` to seal into the stick's
# vault: what to install (target.json, src/install/mod.rs's Target), the
# layout disko writes for its disk, run on the stick with the stick's own
# disk tools, and its whole system as a zstd binary cache.
{
  lib,
  mkBinaryCache,
  pkgs,
  runCommand,
  writeText,
}:
{
  # The machine's configuration, as nixosSystem's config or a NixOS test's
  # node gives it.
  config,
  # Its YubiKeys by the names the person knows them by, in the order the
  # guide enrols them.
  yubikeys,
  # The account the person signs in with, when the machine has no greeter
  # to name it (jitsusama.login.account).
  account ? null,
}:
let
  disk = config.jitsusama.disk;
  system = config.system.build.toplevel;
  installKey =
    if disk.installKey != null then
      disk.installKey
    else
      throw "${config.networking.hostName} needs jitsusama.disk.installKey set for the guide to lay out its disk";
  account' =
    if account != null then
      account
    else
      config.jitsusama.login.account
        or (throw "${config.networking.hostName} names no account for the guide to give a password: set jitsusama.login.account, or pass account");
  target = writeText "target.json" (
    builtins.toJSON {
      machine = config.networking.hostName;
      disk = disk.device;
      disk_name = disk.name;
      install_key = installKey;
      layout_packages = map lib.getName (config.disko.devices._packages pkgs);
      system = "${system}";
      account = account';
      inherit yubikeys;
    }
  );
  cache = mkBinaryCache { rootPaths = [ system ]; };
in
runCommand "${config.networking.hostName}-stick-payload" { } ''
  mkdir $out
  cp ${target} $out/target.json
  cp ${config.system.build.formatMountNoDeps}/bin/disko-format-mount $out/layout.sh
  ln -s ${cache} $out/cache
''
