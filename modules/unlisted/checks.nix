# The checks of the programs kept out of the launcher, which checks.nix
# gathers with every other folder's; its opening comment says what each
# kind of check proves.
{
  account,
  graphical,
  onLinux,
  pkgs,
  workstation,
  ...
}:
onLinux {
  # Each program nobody starts has its entry masked in the profile the
  # launcher reads it from, the machine's for what the machine installs and
  # the account's for what the account does, and the program itself is
  # still there.
  programs-nobody-starts-stay-unlisted =
    pkgs.runCommandLocal "programs-nobody-starts-stay-unlisted" { }
      ''
        unlisted() {
          entry=$1/share/applications/$3.desktop
          if ! [ -e "$entry" ]; then
            echo "$3 has no entry in $1 to mask"
            exit 1
          fi
          if ! grep -qx 'Hidden=true' "$entry"; then
            echo "$3's entry in $1 isn't masked:"
            cat "$entry"
            exit 1
          fi
          if ! [ -x "$1/bin/$2" ]; then
            echo "$2 is gone from $1 with its entry"
            exit 1
          fi
        }
        unlisted ${workstation.config.system.path} virt-manager virt-manager
        unlisted ${graphical.config.system.path} nixos-help nixos-manual
        unlisted ${(account graphical).home.path} umpv umpv
        touch $out
      '';
}
