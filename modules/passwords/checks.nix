# The checks of passwords, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# Passwords serves the Linux desktop's shell, and builds only on Linux.
onLinux {
  # The passwords daemon's tests run as it builds, with stand-in password
  # managers in the shell: a list holds every entry and no secret, even
  # one a manager gives with its secrets on; a secret goes only to the
  # client that asked to see it, or to the clipboard marked sensitive,
  # cleared when its time is up; a one-time code says its period; a
  # manager signed out is said so and gives no secret; a watch hears
  # entries arrive; and an ask or kind it doesn't know is refused.
  passwords-holds-no-secret = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the passwords daemon, its tests included.
  passwords-passes-clippy = passesClippy "passwords-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # On a machine with 1Password, the daemon runs it as a password manager
  # on the daemons' socket, which systemd holds, with wl-copy to copy and
  # no core dump to write a secret in, and the launcher's Passwords
  # provider knows its program.
  passwords-runs-each-password-manager =
    let
      home = account graphical;
      service = home.systemd.user.services.passwords or { };
      started = lib.concatStringsSep " " (lib.toList (service.Service.ExecStart or ""));
      provider = home.jitsusama.quickshell.launcher.providers.passwords or { };
    in
    holds "passwords-runs-each-password-manager"
      "The passwords daemon doesn't run 1Password's manager as it should."
      (
        lib.hasInfix "%t/daemons/passwords.sock" started
        && (home.systemd.user.sockets.passwords.Socket.ListenStream or "") == "%t/daemons/passwords.sock"
        && (service.Service.LimitCORE or null) == 0
        && lib.elem "graphical-session.target" (service.Unit.PartOf or [ ])
        && lib.any (lib.hasInfix "PASSWORDS_COPIER=") (lib.toList (service.Service.Environment or [ ]))
        && home.jitsusama.passwords.managers ? "1password"
        && lib.hasSuffix "/op" (lib.last home.jitsusama.passwords.managers."1password".command)
        && map (manager: manager.program) (provider.managers or [ ]) == [ "com.onepassword.OnePassword" ]
      );
}
