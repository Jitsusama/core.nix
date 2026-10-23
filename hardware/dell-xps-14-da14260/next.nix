# What linux-next has for the XPS 14's parts and no release does yet, taken
# onto 7.2 so the laptop has it before 7.4. Each patch in ./next names the
# commits it holds and the series they were posted as, and applies after the
# backports from 7.3. Moving the kernel to 7.4 fails here, and the way
# through is to drop each patch that kernel has, then this file.
{ config, lib, ... }:
let
  next = name: {
    inherit name;
    patch = ./next + "/${name}.patch";
  };
in
{
  assertions = [
    {
      assertion = lib.versionOlder config.boot.kernelPackages.kernel.version "7.4";
      message = "The XPS 14's patches from linux-next may be in its kernel already: check next.nix.";
    }
  ];

  boot.kernelPatches = lib.mkOrder 1600 (
    map next [
      # Memory reclaim across the whole machine backed up the window buffers
      # programs draw into, so the next frame each drew read them back first,
      # with the buffer locked. The compositor waited on those locks, the
      # keyboard's events with it, so a key let go of under load read late
      # and repeated. The graphics driver reclaims buffers that need reading
      # back at a quarter of the pressure of those it can drop, and a
      # per-device file, shrinker_non_purgeable_seeks, tunes how much.
      "xe-shrinker-spares-buffers"
      # A read the sink never answered was sent again as a byte-by-byte
      # retry, even when it was one byte, so through an MST hub it waited
      # the 4 s sideband timeout twice.
      "dp-byte-reads-only-when-longer"
      # Adding a connector read the DSC caps of each port on a hub, even one
      # with nothing on it, which some hubs never answer, and the monitor's
      # picture waited behind that read on every plug and boot.
      "dp-mst-dsc-skips-empty-ports"
    ]
  );
}
