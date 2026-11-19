// The cases of machine for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Numbers.js" as Numbers
.import "../Machine.js" as Machine

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // The machine's figures, from its files as the kernel writes them.
    machine("the whole processor's sensor", ["Package id 0", "Tctl", "Core 3"]
        .map(label => Machine.namesWhole(label)), [true, true, false]);
    machine("a spark against its most", Machine.spark([1, 2, 4], 1), [0.25, 0.5, 1]);
    machine("a spark against the least it's drawn to", Machine.spark([1, 2], 4), [0.25, 0.5]);
    // As Heat says the processor: its package, else its hottest core.
    const sensors = [{ label: "Core 0", value: 60000 }, { label: "Package id 0", value: 55000 },
        { label: "Core 8", value: 70000 }];
    machine("the processor's package", Machine.processor(sensors).label, "Package id 0");
    machine("the processor's hottest core", Machine.processor(sensors.filter(s => s.label !== "Package id 0")).label,
        "Core 8");
    machine("a processor's only sensor", Machine.processor([{ label: "", value: 1 }]).value, 1);

    // Bytes in decimal units to three significant digits, each
    // converted from bytes, stepping up past 999, whole below a kB.
    machine("bytes, each converted", [873 * 1048576, 6.4 * 1048576, 1.52e9, 873, 0, 999.7e3,
        4096].map(Machine.bytes), ["915 MB", "6.71 MB", "1.52 GB", "873 B", "0 B",
        "1.00 MB", "4.10 kB"]);
    // Every measure takes the one rule: three significant digits, zeros
    // kept, a carry into the next digit, grouped past three whole
    // digits, and the real minus under nothing.
    machine("three significant digits", [0.04, 1.234, 12.34, 123.4, 0.9997, 1234, -3.2]
        .map(value => Numbers.significant(value)),
        ["0.0400", "1.23", "12.3", "123", "1.00", "1,230", "\u22123.20"]);
    machine("three significant digits, grouped and pointed as set",
        Numbers.significant(1234.5, ".", ",") + " " + Numbers.significant(5.2, ".", ","),
        "1.230 5,20");
    return asked;
}
