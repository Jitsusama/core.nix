# The Surface Laptop 4's Kernel Patches

Every patch this laptop's kernel carries beyond core's own kernel, applied
by `../nixos.nix`. Each is written here and in no tree yet, except where the
table says it's mainline's, carried until a stable release has it. Each
patch's header says what it fixes and how it was found.

| Patch                                      | Gives                                                    |
| ------------------------------------------ | -------------------------------------------------------- |
| `amdgpu-no-secure-display.patch`           | a boot with no refused TA from the graphics' PSP         |
| `amd-pstate-skip-matching-auto-sel.patch`  | amd-pstate despite a fixed `_CPC` entry, from mainline   |
| `iwlmvm-country-while-associated.patch`    | Wi-Fi's country, heard after joining at boot             |
| `surface-laptop-amd-sensors-and-fan.patch` | the controller's eight thermal sensors and its fan speed |
| `surface-power-asks-only-awake.patch`      | a wake with no warning from the charger and battery      |
