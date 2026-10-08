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

A new record copies the shape of the others: Status, Context, Decision,
Alternatives and Consequences, numbered after the last.

[1]: 0001-plain-flake.md
[2]: 0002-choosing-is-importing.md
[3]: 0003-core-owns-the-pins.md
[4]: 0004-home-manager-inside-the-system.md
[5]: 0005-modules-and-options-are-the-interface.md
