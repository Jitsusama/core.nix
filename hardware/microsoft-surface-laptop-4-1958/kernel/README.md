# The Surface Laptop 4's Kernel Patches

Every patch this laptop's kernel carries beyond core's own kernel, applied
by `../nixos.nix`. Each is written here and in no tree yet. Each patch's
header says what it fixes and how it was found.

| Patch                                      | Gives                                                    |
| ------------------------------------------ | -------------------------------------------------------- |
| `amdgpu-no-secure-display.patch`           | a boot with no refused TA from the graphics' PSP         |
| `amdgpu-vrr-reference-only-while-on.patch` | a wake with no graphics warning when a screen was off    |
| `iwlmvm-country-while-associated.patch`    | Wi-Fi's country, heard after joining at boot             |
| `surface-events-after-restore.patch`       | a keyboard and touchpad back at once after hibernation   |
| `surface-laptop-amd-sensors-and-fan.patch` | the controller's eight thermal sensors and its fan speed |
| `surface-power-asks-only-awake.patch`      | a wake with no warning from the charger and battery      |
