# A Mac's Nix daemon signs every path it builds with a key of the Mac's own,
# as a NixOS machine's does (nixos.nix says why), so another machine takes
# what it built by trusting that key rather than the account that copies it.
# The key is made once, by root, as the Mac first activates, with the same
# script: the secret half in /etc/nix/signing-key, root's alone, and the
# public half in signing-key.pub for a machine repository to hand the others.
#
# It's written for a Mac whose Nix nix-darwin may or may not manage, so it
# writes the daemon's line to /etc/nix/signing.conf, which the machine's own
# Nix settings include, through nix.extraOptions or nix.custom.conf:
#
#   !include signing.conf
#
# Nix reads its settings only as it starts, so the daemon is restarted when
# the line changes, and only then, since a restart ends the builds it's
# running. The key is made before /etc is written, so a daemon that reads the
# line never looks for a key that isn't there yet, which would fail every
# build.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.jitsusama.nix-daemon;
  # What the daemon reads last, kept to say whether it has read this line.
  read = "/var/lib/nix-signing/read-by-nix";
in
{
  options.jitsusama.nix-daemon = {
    keyName = lib.mkOption {
      type = lib.types.nullOr (lib.types.strMatching "[A-Za-z0-9._-]+");
      default = lib.mapNullable (name: "${name}-1") config.networking.hostName;
      defaultText = lib.literalExpression ''"''${config.networking.hostName}-1"'';
      description = ''
        The name the key signs under, which other machines trust it by: the
        machine's, as caches' keys are, or the Mac's own LocalHostName where
        nix-darwin isn't told one. A key already made keeps its name.
      '';
    };
    label = lib.mkOption {
      type = lib.types.str;
      default = "systems.determinate.nix-daemon";
      description = ''
        The launchd label of the Nix daemon that reads the line: the
        installer's, or org.nixos.nix-daemon where nix-darwin runs Nix.
      '';
    };
  };

  config = {
    system.activationScripts.preActivation.text = lib.mkAfter ''
      mkdir -p /etc/nix ${dirOf read}
      ${pkgs.bash}/bin/bash ${./signing-key.sh} ${config.nix.package}/bin/nix \
        ${
          if cfg.keyName == null then ''"$(/usr/sbin/scutil --get LocalHostName)-1"'' else cfg.keyName
        } /etc/nix/signing-key /etc/nix/signing-key.pub
    '';

    # After /etc is written: restart Nix only when the line it reads differs
    # from the one it was last restarted for.
    system.activationScripts.postActivation.text = lib.mkAfter ''
      line=$(/usr/bin/shasum -a 256 /etc/nix/signing.conf | /usr/bin/cut -d' ' -f1)
      if [ "$line" != "$(/bin/cat ${read} 2>/dev/null)" ] &&
        /bin/launchctl print system/${cfg.label} >/dev/null 2>&1; then
        echo "restarting ${cfg.label}, so it signs what it builds" >&2
        /bin/launchctl kickstart -k system/${cfg.label} && echo "$line" > ${read}
      fi
    '';

    environment.etc."nix/signing.conf".text = ''
      secret-key-files = /etc/nix/signing-key
    '';
  };
}
