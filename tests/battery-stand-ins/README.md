# Battery Stand-Ins

Stand-ins for the programs a service that follows the battery talks to,
so a check proves which calls it makes on battery and on mains without a
battery or a desktop. The panel's refresh rate and the power profile are
checked this way, from their modules' checks.

| File                  | Stands in for, or does                                         |
| --------------------- | -------------------------------------------------------------- |
| `run.sh`              | runs the service against the others and checks the calls it made |
| `busctl.sh`           | busctl asking UPower whether the machine is on battery         |
| `gdbus.sh`            | gdbus watching UPower for the battery's changes                |
| `niri.sh`             | niri listing its outputs and setting their modes               |
| `powerprofilesctl.sh` | power-profiles-daemon listing and setting profiles             |
