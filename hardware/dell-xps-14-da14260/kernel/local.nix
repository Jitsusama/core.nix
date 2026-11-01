# The XPS 14's kernel patches applied in this order, after the camera's
# (../camera/camera.nix) and before next: the ones written here, and
# cvs-wake-irq, taken from elsewhere, which waits to be filed into next/ at
# the stack's next regeneration, since filing it reorders the patches. The
# camera's list holds three more that wait the same way, which kernel/README
# names.
{ lib, pkgs, ... }:
{
  # After the camera's own, which take the default place, and before next
  # (1600), as the stack was made.
  boot.kernelPatches = lib.mkOrder 1100 [
    {
      # Without it the camera's CVS bridge holds the pin all four amplifiers
      # read their speaker ID from, so none of them bind and there's no sound
      # card. From Intel, and accepted into the media tree: message
      # 20260913133017.624919-1-junjie.cao@intel.com on linux-media, as
      # fetchpatch made it. It's kept here because the list's patchwork now
      # turns away every fetch, under the name and hash it was fetched by, so
      # its store path, and the kernel built from it, stay as they were.
      name = "cvs-wake-irq";
      patch = pkgs.runCommand "cvs-wake-irq.patch" {
        outputHash = "sha256-javJIhGW6mUlgQjgUTgKCEqpD7704+t8zZLtVTe72Dw=";
      } "cp ${./local/cvs-wake-irq.patch} $out";
    }
    {
      # The CVS bridge's wake interrupt leaves an active-low flag on that
      # same pin, so the first amplifier to read its speaker ID reads it
      # inverted and loads the other speakers' tuning: the right woofer,
      # at every boot. Written here; the patch says why.
      name = "gpio-acpi-irq-leaves-polarity";
      patch = ./local/gpio-acpi-irq-leaves-polarity.patch;
    }
    {
      # The four amplifiers probe at once and each takes that pin for itself,
      # so now and then one finds it taken and the sound card never comes.
      # Written here; the patch says why.
      name = "cs35l56-spkid-lock";
      patch = ./local/cs35l56-spkid-lock.patch;
    }
    {
      # The USB-C controller lists a charger's offers with an empty one
      # first, so the kernel reads each one a place late and reports the
      # wrong voltage. Written here; the patch says why.
      name = "ucsi-leading-empty-pdo";
      patch = ./local/ucsi-leading-empty-pdo.patch;
    }
    {
      # Now and then the USB-C controller stops answering while the kernel
      # starts it at boot, and the kernel gives up, leaving no USB-C ports
      # or charger reports until the next boot. Written here; the patch says
      # why.
      name = "ucsi-retry-timed-out-init";
      patch = ./local/ucsi-retry-timed-out-init.patch;
    }
    {
      # The USB-C controller answers "not supported" when asked for a
      # charger's sink offers, which it has none of, so the kernel logs an
      # error and never links the charger's offers to it. Written here; the
      # patch says why.
      name = "ucsi-partner-without-pdos";
      patch = ./local/ucsi-partner-without-pdos.patch;
    }
    {
      # The memory is soldered and has no SPD chip, but the kernel probes the
      # SMBus for one anyway: the first probe hangs the bus, the next seven
      # fail, and the SMBus driver stalls for 450 ms at every boot. Written
      # here; the patch says why.
      name = "spd-skips-soldered-memory";
      patch = ./local/spd-skips-soldered-memory.patch;
    }
    {
      # The firmware gives Bluetooth's power limits and antenna gain setting
      # only as UEFI variables, not in ACPI where the kernel looks, so
      # Bluetooth ran without them. Written here; the patch says why. It
      # reads them as the revision 2 table 7.3 takes.
      name = "bluetooth-uefi-power-tables";
      patch = ./local/bluetooth-uefi-power-tables.patch;
    }
    {
      # Dell's driver offers quiet and the SoC slider low-power, so the legacy
      # profile file offered neither, and power-saver ran as balanced.
      name = "soc-slider-takes-quiet";
      patch = ./local/soc-slider-takes-quiet.patch;
    }
    {
      # The panel gives its 20 to 120 Hz variable refresh range only where
      # the kernel doesn't look, so the driver never offers it. Written here;
      # the patch says why.
      name = "panel-variable-refresh";
      patch = ./local/panel-variable-refresh.patch;
    }
    {
      # The panel gives its 400 nit ceiling only where the kernel doesn't
      # look, so the driver guessed 512 and the brightest fifth of the range
      # asked for more than the panel is rated for. Written here; the patch
      # says why. It needs the variable refresh patch before it.
      name = "panel-luminance-range";
      patch = ./local/panel-luminance-range.patch;
    }
    {
      # A boot docked with the lid shut woke the panel just before reading
      # which brightness interface it has, read it before the panel had
      # filled it in, and set the OLED's brightness through the control
      # that flickers on it until the next boot.
      # Written here; the patch says why. Next's edp-wakes-the-panel is what
      # wakes it, but either applies without the other.
      name = "edp-reads-display-control-awake";
      patch = ./local/edp-reads-display-control-awake.patch;
    }
    {
      # The EC sends an event Dell's driver has no key for about hourly,
      # and each one logged an unknown key. Nothing acts on it; written
      # here, and the patch says what's known of it.
      name = "dell-wmi-ignore-0012-0000";
      patch = ./local/dell-wmi-ignore-0012-0000.patch;
    }
    {
      # Reclaim under a build backed up programs' write-combined buffers,
      # and each program's next draw read them back with its windows'
      # locks held, so the compositor waited up to 304 ms and missed
      # frames. Written here; the patch says why. It needs linux-next's
      # 846f333b3e0b, in next.nix's xe-shrinker-spares-buffers: without it
      # a busy buffer ends the shrinker's whole walk, so the first one kept
      # would stop it reclaiming anything.
      name = "xe-shrinker-keeps-write-combined-buffers";
      patch = ./local/xe-shrinker-keeps-write-combined-buffers.patch;
    }
    {
      # The GPU's buffers are taken in 2 MiB blocks where they can be, and
      # while memory was fragmented the thread asking reclaimed and
      # compacted for one before falling back to smaller pages, so niri
      # stalled for up to 1.3 s. Background reclaim makes the blocks now.
      # Written here; the patch says why.
      name = "ttm-pool-defers-big-pages";
      patch = ./local/ttm-pool-defers-big-pages.patch;
    }
  ];
}
