# A YubiKey's touch answers polkit when it asks an administrator, such as
# for run0 or a switch to a system the machine has never run
# (modules/switch-system), with the password still taken when no key is
# there. A touch is quicker than typing the password, and just as far out
# of reach of an agent running as the account. Getting in is the person's
# own secret alone: the key sits in the machine, so a touch is taken only
# once they're in, from their seat's session while it's unlocked
# (once-in.sh), never at the disk, a login, the greeter or the lock.
#
# Each key is enrolled once, by hand, with pamu2fcfg, and what it prints
# is named per account in jitsusama.yubikey.administers; until a key is
# named, polkit asks for the password as before. The keys are bound to one
# origin for every machine, so a key enrolled on one answers on all. They
# live in a root-owned file the system makes, never the account's own
# ~/.config/Yubico, which anything running as the account could add a key
# to.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  enrolled = config.jitsusama.yubikey.administers;
  origin = "pam://administrators";
  # pamu2fcfg's line for an account, its name and then each key's
  # credential, colon-separated.
  # The gate before the touch, which says whether the person is in.
  onceIn = pkgs.writeShellApplication {
    name = "yubikey-once-in";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.gnugrep
      pkgs.systemd
    ];
    text = builtins.readFile ./once-in.sh;
  };
  mappings = lib.concatStrings (
    lib.mapAttrsToList (account: keys: "${lib.concatStringsSep ":" ([ account ] ++ keys)}\n") enrolled
  );
in
{
  options.jitsusama.yubikey.administers = lib.mkOption {
    # A credential as pamu2fcfg prints it after the account's name: its
    # handle, its public key, the algorithm and what it asks of the key.
    type = lib.types.attrsOf (
      lib.types.listOf (
        lib.types.strMatching "[A-Za-z0-9+/=_-]+,[A-Za-z0-9+/=_-]+,(es256|eddsa|rs256),[+a-z]*"
      )
    );
    default = { };
    example = {
      someone = [ "<handle>,<public key>,es256,+presence" ];
    };
    description = ''
      The YubiKeys each account touches to answer polkit as an
      administrator: what
      `pamu2fcfg --origin=pam://administrators --appid=pam://administrators`
      prints, without the account's name before its first colon, one entry
      a key.
    '';
  };

  config = lib.mkMerge [
    {
      # pamu2fcfg, to enrol a key before any is named.
      environment.systemPackages = [ pkgs.pam_u2f ];
    }
    {
      # Getting in is the person's own secret alone: no sign-in but
      # polkit's, which only a session already in reaches, takes a key.
      assertions =
        let
          touching = lib.attrNames (
            lib.filterAttrs (
              name: service: name != "polkit-1" && lib.hasInfix "pam_u2f.so" (toString service.text)
            ) config.security.pam.services
          );
        in
        [
          {
            assertion = touching == [ ];
            message = "A YubiKey's touch would let someone in with no password: ${lib.concatStringsSep ", " touching}.";
          }
        ];
    }
    (lib.mkIf (enrolled != { }) {
      environment.etc.u2f-mappings = {
        text = mappings;
        mode = "0444";
      };

      # Only polkit's own sign-in asks for a touch, and only once the
      # person is in (once-in.sh); the greeter, the lock and every other
      # login keep the password alone.
      security.pam.services.polkit-1 = {
        u2f.enable = true;
        # Skips the touch, straight to the password, unless the gate
        # says yes; a jump counts for nothing in the sign-in's result.
        rules.auth.yubikey-once-in = {
          order = config.security.pam.services.polkit-1.rules.auth.u2f.order - 10;
          control = "[success=ok default=1]";
          modulePath = "${pkgs.pam}/lib/security/pam_exec.so";
          args = [
            "quiet"
            (lib.getExe onceIn)
          ];
        };
      };
      # The helper systemd starts for each prompt hides the machine's
      # devices and the runtime folders; NixOS opens them only for a u2f
      # set on every service, so polkit's alone opens them here: the key's
      # hidraw for pam_u2f, and the runtime folders for the lock's word.
      systemd.services."polkit-agent-helper@".serviceConfig = {
        PrivateDevices = false;
        DeviceAllow = [
          "/dev/urandom r"
          "char-hidraw rw"
        ];
        ProtectHome = "read-only";
      };
      security.pam.u2f.settings = {
        authfile = "/etc/u2f-mappings";
        inherit origin;
        appid = origin;
        # The prompt says to touch the key, and the touch is all it asks:
        # no PIN, since the touch is what an agent can't give.
        cue = true;
        userpresence = 1;
        pinverification = 0;
      };
    })
  ];
}
