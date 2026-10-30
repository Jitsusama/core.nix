# What a stick carries for one machine, built from its NixOS
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
  # Its YubiKeys in the order the guide enrols them, each its name as the
  # person knows it, or { name, recipient } with its age recipient
  # (one beginning age1fido2-hmac1, from age-plugin-fido2-hmac -g) so the stick's
  # vault opens with it. Name every one's recipient or none.
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
  # The option is there with nothing in it on a machine whose greeter
  # isn't told whom it signs in, so an empty one is refused as a missing
  # one is.
  named = if account != null then account else config.jitsusama.login.account or null;
  account' =
    if named != null then
      named
    else
      throw "${config.networking.hostName} names no account for the guide to give a password: set jitsusama.login.account, or pass account";
  keys = map (key: if builtins.isString key then { name = key; } else key) yubikeys;
  recipients = map (key: key.recipient) (builtins.filter (key: key ? recipient) keys);
  target = writeText "target.json" (
    builtins.toJSON {
      machine = config.networking.hostName;
      disk = disk.device;
      disk_name = disk.name;
      install_key = installKey;
      layout_packages = map lib.getName (config.disko.devices._packages pkgs);
      system = "${system}";
      account = account';
      yubikeys = map (key: key.name) keys;
      # How a person reaches its firmware, as its hardware module says.
      start_from_stick = config.jitsusama.firmware.startFromStick or null;
      clear_secure_boot_keys = config.jitsusama.firmware.clearSecureBootKeys or null;
      # What a reinstall leaves behind of the home folders it carries.
      leave_out = config.jitsusama.guide.carry.leaveOut or [ ];
      recipients =
        if recipients == [ ] || builtins.length recipients == builtins.length keys then
          recipients
        else
          throw "${config.networking.hostName} names a recipient for some of its YubiKeys but not all";
    }
  );
  cache = mkBinaryCache { rootPaths = [ system ]; };
in
runCommand "${config.networking.hostName}-stick-payload" { } ''
  mkdir $out
  cp ${target} $out/target.json
  cp ${config.system.build.formatMountNoDeps}/bin/disko-format-mount $out/layout.sh
  # What opens and mounts the disk as it was left, for an install a
  # restart stopped part way.
  cp ${config.system.build.mountNoDeps}/bin/disko-mount $out/mount.sh
  ln -s ${cache} $out/cache
''
