# Decisions

Each record says what was decided, what it was weighed against, and what it
costs. A record is never rewritten to change its decision: a later record
replaces it and says so.

| Record    | Decision                                     |
| --------- | -------------------------------------------- |
| [0001][1] | A plain flake, with `flake.nix` as the index |
| [0002][2] | Choosing is importing                        |
| [0003][3] | core.nix owns the pins                       |
| [0004][4] | home-manager runs inside the system          |
| [0005][5] | Modules and options are the interface        |
| [0006][6] | Build the kernel here                        |
| [0007][7] | Hardware lives here                          |
| [0008][8] | Only Joel's keys, and the TPM opens the disk |
| [0009][9] | The TPM holds the keys                       |
| [0010][10] | Checks serve the loop                        |

A new record copies the shape of the others: Status, Context, Decision,
Alternatives and Consequences, numbered after the last.

[1]: 0001-plain-flake.md
[2]: 0002-choosing-is-importing.md
[3]: 0003-core-owns-the-pins.md
[4]: 0004-home-manager-inside-the-system.md
[5]: 0005-modules-and-options-are-the-interface.md
[6]: 0006-build-the-kernel-here.md
[7]: 0007-hardware-lives-here.md
[8]: 0008-own-keys-and-a-tpm-sealed-disk.md
[9]: 0009-the-tpm-holds-the-keys.md
[10]: 0010-checks-serve-the-loop.md
