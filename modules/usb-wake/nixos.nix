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
#
# Only the USB 2 and 1.1 root hubs, at 480 and 12 Mbit/s, and not the
# SuperSpeed ones, at 5 Gbit/s and more. xHCI arms a port that has a device
# in it to wake on its disconnect, and an empty one on a connect, so every
# device still plugged in when the laptop sleeps at the desk woke it again as
# it came out, for logind's 30 s. A monitor's hub, at either speed, joins on
# the USB 2 bus, so its cable still wakes the laptop; a 5 Gbit/s or faster
# device, as a webcam on USB 3 is, comes and goes on the faster bus alone and
# no longer wakes it. A USB 2 or 1.1 device pulled out still does.
_: {
  services.udev.extraRules = ''
    ACTION=="add", SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", KERNEL=="usb[0-9]*", ATTR{speed}=="12|480", ATTR{power/wakeup}="enabled"
  '';
}
