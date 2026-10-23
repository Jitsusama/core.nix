# Plugging a laptop back in at its desk wakes it, as a key on its keyboard
# already does. Unplugging a monitor's USB-C cable with the lid shut suspends
# the laptop (the lid module), and plugging it back in should wake it, but the
# kernel leaves every USB root hub's wakeup off, so the port never asks to
# wake for a cable that arrives, and the lid had to be lifted.
#
# On an XPS 14 at a monitor's USB-C hub, with each root hub's wakeup on, the
# cable plugged back in 4 s into a sleep woke it at once, and the monitor was
# found 0.17 s after. Only the root hubs: a key on a keyboard behind the
# monitor's two hubs woke it in 6 s with their wakeup off, since the kernel
# keeps a hub's port ready for any device below it that may wake. A wake with
# nobody there, a cable pulled in a bag, suspends again, since logind looks at
# a shut lid on every wake.
_: {
  services.udev.extraRules = ''
    ACTION=="add", SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", KERNEL=="usb[0-9]*", ATTR{power/wakeup}="enabled"
  '';
}
