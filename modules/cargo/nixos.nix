# A filesystem of cargo's own, where every account's builds keep their
# intermediates. On a Btrfs disk, an editor's careful save (write, fsync,
# rename, fsync the folder) waits on the filesystem's log, which every
# writer on that filesystem shares. With twelve cargo builds writing beside
# it, saves took 942.8 ms and 1,247.5 ms at the 99th percentile in two
# rounds; with the builds writing to an ext4 image instead, 13.6 ms and
# 74.6 ms. A subvolume of their own made no difference, since a subvolume
# shares the log.
#
# The image is a file on the system's disk made without copy-on-write, so
# Btrfs sees builds only as overwrites of blocks the file already holds, and
# ext4 keeps its own journal. The image's size is reserved whole, and what's
# in it is cache: the disk guard collects a build folder there once no
# process holds it, as it does a target.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.cargo.builds;
  accounts = lib.filterAttrs (_: user: user.isNormalUser) config.users.users;
  owners = lib.mapAttrsToList (name: user: "${name}:${user.group}") accounts;
  script = pkgs.writeShellApplication {
    name = "cargo-builds";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.e2fsprogs
      pkgs.util-linux
    ];
    text = builtins.readFile ./builds.sh;
  };
in
{
  options.jitsusama.cargo.builds = {
    folder = lib.mkOption {
      type = lib.types.str;
      default = "/var/cache/cargo-builds";
      readOnly = true;
      description = ''
        Where cargo's own filesystem is mounted, with a folder in it for each
        account, which the account's cargo builds its intermediates in.
      '';
    };
    size = lib.mkOption {
      type = lib.types.str;
      default = "48G";
      description = ''
        How big the image is, as fallocate reads a size. It's reserved whole
        on the system's disk. Raising it grows the image at the next boot or
        switch; lowering it leaves the image as it is.
      '';
    };
  };

  config.systemd.services.cargo-builds =
    let
      attach = lib.escapeShellArgs (
        [
          (lib.getExe script)
          "attach"
          "/var/cache/cargo-builds.img"
          cfg.size
          cfg.folder
        ]
        ++ owners
      );
    in
    {
      description = "Gives cargo's builds a filesystem of their own";
      # Mounted before anyone logs in, so no build starts in the folder
      # underneath and is hidden once the image covers it.
      wantedBy = [ "multi-user.target" ];
      before = [ "systemd-user-sessions.service" ];
      after = [ "local-fs.target" ];
      # A switch that changes it attaches again, which only adds a new
      # account's folder or grows the image, rather than restarting it, which
      # would unmount the folder under any build running in it.
      reloadIfChanged = true;
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = attach;
        ExecReload = attach;
        ExecStop = lib.escapeShellArgs [
          (lib.getExe script)
          "detach"
          cfg.folder
        ];
      };
    };
}
