// The cases of sums for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Numbers.js" as Numbers

// What's typed, and the answer's headline.
var sums = ({
    "12*7": "84",
    "2^10": "1,024",
    "2**10": "1,024",
    "(3+4)/2": "3.5",
    "-3 + 5": "2",
    "10 % 4": "2",
    "1/3": "0.3333333333",
    "sqrt(2)": "1.4142135624",
    "2 * pi": "6.2831853072",
    "1,000,000 * 3": "3,000,000",
    "6 \u00d7 7": "42",
    "0x1f": "31",
    "0o17": "15",
    "0b1010": "10",
    "255 in hex": "0xff",
    "255 to bin": "0b11111111",
    "0o17 in dec": "15",
    "0xff + 1": "256",
    "0xff_ff": "65,535",
    "0b1011 << 3": "88",
    "1 << 4": "16",
    "0x10 >> 2": "4",
    "~5": "-6",
    "5 xor 3": "6",
    "5 & 3": "1",
    "5 | 3": "7",
    "2^64": "18,446,744,073,709,551,616",
    "2^64 in hex": "0x10000000000000000",
    "5!": "120",
    "log(100)": "2",
    "ln(e^2)": "2",
    "20% of 50": "10",
    "3 squared": "9",
    "2 cubed": "8",
    "gcd(12, 18)": "6",
    "sqrt(-1)": "i"
});

// Division by zero has no value, so it isn't answered: the old engine
// said infinity, which it isn't from both sides. What other answers
// own stays theirs: addresses, a hardware address, a time, money,
// measures, a colour and words.
var notSums = ["2", "-5", "pi", "kitty", "ws 3", "1password", "12*",
    "(3", "sqrt", "3.5 in hex", "2fa", "e", "1/0", "8.8.8.8", "10.0.0.0/22",
    "3c:22:fb:12:34:56", "12:30", "100 USD in CAD", "5 km * 2", "350 f in c",
    "brightness 40", "volume 40", "time in utc", "#f53", "now", "command(\"true\")",
    "battery life", "2 + x"];

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    for (const typed in sums) {
        asked++;
        const got = Numbers.answer(typed, undefined, Maths);
        if (!got || got.headline !== sums[typed])
            failures.push(`"${typed}" answered ${quoted(got && got.headline)}, `
                + `not "${sums[typed]}"`);
    }
    for (const typed of notSums) {
        asked++;
        const got = Numbers.answer(typed, undefined, Maths);
        if (got)
            failures.push(`"${typed}" answered ${quoted(got.headline)}, but nothing should`);
    }
    // Whole numbers keep their zeros, with or without decimals asked for.
    for (const [value, places, written] of [[10, 0, "10"], [1200, 0, "1,200"],
            [2.50, 2, "2.5"], [100, 2, "100"], [0, 0, "0"]]) {
        asked++;
        const got = Numbers.format(value, places);
        if (got !== written)
            failures.push(`${value} to ${places} places was written "${got}", `
                + `not "${written}"`);
    }
    // A collection's count: its own one and many, grouped in threes, the
    // bare figure where it has none, and none held said as empty.
    const keys = { one: "{count} key", many: "{count} keys" };
    for (const [count, said, written] of [[0, keys, "empty"], [1, keys, "1 key"],
            [1204, keys, "1,204 keys"], [3, {}, "3"], [1, undefined, "1"]]) {
        asked++;
        const got = Numbers.counted(count, said);
        if (got !== written)
            failures.push(`${count} was counted "${got}", not "${written}"`);
    }
    // Places in a line, the teens apart.
    for (const [place, said] of [[1, "1st"], [2, "2nd"], [3, "3rd"], [4, "4th"], [11, "11th"],
            [12, "12th"], [13, "13th"], [21, "21st"], [22, "22nd"], [111, "111th"]]) {
        asked++;
        const got = Numbers.ordinal(place);
        if (got !== said)
            failures.push(`place ${place} was said "${got}", not "${said}"`);
    }
    // A base's answer, or a sum on bits, shows each other base as a row,
    // a negative number signed rather than in two's complement.
    for (const [typed, rows] of [["0x1f", ["0x1f", "0o37", "0b11111"]],
            ["0b1011 << 3", ["0x58", "0o130", "0b1011000"]],
            ["~5", ["-0x6", "-0o6", "-0b110"]],
            ["255 in hex", ["255", "0o377", "0b11111111"]]]) {
        asked++;
        const got = Numbers.answer(typed, undefined, Maths);
        if (!got || quoted(got.rows.map(r => r.text)) !== quoted(rows))
            failures.push(`"${typed}" gave the rows ${quoted(got && got.rows)}`);
    }
    // A plain sum has none, and what's copied is never grouped.
    const plain = Numbers.answer("1000 * 1000", undefined, Maths);
    asked++;
    if (!plain || plain.rows.length !== 0 || plain.copy !== "1000000")
        failures.push(`"1000 * 1000" gave ${quoted(plain)}`);
    // A sum past its patience is late, not refused, and isn't kept: asked
    // again with time enough it's worked out, rather than late for good.
    // This one takes tens of milliseconds, so a millisecond is too few.
    const slow = "isprime(2^4423-1)";
    const hurried = Maths.calculate(slow, 1);
    const given = Maths.calculate(slow, 0);
    asked++;
    if (!hurried.late || given.late)
        failures.push(`"${slow}" was late ${hurried.late} hurried, then ${given.late}`);
    return asked;
}
