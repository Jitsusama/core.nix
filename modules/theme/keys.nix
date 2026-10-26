# How a key is drawn wherever a surface names one, by its xkb name: the
# desktop's shortcuts and launcher, and each program's own line of keys. A
# key's own glyph where a proper one exists, its name where none does, and
# a letter as itself (docs/design.md, Shape and Kinds). A key's glyph is the
# theme's mark for what it prints, or for the one meaning it shares with
# the rest of the desktop. Icon marks draw wider than a letter, so they're
# marked to be given room. The theme module's keys option starts from this
# (home.nix), with the marks it was given.
{ mark }:
{
  Super = {
    text = mark "superKey";
    wide = true;
  };
  Shift.text = "⇧";
  Return.text = "↵";
  Up.text = "↑";
  Down.text = "↓";
  Left.text = "←";
  Right.text = "→";
  Escape.text = "Esc";
  Page_Up.text = "PgUp";
  Page_Down.text = "PgDn";
  Minus.text = "-";
  Equal.text = "=";
  Slash.text = "/";
  Period.text = ".";
  Semicolon.text = ";";
  Comma.text = ",";
  Grave.text = "`";
  BracketLeft.text = "[";
  BracketRight.text = "]";
  WheelScrollUp.text = "Wheel ↑";
  WheelScrollDown.text = "Wheel ↓";
  WheelScrollLeft.text = "Wheel ←";
  WheelScrollRight.text = "Wheel →";
  XF86AudioRaiseVolume = {
    text = mark "louder";
    wide = true;
  };
  XF86AudioLowerVolume = {
    text = mark "quieter";
    wide = true;
  };
  XF86AudioMute = {
    text = mark "silent";
    wide = true;
  };
  XF86AudioMicMute = {
    text = mark "muted";
    wide = true;
  };
  XF86MonBrightnessUp = {
    text = mark "brightness";
    wide = true;
  };
  XF86MonBrightnessDown = {
    text = mark "dimmer";
    wide = true;
  };
  XF86AudioPrev = {
    text = mark "previous";
    wide = true;
  };
  XF86AudioNext = {
    text = mark "next";
    wide = true;
  };
  XF86AudioPlay = {
    text = mark "playPause";
    wide = true;
  };
  # The lock key and the Do Not Disturb key, as a padlock and the moon a
  # Mac keyboard prints on its Focus key, rather than their long names.
  XF86ScreenSaver = {
    text = mark "session";
    wide = true;
  };
  XF86DoNotDisturb = {
    text = mark "quietKey";
    wide = true;
  };
  # The Copilot key, by what it prints, though xkb names it for F23.
  XF86TouchpadOff.text = "Copilot";
}
