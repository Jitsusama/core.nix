# Which of the processor's cores are its efficient ones, a fact of the
# machine's model that its hardware names. It holds nothing on its own:
# the slices read it, to keep agents' work off the performance cores
# (efficient-work.nix says why), and a machine without the slices names it
# to no effect. It's a file of its own so a model's hardware can name it
# without importing the slices.
{ lib, ... }:
{
  options.jitsusama.slices.efficientCores = lib.mkOption {
    description = ''
      The processor's efficient cores, as the kernel lists them in
      /sys/devices/cpu_atom/cpus, such as "4-15". With the slices,
      agents' work and the builds run on them alone. A processor without
      efficient cores names none.
    '';
    default = null;
    example = "4-15";
    type = lib.types.nullOr (lib.types.strMatching "[0-9]+(-[0-9]+)?(,[0-9]+(-[0-9]+)?)*");
  };
}
