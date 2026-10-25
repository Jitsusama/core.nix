# What 7.3 adds or fixes for the XPS 14's parts, taken onto 7.2 so the laptop
# has it before its kernel moves. Each patch in ./backports names the mainline
# commits it holds, in order; together they apply to 7.2.12 after the
# kernel's own patches, so they go last. 7.3 has them all, so moving the
# kernel to it fails here, and the way through is to drop this file.
{ config, lib, ... }:
let
  backport = name: {
    inherit name;
    patch = ./backports + "/${name}.patch";
  };
in
{
  assertions = [
    {
      assertion = lib.versionOlder config.boot.kernelPackages.kernel.version "7.3";
      message = "The XPS 14's backports from 7.3 are in its kernel already: drop backports.nix.";
    }
  ];

  boot.kernelPatches = lib.mkAfter (
    map backport [
      # A page flip on a panel whose variable refresh is always on could miss
      # its window and wait a frame.
      "xe-flips-in-the-vrr-window"
      # Panther Lake programmed a vertical total of 1, which the DP stream
      # repeats to the monitor, so some USB-C monitors blanked.
      "xe-real-vtotal"
      # A refresh rate change the compositor asks for without a modeset
      # happens in a frame instead of blanking the panel.
      "xe-refresh-without-modeset"
      # Xe3's memory bandwidth point and buddy logic, so the display asks for
      # no more memory bandwidth than it uses, which saves power.
      "xe-peak-bandwidth"
      # The display clock stayed too low for a downscaled plane, which can
      # underrun and flicker.
      "xe-cdclk-for-downscaling"
      # The console's framebuffer no longer takes stolen memory; hotplug is
      # polled again after a resume; DP audio works over UHBR; four GT fixes.
      "xe-small-fixes"
      # The audio DSP's boot, its host and link DMA, and the period interrupt,
      # which now runs on a high-priority queue, so audio keeps its deadline
      # under memory pressure.
      "sof-dsp-and-dma"
      # The amplifiers' feedback capture is mapped to the right channels.
      "amp-feedback-channels"
      # An HDMI or DP jack plugged in during a suspend is seen at resume.
      "hdmi-jack-after-resume"
      # The headset jack's button device was never removed, so a codec that
      # bound again left a stale one pointing at freed memory. It needs the
      # RT766 codec's driver, which comes along unused.
      "sdca-hid-lifetime"
      # Wi-Fi runs firmware 107, which linux-firmware ships and 7.2 can't
      # talk to, and which says per country whether a channel may be
      # punctured.
      "iwlwifi-core-107"
      # Wi-Fi comes back cleanly from a resume that powered it off.
      "iwlmld-resume-fixes"
      # Bluetooth's resets no longer race each other, and a wedged controller
      # can be reset from vendor_reset without a reboot.
      "btintel-pcie-reset"
      # LE Audio broadcast reports can't read past their buffer, and toggling
      # discoverable while advertising no longer fails.
      "bluetooth-le-fixes"
      # The sensor hub no longer resets when its firmware re-announces its
      # sensors; the light, presence and motion sensors are ready before
      # userspace sees them; the colour sensor's channels have their own
      # scale.
      "ish-sensors"
      # The haptic touchpad offers force feedback from the moment it appears.
      "haptic-ff-before-register"
      # A Thunderbolt 3 dock or eGPU is told it's unplugged at shutdown, so
      # it doesn't poll a dead link for up to a minute, which can turn a
      # power-off into a warm reset.
      "thunderbolt-reset-at-shutdown"
    ]
    ++ [
      # A driver for the Silicon Security Engine, which had none, so the
      # device is no longer left unclaimed.
      (
        backport "issei"
        // {
          structuredExtraConfig = {
            INTEL_SSEI = lib.kernel.module;
            INTEL_SSEI_HW_HECI = lib.kernel.module;
          };
        }
      )
    ]
  );
}
