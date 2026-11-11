// The cases of heat for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Heat.js" as Heat

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // A temperature drawn against its most, as Machine draws the
    // processor's and the disk's (Heat.js's level); Heat's own rows are
    // the machine daemon's, and their cases are its (heat_rows.rs).
    machine("a processor well below its most", Heat.level(83, 100, 100, "processor"),
        { value: 0.83, warn: false });
    machine("a processor near its most", Heat.level(92, 100, 100, "processor").warn, true);
    machine("a disk past its own high point", Heat.level(83, 82.85, 84.85, "disk").warn, true);
    machine("a disk below its own high point", Heat.level(80, 82.85, 84.85, "disk").warn,
        false);
    machine("a battery with no limits said", Heat.level(30, null, null, "battery"),
        { value: 0.5, warn: false });
    machine("a part past its most, drawn full", Heat.level(110, null, 100, "processor").value,
        1);
    return asked;
}
