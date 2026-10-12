# How a held key repeats, the same on every machine: how long it's held before
# it starts, in milliseconds, and how many times a second it goes on. Repeat
# belongs to the computer rather than the keyboard, so it's set once here for
# every keyboard, and niri and macOS each read it in their own terms. A key
# whose release reaches a program later than the delay repeats as if held, so
# the delay is also how late input may be; modules/slices keeps it from being
# that late.
{
  delay = 200;
  rate = 40;
}
