// The cases of machine for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Numbers.js" as Numbers
.import "../Machine.js" as Machine

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // The machine's figures, from its files as the kernel writes them.
    // A served row's times are worded here, by their form, into the
    // item's own words: the uptime counts up from the boot, the start
    // is a moment.
    const timed = {
        lasted: seconds => `${seconds} s`,
        moment: (at, now) => `at ${at} of ${now}`
    };
    machine("a lasted time counts up from it, in its words", Machine.sideSaid(
        [{ at: 1000, as: "lasted", said: "up {when}" }], 61_000, timed), "up 60 s");
    machine("a moment says when", Machine.sideSaid([{ at: 5, as: "moment" }], 9, timed),
        "at 5 of 9");
    machine("a time from a wrong clock never reads as negative", Machine.sideSaid(
        [{ at: 9000, as: "lasted" }], 1000, timed), "0 s");
    machine("words and figures joined, the empty given way", Machine.sideSaid(
        [{ said: "8.00 GB used" }, { said: "" }, { said: "50%" }], 0, timed),
        "8.00 GB used \u00b7 50%");
    // A served row drawn: its thing as its key (either name the record
    // gives it), what Copy copies from its copy act, the machine's hue
    // where it names none.
    machine("a served row's fields", Machine.served({ key: "row:machine:memory:used",
        kind: "row", row: { thing: "machine:memory:used", name: "Used",
            side: [{ said: "8.00 GB of 16.0 GB" }], graph: { shape: "bar", values: [0.5] },
            note: "8.00 GB available", glyph: "memory",
            acts: [{ key: "verb:copy", copy: "8.00 GB of 16.0 GB" }] } }, 0, timed),
        { key: "machine:memory:used", name: "Used", side: "8.00 GB of 16.0 GB",
            graph: { shape: "bar", values: [0.5] }, note: "8.00 GB available",
            copy: "8.00 GB of 16.0 GB", glyph: "memory", hue: "machine" });
    machine("a served row before its thing was renamed", Machine.served({
        row: { row: "machine:uptime", name: "Uptime", side: [], acts: [] } }, 0, timed).key,
        "machine:uptime");
    machine("a served row copying nothing", Machine.served({ row: { name: "Using It",
        side: [{ said: "\u2026" }] } }, 0, timed).copy, "");
    // A note that's a time, as a disk's health says when its controller
    // said it, is worded as a side's time is.
    machine("a served row's note as a time", Machine.served({ row: { name: "Health",
        side: [], note: { at: 5, as: "moment", said: "its controller said {when}" } }
    }, 9, timed).note, "its controller said at 5 of 9");
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
