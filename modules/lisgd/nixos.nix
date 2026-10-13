# A touchscreen, for lisgd's swipes (home.nix): udev names the first one
# /dev/input/touchscreen, opens it to whoever is signed in at the seat, as
# it does a camera, and starts the account's lisgd when one is plugged in.
# A machine without one gains nothing.
{ pkgs, ... }:
{
  # Before udev's seat rules act on uaccess, which a later file's tag misses.
  services.udev.packages = [
    (pkgs.writeTextDir "lib/udev/rules.d/70-touchscreen.rules" ''
      ACTION!="remove", SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_TOUCHSCREEN}=="1", \
        SYMLINK+="input/touchscreen", TAG+="uaccess", \
        TAG+="systemd", ENV{SYSTEMD_USER_WANTS}+="lisgd.service"
    '')
  ];
}
