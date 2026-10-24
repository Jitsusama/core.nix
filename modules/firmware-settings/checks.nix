# The checks of firmware-settings, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  pkgs,
  ...
}:
{
  # firmware-settings names every setting the firmware holds apart from the
  # declared ones, writes them with the admin password from its input, and
  # wipes that password from the kernel whether the writes worked or not. A
  # value it can't read, as without root, it refuses rather than calling
  # it changed.
  # Files stand in for the kernel's firmware-attributes class.
  the-firmware-takes-what-is-declared =
    pkgs.runCommandLocal "the-firmware-takes-what-is-declared"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        fw=$PWD/fw/vendor
        setting() {
          mkdir -p "$fw/attributes/$1"
          echo "$2" > "$fw/attributes/$1/current_value"
        }
        value() { cat "$fw/attributes/$1/current_value"; }
        fail() { echo "$1"; cat out; exit 1; }
        mkdir -p "$fw/authentication/Admin" "$fw/attributes"
        echo 1 > "$fw/authentication/Admin/is_enabled"
        : > "$fw/authentication/Admin/current_password"
        echo 0 > "$fw/attributes/pending_reboot"
        setting Mode Adaptive
        setting Start 50
        setting Stop 90
        printf 'Mode=Custom\nStart=75\nStop=80\n' > declared
        export FIRMWARE_ATTRIBUTES=$PWD/fw FIRMWARE_SETTINGS=$PWD/declared
        password() { tr -d '\n' < "$fw/authentication/Admin/current_password"; }

        firmware-settings check > out 2>&1 && fail "check passed with settings to change"
        grep -qF "Start is 50, declared 75" out || fail "check didn't name what differs"

        echo secret | firmware-settings apply > out 2>&1 || fail "apply failed"
        [ "$(value Mode)/$(value Start)/$(value Stop)" = Custom/75/80 ] ||
          fail "apply left $(value Mode)/$(value Start)/$(value Stop)"
        [ -z "$(password)" ] || fail "apply left the password in the kernel"
        firmware-settings check > out 2>&1 || fail "check failed after apply"

        setting Locked old
        chmod a-w "$fw/attributes/Locked/current_value"
        echo Locked=new >> declared
        echo secret | firmware-settings apply > out 2>&1 && fail "apply passed a setting it couldn't write"
        grep -qF "Locked is old, declared new" out || fail "apply didn't name what it couldn't write"
        [ -z "$(password)" ] || fail "a failed apply left the password in the kernel"

        echo 0 > "$fw/authentication/Admin/is_enabled"
        printf 'Mode=Standard\n' > declared
        firmware-settings apply < /dev/null > out 2>&1 || fail "apply asked for a password none is set for"
        [ "$(value Mode)" = Standard ] || fail "apply without a password didn't write"

        printf 'Missing=x\n' > declared
        status=0
        firmware-settings check > out 2>&1 || status=$?
        [ "$status" = 2 ] || fail "check of a setting the firmware lacks exited $status"
        grep -qF "no firmware setting Missing" out || fail "check didn't name the missing setting"

        chmod a-r "$fw/attributes/Mode/current_value"
        printf 'Mode=Standard\n' > declared
        status=0
        firmware-settings check > out 2>&1 || status=$?
        [ "$status" = 77 ] || fail "check of a setting it can't read exited $status"
        grep -qF "can't read Mode; run it as root: run0 firmware-settings check" out ||
          fail "check didn't say to run it as root"
        grep -qF "Mode is" out && fail "check named a setting it couldn't read as differing"
        touch $out
      '';
}
