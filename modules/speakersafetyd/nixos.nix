# speakersafetyd keeps an Apple silicon Mac's speakers from burning out:
# it models the coils' heat from what the amplifiers report and turns them
# down before they overheat. Asahi's kernel leaves the speakers off until it
# runs. Its service and udev rules come from its package.
{ pkgs, ... }:
{
  systemd.packages = [ pkgs.speakersafetyd ];
  services.udev.packages = [ pkgs.speakersafetyd ];
}
