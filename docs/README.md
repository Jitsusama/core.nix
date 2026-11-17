# Docs

The guides to core.nix, each about one part of it, and the records of why its
design is what it is. [AGENTS.md][agents] points at each from the job it's
read for.

| Guide                | Says                                                       |
| -------------------- | ---------------------------------------------------------- |
| [architecture.md][1] | how the pieces connect, and how to trace any of them       |
| [glossary.md][2]     | what every thing is called, once                           |
| [conventions.md][3]  | the rules core.nix is written to, with their reasons       |
| [modules.md][4]      | adding a tool, or changing which machines get it           |
| [machines.md][5]     | how a machine repository defines, changes and applies one  |
| [hardware.md][6]     | each machine model: what works, and each workaround's why  |
| [kernel.md][7]       | the kernel core builds, its patches and settings           |
| [installing.md][8]   | a laptop from a blank disk to Secure Boot, its disk sealed |
| [recovering.md][9]   | what to do when a laptop's disk, keys or boot fail         |
| [ergodox.md][10]     | the ErgoDox's firmware, built here from its Oryx layout    |
| [design.md][11]      | how everything on a screen looks, reads and behaves        |
| [daemons.md][12]     | the shape every daemon takes                               |
| [commands.md][13]    | the shape every command takes                              |
| [testing.md][14]     | what each kind of check proves, and where a case goes      |
| [delivery.md][15]    | how a change reaches Joel's screen, and spending builders  |

[`decisions/`][16] holds the decision records, one a file, with an index of
its own.

[agents]: ../AGENTS.md
[1]: architecture.md
[2]: glossary.md
[3]: conventions.md
[4]: modules.md
[5]: machines.md
[6]: hardware.md
[7]: kernel.md
[8]: installing.md
[9]: recovering.md
[10]: ergodox.md
[11]: design.md
[12]: daemons.md
[13]: commands.md
[14]: testing.md
[15]: delivery.md
[16]: decisions/README.md
