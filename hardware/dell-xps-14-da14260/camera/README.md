# The XPS 14's Cameras

What the laptop's colour and IR cameras need beyond core's camera
modules: Intel's camera HAL with Dell's tuning, the IPU7's out-of-tree
driver, and the IR sensor's capture pipeline. [docs/hardware.md][hardware]
says why each piece is there.

| File                           | Holds                                                    |
| ------------------------------ | -------------------------------------------------------- |
| `camera.nix`                   | the module: the HAL, the driver, the IR pipeline, and which kernel patches the camera takes |
| `board.nix`                    | the option naming which colour camera board is fitted, for its tuning |
| `ir-camera-pipeline.sh`        | the IR sensor's link, set up at each boot                |
| `camera-cvs.patch`             | the HAL, made to route this sensor through the CVS bridge |
| `ipu7-psys-*.patch`            | the out-of-tree IPU7 driver, as its processing module needs it |

The camera's kernel patches live with the laptop's other kernel
patches in [`../kernel/local/`][local], since they're made against the
same kernel and regenerated with it, and `camera.nix` applies them:
`ipu7-survives-resume`, `cvs-link-owner-wait`, `ipu-bridge-hm1092`,
`ipu-bridge-da14260-upside-down`, `hm1092-ir-sensor`, `hm1092-ir-flood`,
`ipu7-mono-formats`, `ipu7-outlives-open-files`,
`ipu7-isys-skips-unread-log` and `ipu7-isys-releases-firmware`.

[hardware]: ../../../docs/hardware.md
[local]: ../kernel/local/
