# The XPS 14's kernel patches applied in this order, after the camera's
# (../camera/camera.nix) and before the backports and next: the ones written
# here, and four taken from elsewhere that wait to be filed by where they
# stand upstream (cvs-wake-irq into next/; dm-crypt-status-after-last-reference,
# cs35l56-suspend-before-attach and bluetooth-sar-rev2 into backports/) at the
# stack's next regeneration, since filing them reorders the patches. The
# camera's list holds four more that wait the same way, which kernel/README
# names.
{ lib, pkgs, ... }:
{
  # After the camera's own, which take the default place, and before the
  # backports (lib.mkAfter) and next (1600), as the stack was made.
  boot.kernelPatches = lib.mkOrder 1100 [
    {
      # Without it the camera's CVS bridge holds the pin all four amplifiers
      # read their speaker ID from, so none of them bind and there's no sound
      # card. From Intel, and accepted into the media tree.
      name = "cvs-wake-irq";
      patch = pkgs.fetchpatch {
        name = "cvs-wake-irq.patch";
        url = "https://patchwork.linuxtv.org/project/linux-media/patch/20260913133017.624919-1-junjie.cao@intel.com/mbox/";
        hash = "sha256-javJIhGW6mUlgQjgUTgKCEqpD7704+t8zZLtVTe72Dw=";
      };
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
      # The encrypted disk could report a failed read or write as a success
      # when two completions raced. Mainline's fix, which 7.2's stable
      # releases have yet to take.
      name = "dm-crypt-status-after-last-reference";
      patch = ./local/dm-crypt-status-after-last-reference.patch;
    }
    {
      # This chip's Bluetooth power limits come as a revision 2 table, which
      # 7.2 rejects. Mainline's support for it, from 7.3.
      name = "bluetooth-sar-rev2";
      patch = ./local/bluetooth-sar-rev2.patch;
    }
    {
      # The firmware gives Bluetooth's power limits and antenna gain setting
      # only as UEFI variables, not in ACPI where the kernel looks, so
      # Bluetooth ran without them. Written here; the patch says why. It
      # needs the revision 2 patch before it.
      name = "bluetooth-uefi-power-tables";
      patch = ./local/bluetooth-uefi-power-tables.patch;
    }
    {
      # A suspend before an amplifier first attaches skipped a step its
      # resume always undoes, unbalancing its power management. Mainline's
      # fix, backported to 7.2's names.
      name = "cs35l56-suspend-before-attach";
      patch = ./local/cs35l56-suspend-before-attach.patch;
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
      # The EC sends an event Dell's driver has no key for about hourly,
      # and each one logged an unknown key. Nothing acts on it; written
      # here, and the patch says what's known of it.
      name = "dell-wmi-ignore-0012-0000";
      patch = ./local/dell-wmi-ignore-0012-0000.patch;
    }
    {
      # Reclaim under a build backed up the window buffers programs share
      # with the compositor, and the next draw read each back with its lock
      # held, so the compositor waited up to 220 ms and missed frames, even
      # at the most the shrinker's seeks can be raised. Written here; the
      # patch says why.
      name = "xe-shrinker-keeps-exported-buffers";
      patch = ./local/xe-shrinker-keeps-exported-buffers.patch;
    }
  ];
}
