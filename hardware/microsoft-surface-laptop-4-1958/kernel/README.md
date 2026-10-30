# The Surface Laptop 4's Kernel Patches

Every patch this laptop's kernel carries beyond core's own kernel, each
written here and in no tree yet, applied by `../nixos.nix`. Each patch's
header says what it fixes and how it was found.

| Patch                                         | Gives                                                    |
| --------------------------------------------- | -------------------------------------------------------- |
| `amd-pstate-fixed-autonomous-selection.patch` | amd-pstate for the CPU, despite a fixed `_CPC` entry     |
| `iwlmvm-country-while-associated.patch`       | Wi-Fi's country, heard after joining at boot             |
| `surface-laptop-amd-sensors-and-fan.patch`    | the controller's eight thermal sensors and its fan speed |
