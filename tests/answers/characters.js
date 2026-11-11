// The cases of characters for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Numbers.js" as Numbers
.import "../Subnets.js" as Subnets
.import "../Characters.js" as Characters
.import "../Units.js" as Units
.import "subnets.js" as SubnetCases

// What's typed, the answer's headline, what Return copies, and its
// rows: a part's id and what it says, or a character of several, what
// it says and its name.
var characters = [
    ["ascii 65", "A", "A", ["decimal 65", "hex 0x41", "binary 0b1000001", "point U+0041",
        "name latin capital letter a", "utf8 41"]],
    ["char A", "65", "65", ["hex 0x41", "binary 0b1000001", "point U+0041",
        "name latin capital letter a", "utf8 41"]],
    ["ord 1", "49", "49", ["hex 0x31", "binary 0b110001", "point U+0031", "name digit one",
        "utf8 31"]],
    ["0x41 as text", "A", "A", ["decimal 65", "hex 0x41", "binary 0b1000001",
        "point U+0041", "name latin capital letter a", "utf8 41"]],
    ["U+1F600", "\u{1f600}", "\u{1f600}", ["decimal 128512", "hex 0x1f600",
        "binary 0b11111011000000000", "point U+1F600", "name grinning face",
        "utf8 f0 9f 98 80"]],
    ["72 105 as text", "Hi", "Hi", ["H U+0048 latin capital letter h",
        "i U+0069 latin small letter i"]],
    ["ord \u{1f44b}\u{1f3fd}", "128075 127997", "128075 127997", [
        "\u{1f44b} U+1F44B waving hand sign",
        "\u{1f3fd} U+1F3FD emoji modifier fitzpatrick type-4"]],
    ["char 10", "\u240a", "\n", ["decimal 10", "hex 0xa", "binary 0b1010", "point U+000A",
        "name line feed", "utf8 0a"]],
    ["char 0x301", "\u25cc\u0301", "\u0301", ["decimal 769", "hex 0x301",
        "binary 0b1100000001", "point U+0301", "name combining acute accent", "utf8 cc 81"]],
    ["char 8203", "zero width space", "\u200b", ["decimal 8203", "hex 0x200b",
        "binary 0b10000000001011", "point U+200B", "name zero width space", "utf8 e2 80 8b"]],
    ["char 0xac01", "\uac01", "\uac01", ["decimal 44033", "hex 0xac01",
        "binary 0b1010110000000001", "point U+AC01", "name hangul syllable gag",
        "utf8 ea b0 81"]],
    ["char 0x4e00", "\u4e00", "\u4e00", ["decimal 19968", "hex 0x4e00",
        "binary 0b100111000000000", "point U+4E00", "name cjk unified ideograph-4e00",
        "utf8 e4 b8 80"]],
    ["utf8 \u00e9", "c3 a9", "c3 a9", ["escaped \\xc3\\xa9", "percent %C3%A9",
        "bits 11000011 10101001", "points U+00E9", "length 2 bytes"]],
    ["utf-8 e\u0301", "65 cc 81", "65 cc 81", ["escaped \\x65\\xcc\\x81",
        "percent %65%CC%81", "bits 01100101 11001100 10000001", "points U+0065 U+0301",
        "length 3 bytes"]]
];

var notCharacters = ["char", "char ", "char 0xd800", "char 1114112",
    "U+41", "u+12345678", "0xzz as text", "kitty", "charm 65", "ord", "12*7"];

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    const parted = Common.parted;
    const subnets = SubnetCases.subnets;
    const naming = new XMLHttpRequest();
    naming.open("GET", fromRoot.url("names.txt"), false);
    naming.send();
    const names = Characters.prepare(naming.responseText);
    for (const [typed, headline, copy, rows] of characters) {
        asked++;
        const got = Characters.answer(typed, names);
        // Bytes name no character, so they say nothing of the names.
        const note = typed.startsWith("utf") ? "" : `names from Unicode ${names.version}`;
        if (!got || got.headline !== headline || got.copy !== copy
                || quoted(parted(got.rows)) !== quoted(rows) || got.note !== note)
            failures.push(`"${typed}" told ${quoted(got && [got.headline, got.copy,
                parted(got.rows), got.note])}, not ${quoted([headline, copy, rows])}`);
    }
    for (const typed of notCharacters) {
        asked++;
        const got = Characters.answer(typed, names);
        if (got)
            failures.push(`"${typed}" told ${quoted(got.headline)}, but nothing should`);
    }
    // Without the names, a character is told by its numbers alone.
    const unnamed = Characters.answer("ascii 65");
    asked++;
    if (!unnamed || unnamed.rows.some(r => r.id === "name") || unnamed.note !== "")
        failures.push(`"ascii 65" without names told ${quoted(unnamed)}`);
    // A character of several holds its own parts.
    const held = Characters.answer("72 105 as text", names);
    asked++;
    if (!held || quoted(parted(held.rows[0].parts)) !== quoted(["decimal 72", "hex 0x48",
            "binary 0b1001000", "point U+0048", "name latin capital letter h", "utf8 48"]))
        failures.push(`"72 105 as text" held ${quoted(held && parted(held.rows[0].parts))}`);
    // The answers asked before these leave what they ask alone, and
    // each of these leaves the other's.
    for (const [typed] of subnets.concat(characters)) {
        asked++;
        const sum = Numbers.answer(typed, undefined, Maths);
        const measure = Units.answer(typed, undefined, Maths);
        if (sum || measure)
            failures.push(`"${typed}" was answered ${quoted((sum || measure).headline)} `
                + "by the sums or the units first");
    }
    for (const [typed] of characters) {
        asked++;
        if (Subnets.answer(typed))
            failures.push(`"${typed}" was told as a network first`);
    }
    for (const [typed] of subnets) {
        asked++;
        if (Characters.answer(typed, names))
            failures.push(`"${typed}" was told as characters`);
    }
    return asked;
}
