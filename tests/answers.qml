// What the launcher's answers must make of what's typed, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-launcher-answers-what-is-asked` check copies Numbers.js,
// Addresses.js and the rest beside this file, with the character names
// character-names.py builds, and runs `qml -platform offscreen
// answers.qml`, with the shell's maths (Maths, libqalculate) on its import
// path, so sums and measures are worked out as the shell works them out.
// Each case names what's typed and what has to come back, or that nothing
// should.
import QtQuick
import Maths
import "Numbers.js" as Numbers
import "Addresses.js" as Addresses
import "Subnets.js" as Subnets
import "Characters.js" as Characters
import "Units.js" as Units
import "Zones.js" as Zones
import "Moment.js" as Moment
import "Keeping.js" as Keeping
import "Warmth.js" as Warmth
import "Here.js" as Here
import "Colours.js" as Colours
import "Asking.js" as Asking
import "Machine.js" as Machine
import "Heat.js" as Heat
import "Power.js" as Power
import "Awake.js" as Awake
import "Links.js" as Links
import "Keyboards.js" as Keyboards
import "KeyCounts.js" as KeyCounts
import "Standing.js" as Standing
import "Traffic.js" as Traffic
import "Sounds.js" as Sounds
import "Cameras.js" as Cameras
import "Warp.js" as Warp
import "WarpSeen.js" as WarpSeen
import "WarpSwitch.js" as WarpSwitch
import "Seen.js" as Seen
import "Interfaces.js" as Interfaces
import "Networkd.js" as Networkd
import "colour-cases.js" as ColourCases

QtObject {
    // What's typed, and the answer's headline.
    readonly property var sums: ({
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
    })
    // Division by zero has no value, so it isn't answered: the old engine
    // said infinity, which it isn't from both sides. What other answers
    // own stays theirs: addresses, a hardware address, a time, money,
    // measures, a colour and words.
    readonly property var notSums: ["2", "-5", "pi", "kitty", "ws 3", "1password", "12*",
        "(3", "sqrt", "3.5 in hex", "2fa", "e", "1/0", "8.8.8.8", "10.0.0.0/22",
        "3c:22:fb:12:34:56", "12:30", "100 USD in CAD", "5 km * 2", "350 f in c",
        "brightness 40", "volume 40", "time in utc", "#f53", "now", "command(\"true\")",
        "battery life", "2 + x"]

    // What's typed, and fields its reading must have.
    readonly property var addresses: ({
        "10.0.0.0/22": {
            version: 4, usable: 1022, first: "10.0.0.1", last: "10.0.3.254",
            broadcast: "10.0.3.255", mask: "255.255.252.0", network: "10.0.0.0/22"
        },
        "192.168.4.65/22": { network: "192.168.4.0/22", aligned: false },
        "8.8.8.8": { version: 4, prefix: null, text: "8.8.8.8" },
        "10.1.2.3/31": { usable: 2, first: "10.1.2.2", last: "10.1.2.3" },
        "0.0.0.0/0": { size: 4294967296, mask: "0.0.0.0" },
        "2001:db8::/48": {
            version: 6, bits: 80, first: "2001:db8::", last: "2001:db8:0:ffff:ffff:ffff:ffff:ffff",
            full: "2001:0db8:0000:0000:0000:0000:0000:0000"
        },
        "2001:0DB8:0000:0000:0001:0000:0000:0001": { text: "2001:db8::1:0:0:1" },
        "fe80::15b3:5aba:b3d4:6834%wlp0s20f3": { text: "fe80::15b3:5aba:b3d4:6834" },
        "::ffff:192.0.2.1": { text: "::ffff:c000:201" },
        "::": { text: "::" },
        "1:0:0:2:0:0:0:3": { text: "1:0:0:2::3" }
    })
    // What's typed, and the kind of range it's in.
    readonly property var kinds: ({
        "8.8.8.8": "public",
        "10.20.30.40": "private",
        "172.31.255.255": "private",
        "172.32.0.0": "public",
        "100.104.118.2": "shared, behind carrier NAT",
        "127.0.0.1": "loopback",
        "192.0.2.7": "documentation only",
        "224.0.0.251": "multicast",
        "2606:4700::1111": "public",
        "2001:db8::1": "documentation only",
        "fd01:5ca1:ab1e:8cda::1": "unique local",
        "fe80::1": "link-local",
        "::1": "loopback",
        "ff02::1": "multicast"
    })
    readonly property var notAddresses: ["1.2.3", "256.1.1.1", "1.2.3.4/33", "1:2:3", "a::b::c",
        "12345::", "kitty", "10.0.0.0/", "::/129"]

    // What's typed, the answer's headline, what Return copies, and its
    // parts, each its id (or a split's subnet) and what it says.
    readonly property var subnets: [
        ["10.0.0.0/22", "1,022 hosts", "10.0.0.0/22", ["range 10.0.0.0 \u2013 10.0.3.255",
            "mask 255.255.252.0", "wildcard 0.0.3.255", "network 10.0.0.0/22",
            "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254",
            "size 1,024 addresses"]],
        ["10.0.0.0 255.255.252.0", "1,022 hosts", "10.0.0.0/22", [
            "range 10.0.0.0 \u2013 10.0.3.255", "mask 255.255.252.0", "wildcard 0.0.3.255",
            "network 10.0.0.0/22", "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254",
            "size 1,024 addresses"]],
        ["10.0.0.5/22", "in 10.0.0.0/22", "10.0.0.0/22", ["range 10.0.0.0 \u2013 10.0.3.255",
            "mask 255.255.252.0", "wildcard 0.0.3.255", "network 10.0.0.0/22",
            "broadcast 10.0.3.255", "first 10.0.0.1", "last 10.0.3.254", "hosts 1,022",
            "size 1,024 addresses"]],
        ["ip 10.1.2.3/31", "in 10.1.2.2/31", "10.1.2.2/31", ["range 10.1.2.2 \u2013 10.1.2.3",
            "mask 255.255.255.254", "wildcard 0.0.0.1", "network 10.1.2.2/31",
            "first 10.1.2.2", "last 10.1.2.3", "hosts 2", "size 2 addresses"]],
        ["is 10.1.2.3 in 10.0.0.0/16", "no", "no", ["network 10.0.0.0/16",
            "range 10.0.0.0 \u2013 10.0.255.255"]],
        ["is 10.0.2.3 in 10.0.0.0 255.255.0.0?", "yes", "yes", ["network 10.0.0.0/16",
            "range 10.0.0.0 \u2013 10.0.255.255"]],
        ["is 10.0.4.0/24 in 10.0.0.0/22", "no", "no", ["network 10.0.0.0/22",
            "range 10.0.0.0 \u2013 10.0.3.255"]],
        ["is 2001:db8::5 in 2001:db8::/32", "yes", "yes", ["network 2001:db8::/32",
            "range 2001:db8:: \u2013 2001:db8:ffff:ffff:ffff:ffff:ffff:ffff"]],
        ["/22", "255.255.252.0", "255.255.252.0", ["wildcard 0.0.3.255", "hosts 1,022",
            "size 1,024 addresses"]],
        ["/48", "65,536 /64s", "65,536 /64s", ["size 2^80 addresses"]],
        ["10.0.0.0/22 in /24s", "4 /24s", "10.0.0.0/24\n10.0.1.0/24\n10.0.2.0/24\n10.0.3.0/24", [
            "10.0.0.0/24 10.0.0.0 \u2013 10.0.0.255", "10.0.1.0/24 10.0.1.0 \u2013 10.0.1.255",
            "10.0.2.0/24 10.0.2.0 \u2013 10.0.2.255", "10.0.3.0/24 10.0.3.0 \u2013 10.0.3.255"]],
        ["2001:db8::/63 into /64s", "2 /64s", "2001:db8::/64\n2001:db8:0:1::/64", [
            "2001:db8::/64 2001:db8:: \u2013 2001:db8::ffff:ffff:ffff:ffff",
            "2001:db8:0:1::/64 2001:db8:0:1:: \u2013 2001:db8:0:1:ffff:ffff:ffff:ffff"]],
        ["2001:db8::/48", "65,536 /64s", "2001:db8::/48", [
            "range 2001:db8:: \u2013 2001:db8:0:ffff:ffff:ffff:ffff:ffff",
            "size 2^80 addresses", "full 2001:0db8:0000:0000:0000:0000:0000:0000"]],
        ["2001:db8::1/48", "in 2001:db8::/48", "2001:db8::/48", [
            "range 2001:db8:: \u2013 2001:db8:0:ffff:ffff:ffff:ffff:ffff",
            "network 2001:db8::/48", "subnets 65,536 /64s", "size 2^80 addresses",
            "full 2001:0db8:0000:0000:0000:0000:0000:0001"]]
    ]
    readonly property var notSubnets: ["10.0.0.0 255.255.253.0", "/129", "is 10.0.0.1 in",
        "10.0.0.0/24 in /22s", "10.0.0.0/22 in /33s", "is kitty in 10.0.0.0/8", "/", "22"]

    // What's typed, the answer's headline, what Return copies, and its
    // rows: a part's id and what it says, or a character of several, what
    // it says and its name.
    readonly property var characters: [
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
    ]
    readonly property var notCharacters: ["char", "char ", "char 0xd800", "char 1114112",
        "U+41", "u+12345678", "0xzz as text", "kitty", "charm 65", "ord", "12*7"]

    // What's typed, and fields a hardware address's reading must have.
    readonly property var macs: ({
        "3c:22:fb:12:34:56": {
            oui: "3C22FB", dotted: "3c22.fb12.3456", local: false, multicast: false
        },
        "3C-22-FB-12-34-56": { colon: "3c:22:fb:12:34:56" },
        "3c22.fb12.3456": { colon: "3c:22:fb:12:34:56" },
        "da:a1:19:00:00:01": { local: true },
        "01:00:5e:00:00:fb": { multicast: true }
    })
    readonly property var notMacs: ["3c:22:fb:12:34", "3c:22-fb:12:34:56", "gg:22:fb:12:34:56",
        "3c22fb123456"]

    // What's typed, and the conversion's headline.
    readonly property var conversions: ({
        "5 km in mi": "3.10686 mi",
        "1 mi to km": "1.60934 km",
        "6 ft in cm": "182.88 cm",
        "3 in in cm": "7.62 cm",
        "3 inches to cm": "7.62 cm",
        "5 nautical miles in km": "9.26 km",
        "1e3 m in km": "1 km",
        "1 lb in kg": "0.453592 kg",
        "1,500 g as kg": "1.5 kg",
        "2 cups in ml": "473.176 mL",
        "100 km/h in mph": "62.1371 mph",
        "100 c in f": "212 \u00b0F",
        "350f to c": "176.667 \u00b0C",
        "0 kelvin in celsius": "-273.15 \u00b0C",
        "1 gib in mb": "1,073.74 MB",
        "10 gbit in gb": "1.25 GB",
        "1 year in days": "365.25 d",
        "90 deg in rad": "1.5708 rad",
        "180 degrees in turns": "0.5 turn",
        "1 kwh in kcal": "860.421 kcal",
        "1 atm in psi": "14.6959 psi",
        "25ms in s": "0.025 s",
        "25' in m": "7.62 m",
        // A measure alone is said in its kind's base unit, and a
        // temperature in the other scale people use.
        "25ms": "0.025 s",
        "5 km": "5,000 m",
        "3 ft": "0.9144 m",
        "2 hours": "7,200 s",
        "100\u00b0f": "37.7778 \u00b0C",
        // What the table can't, the maths works out.
        "5 parsecs in km": "154,283,879,074,568 km",
        "5 ft + 3 in in cm": "160.02 cm",
        "60 mph * 2 h": "120 mi",
        "1 gib / (10 mbit/s) in min": "14.3166 min",
        "3 cups + 2 tbsp in ml": "739.338 mL",
        "2 pints + 1 quart in l": "1.89271 L"
    })
    // A letter after a number is a word's end as often as a unit ("4k",
    // "3d", "1st"), so a measure alone needs a longer name.
    readonly property var notConversions: ["5 km in kg", "5 km in km", "100 usd in cad",
        "100 USD in CAD", "255 in hex", "km in mi", "kitty", "4k", "3d", "1st", "100 c",
        "ws 3", "2fa", "5 km + 3 kg", "1 m / 1 cm", "12*7", "5 m"]

    // What's typed, when (in UTC), and the time there, from the zones
    // zones.py built, told against Toronto.
    readonly property var times: [
        ["time in tokyo", "2026-01-15T12:00", "21:00"],
        ["london time", "2026-01-15T12:00", "12:00"],
        ["london time", "2026-07-15T12:00", "13:00"],
        ["time in the uk", "2026-07-15T12:00", "13:00"],
        ["time", "2026-01-15T12:00", "07:00"],
        ["time", "2026-07-15T12:00", "08:00"],
        ["what time is it in india?", "2026-01-15T12:00", "17:30"],
        ["kathmandu time", "2026-01-15T12:00", "17:45"],
        ["time in New York", "2026-01-15T12:00", "07:00"],
        ["pst time", "2026-01-15T12:00", "04:00"],
        ["time in south korea", "2026-01-15T12:00", "21:00"],
        ["time in utc", "2026-01-15T12:00", "12:00"],
        ["time in auckland", "2026-01-15T12:00", "01:00"],
        // Toronto's clocks go forward at 02:00 on 8 March 2026, 07:00 UTC.
        ["time", "2026-03-08T06:59", "01:59"],
        ["time", "2026-03-08T07:00", "03:00"]
    ]
    readonly property var notTimes: ["bedtime", "uptime", "timer", "bed time", "kitty",
        "time out"]

    function quoted(value) {
        return JSON.stringify(value);
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
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
        for (const typed in addresses) {
            asked++;
            const got = Addresses.read(typed);
            for (const field in addresses[typed])
                if (!got || got[field] !== addresses[typed][field])
                    failures.push(`"${typed}" read ${field} as ${quoted(got && got[field])}, `
                        + `not ${quoted(addresses[typed][field])}`);
        }
        for (const typed in kinds) {
            asked++;
            const got = Addresses.read(typed);
            if (!got || got.kind.kind !== kinds[typed])
                failures.push(`"${typed}" is ${quoted(got && got.kind.kind)}, `
                    + `not "${kinds[typed]}"`);
        }
        for (const typed of notAddresses) {
            asked++;
            if (Addresses.read(typed))
                failures.push(`"${typed}" read as an address, but it isn't one`);
        }
        for (const typed in macs) {
            asked++;
            const got = Addresses.mac(typed);
            for (const field in macs[typed])
                if (!got || got[field] !== macs[typed][field])
                    failures.push(`"${typed}" read ${field} as ${quoted(got && got[field])}, `
                        + `not ${quoted(macs[typed][field])}`);
        }
        for (const typed of notMacs) {
            asked++;
            if (Addresses.mac(typed))
                failures.push(`"${typed}" read as a hardware address, but it isn't one`);
        }

        // Each part as a line: its id, or a split's subnet or a character
        // of several, then what it says and its note.
        const parted = rows => rows.map(r => [r.id ?? r.name, r.side, r.note]
            .filter(said => said).join(" "));
        for (const [typed, headline, copy, rows] of subnets) {
            asked++;
            const got = Subnets.answer(typed);
            if (!got || got.headline !== headline || got.copy !== copy
                    || quoted(parted(got.rows)) !== quoted(rows))
                failures.push(`"${typed}" told ${quoted(got && [got.headline, got.copy,
                    parted(got.rows)])}, not ${quoted([headline, copy, rows])}`);
        }
        for (const typed of notSubnets) {
            asked++;
            const got = Subnets.answer(typed);
            if (got)
                failures.push(`"${typed}" told ${quoted(got.headline)}, but nothing should`);
        }
        // A split into more than it lists says how many it lists.
        const split = Subnets.answer("10.0.0.0/8 in /24s");
        asked++;
        if (!split || split.headline !== "65,536 /24s" || split.rows.length !== 256
                || split.note !== "the first 256")
            failures.push(`"10.0.0.0/8 in /24s" told ${quoted(split && [split.headline,
                split.rows.length, split.note])}`);

        const naming = new XMLHttpRequest();
        naming.open("GET", Qt.resolvedUrl("names.txt"), false);
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

        for (const typed in conversions) {
            asked++;
            const got = Units.answer(typed, undefined, Maths);
            if (!got || got.headline !== conversions[typed])
                failures.push(`"${typed}" converted to ${quoted(got && got.headline)}, `
                    + `not "${conversions[typed]}"`);
        }
        for (const typed of notConversions) {
            asked++;
            const got = Units.answer(typed, undefined, Maths);
            if (got)
                failures.push(`"${typed}" converted to ${quoted(got.headline)}, `
                    + "but nothing should");
        }
        const rated = Units.answer("5 km in mi", undefined, Maths);
        asked++;
        if (!rated || quoted(rated.rows.map(r => `${r.text} ${r.side}`))
                !== quoted(["1 km 0.621371 mi", "1 mi 1.60934 km"]))
            failures.push(`"5 km in mi" gave the rows ${quoted(rated && rated.rows)}`);
        // A temperature's third scale is named first, as a part is.
        const scaled = Units.answer("100 c in f", undefined, Maths);
        asked++;
        if (!scaled || quoted(scaled.rows.map(r => `${r.text} ${r.side}`))
                !== quoted(["Kelvin 373.15 K"]))
            failures.push(`"100 c in f" gave the rows ${quoted(scaled && scaled.rows)}`);

        const read = new XMLHttpRequest();
        read.open("GET", Qt.resolvedUrl("zones.jsonl"), false);
        read.send();
        const zones = Zones.prepare(read.responseText);
        const at = when => Date.parse(`${when}:00Z`);
        for (const [typed, when, time] of times) {
            asked++;
            const got = Zones.answer(typed, zones, at(when), "America/Toronto", 0);
            if (!got || got.headline !== time)
                failures.push(`"${typed}" at ${when} said ${quoted(got && got.headline)}, `
                    + `not "${time}"`);
        }
        for (const typed of notTimes) {
            asked++;
            const got = Zones.answer(typed, zones, at("2026-01-15T12:00"), "America/Toronto", 0);
            if (got)
                failures.push(`"${typed}" told ${quoted(got)}, but nothing should`);
        }
        // The day of the year, in a zone with summer time (the check runs
        // in Toronto's): either side of the spring change, and the last.
        for (const [year, month, date, day] of [[2026, 0, 1, 1], [2026, 2, 7, 66],
                [2026, 2, 9, 68], [2026, 9, 22, 295], [2024, 11, 31, 366]]) {
            asked++;
            const got = Zones.dayOfYear(new Date(year, month, date));
            if (got !== day)
                failures.push(`${year}-${month + 1}-${date} was day ${got}, not ${day}`);
        }
        const told = Zones.answer("time in auckland", zones, at("2026-01-15T12:00"),
            "America/Toronto", 0);
        asked++;
        // Each row a part, its value, then its note.
        const lines = answer => answer.rows.map(r => [r.part, r.text, r.note].filter(s => s)
            .join(" "));
        if (!told || quoted(lines(told)) !== quoted(["day Friday 16 January tomorrow",
                "apart 18 h ahead", "zone Pacific/Auckland NZDT, UTC+13"]))
            failures.push(`"time in auckland" gave the rows ${quoted(told && told.rows)}`);
        // Here, the day needs no "today", and UTC is said once.
        const cases = [
            ["time", ["day Thursday 15 January", "zone America/Toronto EST, UTC\u22125"]],
            ["time in utc", ["day Thursday 15 January today", "apart 5 h ahead", "zone UTC"]],
            ["time in kathmandu", ["day Thursday 15 January today", "apart 10 h 45 min ahead",
                "zone Asia/Kathmandu UTC+5:45"]]
        ];
        for (const [typed, expected] of cases) {
            asked++;
            const got = Zones.answer(typed, zones, at("2026-01-15T12:00"), "America/Toronto", 0);
            if (!got || quoted(lines(got)) !== quoted(expected))
                failures.push(`"${typed}" gave the rows ${quoted(got && lines(got))}`);
        }
        asked++;
        const away = Zones.answer("time in narnia", zones, at("2026-01-15T12:00"),
            "America/Toronto", 0);
        if (!away || away.unknown !== "narnia")
            failures.push(`"time in narnia" gave ${quoted(away)}, not a place to look up`);
        asked++;
        const change = Zones.nextChange(zones.zones["America/Toronto"],
            at("2026-11-04T22:00") / 1000);
        if (!change || new Date(change.when).toISOString() !== "2027-03-14T07:00:00.000Z"
            || change.abbreviation !== "EDT" || change.minutes !== -240)
            failures.push(`Toronto next changed at ${quoted(change)}`);

        // The clock's and the calendar's figures: the equinoxes, the
        // solstices and the moons against the instants USNO publishes, to
        // the minute, and the rest worked by hand.
        const minute = ms => new Date(ms).toISOString().slice(0, 16);
        // The words Keeping.js is given, as the vocabulary's Clock and
        // Servers say them (launcher/words.json).
        const KeepingCases = {
            clock: { onTime: "on time", fast: "{offset} fast", slow: "{offset} slow",
                gains: "gains {seconds} a day", loses: "loses {seconds} a day" },
            servers: { reach: "{count} of the last 8", unreachable: "out of reach",
                unusable: "set aside", aside: "set aside: {why}",
                "standing:chosen": "chosen", "standing:tooVariable": "set aside: too variable",
                "standing:disagrees": "set aside: disagrees",
                "why:tooVariable": "too variable", "why:disagrees": "disagrees with the others",
                "why:untrusted": "disagrees with the trusted servers",
                "why:tooFewSamples": "too few samples yet",
                "why:chosen": "the clock is kept by it",
                "why:combined": "combined with the chosen one",
                silent: "not answering", unsampled: "no sample yet",
                "kind:reference": "reference clock", interleaved: "interleaved",
                basic: "basic", authenticated: "authenticated",
                unauthenticated: "unauthenticated", exchange: "{mode} \u00b7 {authentication}",
                allGood: "{sent} sent, all good",
                packets: "{sent} sent \u00b7 {received} received \u00b7 {valid} valid \u00b7 "
                    + "{good} good",
                offline: "{count} offline", burst: "{count} in a burst",
                unresolved: "{count} unresolved" }
        };
        // A night in Toronto's November as the daemon says it, the clock
        // given as hours and minutes in local time and a kelvin bare, so
        // the cases read the same wherever they run.
        const local = (year, month, day, hour, minute) =>
            new Date(year, month - 1, day, hour, minute).getTime();
        const WarmthCases = {
            night: { kelvin: 4000, followsTheRoom: true, seesColour: true, roomWhite: 3200,
                nightLight: true, nightWhite: 4000, sunset: local(2026, 11, 8, 16, 59),
                sunrise: local(2026, 11, 9, 7, 4) },
            evening: local(2026, 11, 8, 21, 0),
            noon: local(2026, 11, 8, 12, 0),
            said: { until: "night light until {time}", from: "night light from {time}",
                waits: "night light waits for where you are", following: "following the room",
                own: "the screen's own white", tomorrow: "tomorrow", yesterday: "yesterday" },
            told: {
                clock: at => {
                    const moment = new Date(at);
                    const two = value => String(value).padStart(2, "0");
                    return `${two(moment.getHours())}:${two(moment.getMinutes())}`;
                },
                kelvin: value => `${value} K`
            }
        };
        const evening = at("2026-11-04T22:00");
        const day = new Date(2026, 10, 4);
        const moon = Moment.moon(evening);
        const season = Moment.season(evening, 43.67);
        // Location's words, as words.json says them, and its numbers in an
        // English locale's marks.
        const hereSaid = { metres: "within {n} m", kilometres: "within {n} km", north: "N",
            south: "S", east: "E", west: "W", nearEdge: "near a time zone's edge",
            foundWithin: "found {within}", via: "found \u00b7 location",
            viaHand: "set by hand \u00b7 location", "via-wifi": "via Wi-Fi \u00b7 location",
            "via-gps": "via GPS \u00b7 location", altitude: "{n} m", speed: "{n} km/h",
            heading: "{n}\u00b0", "no-altitude": "altitude", "no-speed": "speed",
            "no-heading": "heading", or: "or" };
        const hereTells = { leftOut: "Its source gives no {missing}.",
            "leftOut-wifi": "Wi-Fi gives no {missing}.", "leftOut-gps": "GPS gave no {missing}." };
        const leftOut = here => Here.leftOut(here, hereSaid, hereTells);
        // A fix GPS found, with as many of its three figures as given.
        const gps = (altitude, speed, heading) => ({ latitude: 12.3457, longitude: -98.7654,
            accuracy: 5, when: 1000, source: "gps", altitude, speed, heading });
        // A fix in another zone, by its width and whether its zone holds.
        const unfollowed = (accuracy, zoneHolds) => Here.unfollowed({ accuracy, zoneHolds },
            hereSaid, hereTold);
        const hereTold = { number: (value, decimals) => Qt.locale("en_CA").toString(value, "f",
            decimals) };
        // A plate's first two facts, when it was found and how near.
        const nearAndWhen = [{ id: "located", value: "12 min ago" },
            { id: "accuracy", value: "within 26 km" }];
        const figures = [
            ["UTC", Moment.utcClock(evening + 7000), "22:00:07"],
            ["Unix time", Moment.unix(evening), 1793829600],
            ["TAI", Moment.tai(evening), "22:00:37"],
            ["GPS time", quoted(Moment.gps(evening)), quoted({ week: 2443, seconds: 338418 })],
            // As Time says it, in the launcher's own marks (Y2YBTS's review of #2489).
            ["GPS time said", Moment.gpsSaid(evening, "week {week} \u00b7 {seconds} s",
                { group: "\u202f", point: "," }), "week 2443 \u00b7 338\u202f418 s"],
            ["a longitude said", Moment.longitudeSaid(-79.42, ","), "79,42\u00b0 W"],
            ["a longitude east said", Moment.longitudeSaid(10.5, "."), "10.5\u00b0 E"],
            ["Greenwich sidereal time", Moment.sidereal(evening, 0), "00:56:37"],
            ["a date this year, without its year", Moment.day(day, evening),
                "Wednesday 4 November"],
            ["an eyebrow this year, without its year", Moment.eyebrow(day, evening),
                "Wednesday 4 November"],
            // A seven-day forecast from 28 December reaches into next year.
            ["an eyebrow of another year, with its year",
                Moment.eyebrow(new Date(2027, 0, 1), evening), "Friday 1 January 2027"],
            ["a date of another year, with its year", Moment.day(new Date(2027, 0, 1), evening),
                "Friday 1 January 2027"],
            ["ISO date", Moment.iso(day), "2026-11-04"],
            ["ISO week date", Moment.isoWeekDate(day), "2026-W45-3"],
            ["ISO week date across a year", Moment.isoWeekDate(new Date(2027, 0, 1)),
                "2026-W53-5"],
            ["ordinal date", Moment.ordinal(day), 308],
            ["quarter", quoted(Moment.quarter(day)), quoted({ quarter: 4, day: 35, days: 92 })],
            ["Unix day", Moment.unixDay(day), 20761],
            ["Julian day number", Moment.dayNumber(day), 2461349],
            ["modified Julian day", Moment.modified(day), 61348],
            ["March equinox", minute(Moment.solstice(2026, 0)), "2026-03-20T14:45"],
            ["June solstice", minute(Moment.solstice(2026, 1)), "2026-06-21T08:24"],
            ["September equinox", minute(Moment.solstice(2026, 2)), "2026-09-23T00:05"],
            ["December solstice", minute(Moment.solstice(2026, 3)), "2026-12-21T20:50"],
            ["season in the north", season.season, "autumn"],
            ["season in the south", Moment.season(evening, -33.9).season, "spring"],
            ["season's end", minute(season.ends), "2026-12-21T20:50"],
            ["moon's phase", moon.phase, "waningCrescent"],
            ["moon lit", Math.round(moon.lit * 100), 19],
            ["next new moon", minute(moon.new), "2026-11-09T07:02"],
            ["next full moon", minute(moon.full), "2026-11-24T14:53"],
            ["full moon on its day", Moment.moon(at("2026-11-24T12:00")).phase, "full"],
            // How chrony's keeping is worded: seconds in the unit that reads
            // them, signed with a true minus; how far a clock is out and how
            // fast it runs; a span; a hardware clock's drift in a day; how
            // many polls were answered; and where a server stands. Every
            // measure keeps three significant digits, trailing zeros too, in
            // the settings' decimal mark (VKCF3Z, 2026-11-07).
            ["nanoseconds", Keeping.seconds(2.98e-8, false), "29.8 ns"],
            ["microseconds, signed", Keeping.seconds(-0.000388, true), "\u2212388 \u00b5s"],
            ["milliseconds", Keeping.seconds(0.036112, false), "36.1 ms"],
            ["milliseconds, in a comma's mark", Keeping.seconds(0.036112, false, ","),
                "36,1 ms"],
            ["a microsecond short of a millisecond", Keeping.seconds(0.0009996, false),
                "1.00 ms"],
            ["seconds", Keeping.seconds(1.5, true), "+1.50 s"],
            ["under chronyd's finest", Keeping.seconds(1e-10, true), "0 s"],
            ["a clock behind", Keeping.out(-0.000012, KeepingCases.clock), "12.0 \u00b5s slow"],
            ["a clock ahead", Keeping.out(0.31, KeepingCases.clock), "310 ms fast"],
            ["a clock on time", Keeping.out(0, KeepingCases.clock), "on time"],
            ["a clock running fast", Keeping.runs(6.267, KeepingCases.clock), "6.27 ppm fast"],
            ["a clock running slow", Keeping.runs(-3.226, KeepingCases.clock), "3.23 ppm slow"],
            ["a skew of ten", Keeping.ppm(9.861, false), "9.86 ppm"],
            ["a span in minutes", Keeping.span(1024), "17 min 4 s"],
            ["a span in hours", Keeping.span(86400), "24 h"],
            ["a span under a second", Keeping.span(0.5), "0.5 s"],
            ["a drift in a day", Keeping.daily(2.1, KeepingCases.clock), "gains 181 ms a day"],
            ["a loss in a day", Keeping.daily(-0.5, KeepingCases.clock), "loses 43.2 ms a day"],
            ["polls answered", Keeping.reach(0o375, KeepingCases.servers), "7 of the last 8"],
            ["the next poll", Keeping.nextPoll({ poll: 64, lastSample: 12 }), 52],
            ["a chosen server", Keeping.standing({ standing: "chosen", reach: 255 },
                KeepingCases.servers), "chosen"],
            ["a server out of reach", Keeping.standing({ standing: "unusable", reach: 0 },
                KeepingCases.servers), "out of reach"],
            ["a server set aside, and why", Keeping.standing({ standing: "unusable", reach: 3,
                chosen: { why: "tooFewSamples" } }, KeepingCases.servers),
                "set aside: too few samples yet"],
            ["a server too variable", Keeping.standing({ standing: "tooVariable", reach: 255 },
                KeepingCases.servers), "set aside: too variable"],
            // Why is said only where the note doesn't say it (VKCF3Z, 2026-11-08).
            ["a chosen server's why, which its note says", Keeping.whyUnsaid({ standing: "chosen",
                reach: 255, chosen: { why: "chosen" } }, KeepingCases.servers), ""],
            ["a server set aside's why, which its note says", Keeping.whyUnsaid({
                standing: "unusable", reach: 3, chosen: { why: "tooFewSamples" } },
                KeepingCases.servers), ""],
            ["an agreeing server's why, which its note doesn't say", Keeping.whyUnsaid({
                standing: "combined", reach: 255, chosen: { why: "combined" } },
                KeepingCases.servers), "combined with the chosen one"],
            ["a too variable server's why, which its note says", Keeping.whyUnsaid({
                standing: "tooVariable", reach: 255, chosen: { why: "tooVariable" } },
                KeepingCases.servers), ""],
            ["a disagreeing server's why, which its note says", Keeping.whyUnsaid({
                standing: "disagrees", reach: 255, chosen: { why: "disagrees" } },
                KeepingCases.servers), ""],
            ["an untrusted server's why, which says more than its note", Keeping.whyUnsaid({
                standing: "disagrees", reach: 255, chosen: { why: "untrusted" } },
                KeepingCases.servers), "disagrees with the trusted servers"],
            ["a server set aside and out of reach keeps its why", Keeping.whyUnsaid({
                standing: "unusable", reach: 0, chosen: { why: "tooFewSamples" } },
                KeepingCases.servers), "too few samples yet"],
            // The fact the row's side is made from tells only its note, and
            // goes with none; one that only reads the same stays.
            ["a server's facts beside its side", Keeping.besideSide([
                { id: "offset", side: "+12.0 \u00b5s", note: "measured +11.0 \u00b5s" },
                { id: "estimate", side: "+12.0 \u00b5s", note: "" },
                { id: "stratum", side: "2", note: "" }], "offset")
                .map(each => `${each.id} ${each.side}`).join(", "),
                "offset measured +11.0 \u00b5s, estimate +12.0 \u00b5s, stratum 2"],
            ["a server's side made from a fact with no note", Keeping.besideSide([
                { id: "offset", side: "+12.0 \u00b5s", note: "" },
                { id: "stratum", side: "2", note: "" }], "offset")
                .map(each => each.id).join(", "), "stratum"],
            // A server's side is never empty, and its note gives way where
            // the side says it's out of reach.
            ["a server's side, sampled", Keeping.side({ lastSample: 12, offset: 0.000012,
                reach: 255 }, KeepingCases.servers), "+12.0 \u00b5s"],
            ["a server's side, silent", Keeping.side({ lastSample: null, reach: 0 },
                KeepingCases.servers), "not answering"],
            ["a server's side, answering with no sample yet", Keeping.side({ lastSample: null,
                reach: 1 }, KeepingCases.servers), "no sample yet"],
            ["a silent server's note", Keeping.note({ standing: "unusable", lastSample: null,
                reach: 0 }, KeepingCases.servers), ""],
            ["a server once heard, now out of reach", Keeping.note({ standing: "unusable",
                lastSample: 900, reach: 0 }, KeepingCases.servers), "out of reach"],
            ["a server answering with no sample yet still says where it stands", Keeping.note({
                standing: "combined", lastSample: null, reach: 1 },
                { "standing:combined": "agrees" }), "agrees"],
            // The screen's Warmth, from what the backlight daemon says: what
            // holds it, by night, by day, waiting for a place, or the room;
            // and its figures, the room's white where a sensor sees it, then
            // the night's, a day said only when it isn't today (VKCF3Z,
            // 2026-11-08).
            ["nothing warming the screen", Warmth.of([]), null],
            ["a daemon before the night light", quoted(Warmth.of([{ kind: "warmth",
                kelvin: 6500, follows_the_room: true }])), quoted({ kelvin: 6500,
                followsTheRoom: true,
                seesColour: false, roomWhite: null, nightLight: false, nightWhite: null,
                sunset: null, sunrise: null })],
            ["the night light, as the machine daemon serves it", quoted(Warmth.of([
                { kind: "backlight", device: "panel" }, { kind: "warmth", kelvin: 3400,
                    follows_the_room: false, sees_colour: true, room_white_kelvin: 5100,
                    night_light: true, night_white_kelvin: 3400, sunset_ms: 1000,
                    sunrise_ms: 2000 }])), quoted({ kelvin: 3400, followsTheRoom: false,
                seesColour: true, roomWhite: 5100, nightLight: true, nightWhite: 3400,
                sunset: 1000, sunrise: 2000 })],
            ["by night", Warmth.note(WarmthCases.night, WarmthCases.evening, WarmthCases.said,
                WarmthCases.told), "night light until 07:04"],
            ["by day", Warmth.note(WarmthCases.night, WarmthCases.noon, WarmthCases.said,
                WarmthCases.told), "night light from 16:59"],
            ["waiting for a place", Warmth.note(Object.assign({}, WarmthCases.night,
                { sunset: null, sunrise: null }), WarmthCases.noon, WarmthCases.said,
                WarmthCases.told), "night light waits for where you are"],
            ["following the room", Warmth.note(Object.assign({}, WarmthCases.night,
                { nightLight: false }), WarmthCases.evening, WarmthCases.said, WarmthCases.told),
                "following the room"],
            ["the screen's own white", Warmth.note(Object.assign({}, WarmthCases.night,
                { nightLight: false, seesColour: false }), WarmthCases.evening,
                WarmthCases.said, WarmthCases.told), "the screen's own white"],
            ["its figures by night", quoted(Warmth.figures(WarmthCases.night,
                WarmthCases.evening, WarmthCases.said, WarmthCases.told)), quoted([
                { id: "room", side: "3200 K" },
                { id: "sunrise", side: "07:04 \u00b7 tomorrow" },
                { id: "sunset", side: "16:59" }, { id: "night", side: "4000 K" }])],
            ["by day, the sunset comes first, and no room's white without a sensor",
                Warmth.figures(Object.assign({}, WarmthCases.night, { seesColour: false }),
                    WarmthCases.noon, WarmthCases.said, WarmthCases.told)
                    .map(figure => figure.id).join(" "), "sunset sunrise night"],
            // What a server is asked as, how its exchange went, its packets
            // saying only what differs, its score, its delay's asymmetry,
            // and what the servers' activity says beyond their count, a
            // part with nothing in it giving way (VKCF3Z, 2026-11-07).
            ["a reference clock", Keeping.mode({ kind: "reference" }, KeepingCases.servers),
                "reference clock"],
            ["an interleaved exchange, authenticated", Keeping.exchanged({ interleaved: true,
                authenticated: true }, KeepingCases.servers),
                "interleaved \u00b7 authenticated"],
            ["a basic exchange", Keeping.exchanged({ interleaved: false,
                authenticated: false }, KeepingCases.servers), "basic \u00b7 unauthenticated"],
            ["every packet good", Keeping.packets({ sent: 13, received: 13, valid: 13, good: 13 },
                KeepingCases.servers), "13 sent, all good"],
            ["packets lost", Keeping.packets({ sent: 13, received: 12, valid: 12, good: 11 },
                KeepingCases.servers),
                "13 sent \u00b7 12 received \u00b7 12 valid \u00b7 11 good"],
            ["a score", Keeping.score(1), "1.00"],
            ["no score, as chronyd's NaN", Keeping.score(null), ""],
            ["no asymmetry", Keeping.asymmetry(0), "0"],
            ["an asymmetry", Keeping.asymmetry(-0.125), "\u22120.125"],
            ["activity with nothing beyond the count", Keeping.activity({ online: 2,
                offline: 0, burstOnline: 0, burstOffline: 0, unresolved: 0 },
                KeepingCases.servers).join(", "), ""],
            ["activity beyond the count", Keeping.activity({ online: 1, offline: 1,
                burstOnline: 1, burstOffline: 1, unresolved: 1 }, KeepingCases.servers)
                .join(", "), "1 offline, 2 in a burst, 1 unresolved"],
            // Where the laptop is claims no more places than it knows, the
            // nearest order of magnitude to its accuracy: four under 35 m,
            // three under 350 m, two under 3.5 km, one under 35 km, else
            // none; how near is said to three figures, rounded up; and when
            // it was found is how long ago within a day, else its day and
            // time (VKCF3Z, 2026-11-08).
            ["a position to 5 m", Here.coordinates({ latitude: 12.345_678,
                longitude: -98.765_4, accuracy: 5 }, hereSaid),
                "12.3457\u00b0 N \u00b7 98.7654\u00b0 W"],
            ["a position to 80 m", Here.coordinates({ latitude: 12.345_678,
                longitude: -98.765_4, accuracy: 80 }, hereSaid),
                "12.346\u00b0 N \u00b7 98.765\u00b0 W"],
            ["a position to 100 m", Here.coordinates({ latitude: 12.345_678,
                longitude: -98.765_4, accuracy: 100 }, hereSaid),
                "12.346\u00b0 N \u00b7 98.765\u00b0 W"],
            ["a position to 5 km, south and east", Here.coordinates({ latitude: -12.345_6,
                longitude: 123.456_7, accuracy: 5000 }, hereSaid),
                "12.3\u00b0 S \u00b7 123.5\u00b0 E"],
            ["a position to 500 km, in whole degrees", Here.coordinates({ latitude: 12.35,
                longitude: -98.76, accuracy: 500_000 }, hereSaid), "12\u00b0 N \u00b7 99\u00b0 W"],
            ["no hemisphere at none", Here.coordinates({ latitude: -0.04, longitude: 0.04,
                accuracy: 26_000 }, hereSaid), "0.0\u00b0 \u00b7 0.0\u00b0"],
            ["near, in metres", Here.within(80, hereSaid, hereTold), "within 80 m"],
            ["near, a metre short of a kilometre", Here.within(999.5, hereSaid, hereTold),
                "within 1 km"],
            ["near, a metre past one", Here.within(1001, hereSaid, hereTold), "within 1.01 km"],
            ["near, in kilometres", Here.within(26_000, hereSaid, hereTold), "within 26 km"],
            ["near, in metres to three figures", Here.within(100.2, hereSaid, hereTold),
                "within 101 m"],
            ["near, a thousand kilometres on, to three figures",
                Here.within(1_234_000, hereSaid, hereTold), "within 1,240 km"],
            ["located within the day", Here.located(1000, 1000 * 1000 + 720_000,
                { ago: (at, now) => `${(now - at) / 60_000} min ago` }), "12 min ago"],
            ["located days ago", Here.located(1000, 1000 * 1000 + 3 * 86_400_000,
                { day: () => "Saturday", clock: () => "06:46" }), "Saturday 06:46"],
            ["how near goes from the plate while the note says it",
                JSON.stringify(Here.besideNote(nearAndWhen, true).map(fact => fact.id)),
                JSON.stringify(["located"])],
            ["and stays while the note says the zone instead",
                JSON.stringify(Here.besideNote(nearAndWhen, false).map(fact => fact.id)),
                JSON.stringify(["located", "accuracy"])],
            ["a circle a zone's line crosses isn't followed", unfollowed(3000, false),
                "near a time zone's edge"],
            ["one wider than a region says its width", unfollowed(80_000, true),
                "found within 80 km"],
            ["one that's followed, or held put back, says nothing more",
                unfollowed(26_000, true), ""],
            ["one unsaid round its circle is followed only within 10 km",
                `${unfollowed(9000, null)}|${unfollowed(26_000, null)}`, "|found within 26 km"],
            ["one of no width says nothing", unfollowed(null, true), ""],
            // Its plate's eyebrow says what found it, its facts the
            // figures only some sources give, and its body what its
            // source couldn't (VKCF3Z, 2026-11-09).
            ["found by Wi-Fi", Here.eyebrow({ source: "wifi" }, hereSaid),
                "via Wi-Fi \u00b7 location"],
            ["found by a source unknown", Here.eyebrow({ source: "galileo" }, hereSaid),
                "found \u00b7 location"],
            ["found by none named", Here.eyebrow({ source: null }, hereSaid),
                "found \u00b7 location"],
            ["set by hand, though a source is named",
                Here.eyebrow({ source: "wifi", hand: true }, hereSaid),
                "set by hand \u00b7 location"],
            ["GPS's figures follow where on a map, speed in km/h",
                Here.facts(gps(212.4, 4.26, 274.6), 1000, hereSaid,
                    Object.assign({ ago: () => "now" }, hereTold))
                    .map(fact => `${fact.id} ${fact.value}`).slice(2).join("|"),
                "coordinates 12.3457\u00b0 N \u00b7 98.7654\u00b0 W|altitude 212 m|speed 15.3 km/h|"
                    + "heading 275\u00b0"],
            ["a speed of none is held, and said", Here.facts(gps(null, 0, null), 1000,
                hereSaid, Object.assign({ ago: () => "now" }, hereTold))
                .filter(fact => ["altitude", "speed", "heading"].includes(fact.id))
                .map(fact => fact.value).join("|"), "0 km/h"],
            ["a walk's speed keeps three figures, a highway's none past them, a "
                + "round one its zeros", [1.2, 27.8, 10].map(speed => Here.facts(gps(null, speed, null), 1000, hereSaid,
                    Object.assign({ ago: () => "now" }, hereTold))
                    .find(fact => fact.id === "speed").value).join("|"),
                "4.32 km/h|100 km/h|36.0 km/h"],
            ["Wi-Fi gives none of the three", leftOut({ source: "wifi" }),
                "Wi-Fi gives no altitude, speed or heading."],
            ["GPS gave two of them not", leftOut(gps(212, null, null)),
                "GPS gave no speed or heading."],
            ["GPS gave one of them not", leftOut(gps(212, 4, null)),
                "GPS gave no heading."],
            ["all three given leaves nothing to say", leftOut(gps(212, 4, 275)),
                ""],
            ["a source unnamed is said as its source", leftOut({ source: null }),
                "Its source gives no altitude, speed or heading."],
            ["one set by hand was found by no source",
                leftOut({ source: null, hand: true }), ""]
        ];
        for (const [what, got, expected] of figures) {
            asked++;
            if (got !== expected)
                failures.push(`${what} was ${quoted(got)}, not ${quoted(expected)}`);
        }

        for (const [typed, headline, rows] of ColourCases.answers) {
            asked++;
            const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
            const said = got && got.rows.map(r => [r.text, r.side, r.note].filter(s => s)
                .join(" "));
            if (!got || got.headline !== headline || quoted(said) !== quoted(rows))
                failures.push(`"${typed}" gave ${quoted(got && got.headline)} and `
                    + `${quoted(said)}, not "${headline}" and ${quoted(rows)}`);
        }
        asked++;
        const [typed, worn] = ColourCases.swatch;
        const swatched = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
        if (!swatched || swatched.rows[0].swatch !== worn)
            failures.push(`"${typed}" wore ${quoted(swatched && swatched.rows[0].swatch)}`);
        for (const typed of ColourCases.not) {
            asked++;
            const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
            if (got)
                failures.push(`"${typed}" gave ${quoted(got.headline)}, but nothing should`);
        }

        // What a URL said stands while it's asked, then for its fresh
        // time after an answer, or the failure's time after a failure, and
        // is asked again after either.
        const standing = [
            ["nothing asked yet", null, 0, false],
            ["still being asked", { pending: true, at: 0 }, 60000, true],
            ["an answer, fresh", { at: 0 }, 59999, true],
            ["an answer, stale", { at: 0 }, 60000, false],
            ["a failure, fresh", { failed: true, at: 0 }, 9999, true],
            ["a failure, stale", { failed: true, at: 0 }, 10000, false]
        ];
        for (const [what, entry, now, stands] of standing) {
            asked++;
            if (Asking.stands(entry, now, 60000, 10000) !== stands)
                failures.push(`${what} ${stands ? "was asked again" : "wasn't asked again"}`);
        }

        // One still being asked is given up on once its patience has
        // passed, and nothing else ever is.
        const overdue = [
            ["nothing asked yet", null, 0, null],
            ["just asked", { pending: true, at: 1000 }, 1000, 5000],
            ["nearly given up on", { pending: true, at: 1000 }, 5999, 1],
            ["given up on", { pending: true, at: 1000 }, 6000, 0],
            ["long given up on", { pending: true, at: 1000 }, 60000, 0],
            ["answered", { at: 1000 }, 60000, null],
            ["failed", { failed: true, at: 1000 }, 60000, null]
        ];
        for (const [what, entry, now, left] of overdue) {
            asked++;
            const got = Asking.overdueIn(entry, now, 5000);
            if (got !== left)
                failures.push(`${what} was given up on in ${got}, not ${left}`);
        }

        // The machine's figures, from its files as the kernel writes them.
        const machine = (what, got, expected) => {
            asked++;
            if (quoted(got) !== quoted(expected))
                failures.push(`${what} gave ${quoted(got)}, not ${quoted(expected)}`);
        };
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

        // How things stand in the foot, as room runs short: the least of
        // each, then each its glyph by rank, and no glyph ever gives way,
        // each a character wide here, two between.
        const fact = (sign, glance, least, collapse) => ({ sign, glance, least, entry: { collapse } });
        const facts = [fact("", "17:30", "17:30", null), fact("", "Sat 24 Oct", "Sat 24 Oct", 3),
            fact("B", "50% 0:02 left", "50%", 20), fact("N", "", "", 10), fact("V", "40%", "", 0),
            fact("R", "$9 today", "$9", 5), fact("R", "2 working", "2", 6)];
        const footFor = room => Standing.fitted(facts, told => told.reduce((sum, one) => sum
            + one.said.length, 0) + 2 * Math.max(0, told.length - 1) <= room)
            .map(one => one.said).join("  ");
        for (const [room, said] of [
            [100, "17:30  Sat 24 Oct  B 50% 0:02 left  N  V 40%  R $9 today  2 working"],
            [39, "17:30  Sat 24 Oct  B 50%  N  V  R $9  2"],
            // The date has no glyph, so it gives way where it'd shrink.
            [30, "17:30  B 50%  N  V  R $9  2"],
            // The day's spend and the agents as their glyph are one robot.
            [22, "17:30  B 50%  N  V  R"],
            [17, "17:30  B  N  V  R"],
            // Every glyph stays, however short the room, as the time does.
            [12, "17:30  B  N  V  R"],
            [3, "17:30  B  N  V  R"]
        ])
            machine(`how things stand in ${room}`, footFor(room), said);
        // Once the foot has had to read shorter, it reads no longer until
        // the launcher closes, though there's room again.
        const roomy = told => told.reduce((sum, one) => sum + one.said.length, 0)
            + 2 * Math.max(0, told.length - 1) <= 100;
        const tight = Standing.fittedFrom(facts, told => told.reduce((sum, one) => sum
            + one.said.length, 0) + 2 * Math.max(0, told.length - 1) <= 22, 0);
        machine("short of room, the foot reads shorter", tight.told.map(one => one.said).join("  "),
            "17:30  B 50%  N  V  R");
        machine("and with room again, no longer", Standing.fittedFrom(facts, roomy, tight.stage)
            .told.map(one => one.said).join("  "), "17:30  B 50%  N  V  R");
        machine("from the first way, as long as there's room", Standing.fittedFrom(facts, roomy, 0)
            .stage, 0);
        machine("a way past the last is the last", Standing.fittedFrom(facts, roomy, 99)
            .told.map(one => one.said).join("  "), "17:30  B  N  V  R");
        // The keys in the room the facts leave them, Return first as told:
        // those furthest from Return give way first, never Return nor
        // Space ? just before it, each key its letters wide here, two
        // between, as visual's foot over a full list at 2.
        const visualKeys = [["\u21b5", "open 2"], [":", "commands"], ["d", "close"], ["y", "copy"],
            ["Space ?", "keys"]];
        const keysWide = keys => keys.reduce((sum, key) => sum + `${key[0]} ${key[1]}`.length, 0)
            + 2 * Math.max(0, keys.length - 1);
        for (const [room, kept] of [
            [60, "\u21b5 : d y Space ?"],
            [44, "\u21b5 d y Space ?"],
            [33, "\u21b5 y Space ?"],
            [26, "\u21b5 Space ?"],
            [5, "\u21b5 Space ?"]
        ])
            machine(`the foot's keys in ${room}`, Standing.keysFitted(visualKeys, keysWide, room)
                .map(key => key[0]).join(" "), kept);
        machine("Return alone stays", Standing.keysFitted([["\u21b5", "open"]], keysWide, 0)
            .length, 1);
        // On the find they give way by need instead, the words named first
        // going first, in their order, wherever they're drawn: the
        // completion, whose ghost the line shows, then next time, and
        // Tab in never, though it's drawn furthest from Return so nothing
        // slides as it comes and goes (VKCF3Z, 2026-11-06).
        const findKeys = [["\u21b5", "start"], ["Ctrl+]", "in"], ["Ctrl+Y", "complete"],
            ["Super+\u21b5", "next time"]];
        for (const [room, kept] of [
            [54, "\u21b5 Ctrl+] Ctrl+Y Super+\u21b5"],
            [40, "\u21b5 Ctrl+] Super+\u21b5"],
            [30, "\u21b5 Ctrl+]"],
            [5, "\u21b5 Ctrl+]"]
        ])
            machine(`the find's keys in ${room}`, Standing.keysFitted(findKeys, keysWide, room,
                ["complete", "next time"]).map(key => key[0]).join(" "), kept);
        // The foot is told the selected row's d and y again only where
        // they'd say something new.
        const notes = { name: "Notes" };
        const screens = { name: "Screens" };
        const closes = [["d", "close"]];
        for (const [what, last, row, keys, anew] of [
            ["none before and none now tell nothing", { row: notes, keys: [] }, screens, [], false],
            ["the same row's same keys tell nothing", { row: notes, keys: closes }, notes,
                [["d", "close"]], false],
            ["another row's keys, though alike, are told", { row: notes, keys: closes }, screens,
                closes, true],
            ["keys where none were are told", { row: screens, keys: [] }, notes, closes, true],
            ["and none where keys were", { row: notes, keys: closes }, screens, [], true],
            ["and a key's other word", { row: notes, keys: closes }, notes,
                [["d", "quit"]], true]
        ])
            machine(what, Standing.toldAnew(last, row, keys), anew);

        // The network's figures, from /proc/net/dev and the shaper's file.
        const netdev = (wifi, warp) => "Inter-|   Receive |  Transmit\n"
            + " face |bytes    packets errs drop fifo frame compressed multicast|bytes ...\n"
            + "    lo: 500 5 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n"
            + `wlp0s20f3: ${wifi[0]} 10 0 1 0 0 0 0 ${wifi[1]} 10 0 2 0 0 0 0\n`
            + `tailscale0: ${warp[0]} 4 0 0 0 0 0 0 ${warp[1]} 4 0 0 0 0 0 0\n`
            + "shaper2: 9000 9 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n";
        const counted = Traffic.counters(netdev([1000, 200], [400, 100]));
        machine("an interface's counters", counted.wlp0s20f3, { in: 1000, out: 200, dropped: 3 });
        machine("every interface listed", Object.keys(counted),
            ["lo", "wlp0s20f3", "tailscale0", "shaper2"]);
        machine("what isn't /proc/net/dev", Traffic.counters("nothing here\n"), {});
        const cardKinds = { lo: "", wlp0s20f3: "wifi", tailscale0: "tunnel", shaper2: "" };
        const named = all => {
            const out = {};
            for (const name of Object.keys(all))
                out[name] = { kind: cardKinds[name], counters: all[name] };
            return out;
        };
        const recounted = Traffic.counters(netdev([3000, 600], [1400, 300]));
        const moved = Traffic.rates(named(recounted), named(counted), 2);
        machine("the cards' rates, without the tunnel over them",
            [moved.in, moved.out, moved.dropped], [1000, 200, 0]);
        machine("a tunnel's rates, listed apart",
            moved.each.find(one => one.name === "tailscale0").in, 500);
        machine("the loopback and mirrors left out", moved.each.map(one => one.name),
            ["wlp0s20f3", "tailscale0"]);
        const reset = Traffic.counters(netdev([100, 50], [0, 0]));
        machine("a counter that went back starts from nothing",
            Traffic.rates(named(reset), named(counted), 1).in, 100);
        machine("since the start, with no read before",
            Traffic.rates(named(counted), null, 10).in, 100);
        const number = (value, places) => value.toFixed(places);
        machine("a rate in megabits", Traffic.bits(1500000, number), "12 Mb/s");
        machine("a slow rate in kilobits", Traffic.bits(500, number), "4.0 kb/s");
        machine("nothing moving", Traffic.bits(0, number), "0 b/s");
        const status = [{
            name: "wlp0s20f3", ifb: "shaper2", up: 11830, down: 40000, state: "climbing",
            baselines: [["9.9.9.9", 51.1], ["1.1.1.1", 21.1], ["8.8.8.8", 20.6]], follows: null
        }];
        const link = Traffic.line(status, "wlp0s20f3");
        machine("what the line carries, in bytes a second", [link.in, link.out], [5000000, 1478750]);
        machine("the quickest server first", link.trips.map(trip => trip.server),
            ["8.8.8.8", "1.1.1.1", "9.9.9.9"]);
        machine("a line not shaped", Traffic.line(status, "enp1s0"), null);
        machine("no shaper at all", Traffic.line({}, "wlp0s20f3"), null);
        machine("the shaper's mirrors", Traffic.mirrors(status), ["shaper2"]);
        machine("the middle round trip", Traffic.typical(link.trips), 21.1);
        machine("the middle of an even count", Traffic.typical([{ ms: 10 }, { ms: 20 }]), 15);
        machine("no round trips", Traffic.typical([]), null);

        // The machine's connections, from ip, the routing tables and
        // resolv.conf, each by what it is.
        const networkListed = [{
            ifname: "lo", flags: ["LOOPBACK", "UP", "LOWER_UP"], mtu: 65536,
            link_type: "loopback", address: "00:00:00:00:00:00",
            addr_info: [{ family: "inet", local: "127.0.0.1", prefixlen: 8, scope: "host" }]
        }, {
            ifname: "wlp0s20f3", flags: ["BROADCAST", "MULTICAST", "UP", "LOWER_UP"], mtu: 1500,
            link_type: "ether", address: "02:00:00:00:01:00", parentbus: "pci",
            addr_info: [
                { family: "inet", local: "192.168.4.48", prefixlen: 22, scope: "global" },
                { family: "inet6", local: "fe80::ff:fe00:100", prefixlen: 64,
                    scope: "link" }
            ]
        }, {
            ifname: "enp1s0", flags: ["NO-CARRIER", "BROADCAST", "UP"], mtu: 1500,
            link_type: "ether", address: "3c:22:fb:12:34:56", permaddr: "3c:22:fb:00:00:01",
            parentbus: "pci", addr_info: []
        }, {
            ifname: "virbr0", flags: ["NO-CARRIER", "BROADCAST", "MULTICAST", "UP"], mtu: 1500,
            link_type: "ether", address: "52:54:00:62:19:d2", linkinfo: { info_kind: "bridge" },
            addr_info: [{ family: "inet", local: "192.168.122.1", prefixlen: 24, scope: "global" }]
        }, {
            ifname: "tailscale0", flags: ["POINTOPOINT", "UP", "LOWER_UP"], mtu: 1280,
            link_type: "none", linkinfo: { info_kind: "tun" },
            addr_info: [
                { family: "inet", local: "100.101.102.103", prefixlen: 32, scope: "global" },
                { family: "inet6", local: "fd7a:115c:a1e0::1", prefixlen: 128, scope: "global" }
            ]
        }, {
            ifname: "ifb4wlp0s20f3", flags: ["UP"], mtu: 1500, link_type: "ether",
            linkinfo: { info_kind: "ifb" }, addr_info: []
        }, {
            ifname: "veth0", flags: ["UP"], mtu: 1500, link_type: "ether",
            linkinfo: { info_kind: "veth" }, addr_info: []
        }];
        const networkLinks = Interfaces.interfaces(networkListed, ["wlp0s20f3"]);
        const networkKinds = {};
        for (const each of networkLinks)
            networkKinds[each.name] = each.kind;
        machine("each connection by what it is", networkKinds, {
            wlp0s20f3: "wifi", enp1s0: "wired", virbr0: "bridge", tailscale0: "tunnel",
            veth0: "virtual"
        });
        // The network's speed names each the same from sysfs, as it reads it
        // without spawning ip, for these and the tunnels it couldn't tell
        // apart before: a WireGuard one, a VXLAN one, and a GRE one, which
        // sysfs names only by its type.
        const sturdySys = {
            lo: { uevent: "INTERFACE=lo\nIFINDEX=1\n", type: "772" },
            wlp0s20f3: { uevent: "DEVTYPE=wlan\nINTERFACE=wlp0s20f3\n", type: "1", card: true },
            enp1s0: { uevent: "INTERFACE=enp1s0\n", type: "1", card: true },
            virbr0: { uevent: "DEVTYPE=bridge\nINTERFACE=virbr0\n", type: "1" },
            tailscale0: { uevent: "INTERFACE=tailscale0\n", type: "65534", tun: true },
            ifb4wlp0s20f3: { uevent: "INTERFACE=ifb4wlp0s20f3\n", type: "1" },
            veth0: { uevent: "INTERFACE=veth0\n", type: "1" },
            wg0: { uevent: "DEVTYPE=wireguard\nINTERFACE=wg0\n", type: "65534" },
            vxlan0: { uevent: "DEVTYPE=vxlan\nINTERFACE=vxlan0\n", type: "1" },
            gre1: { uevent: "INTERFACE=gre1\n", type: "778" }
        };
        const sturdyIp = networkListed.concat([
            { ifname: "wg0", link_type: "none", linkinfo: { info_kind: "wireguard" } },
            { ifname: "vxlan0", link_type: "ether", linkinfo: { info_kind: "vxlan" } },
            { ifname: "gre1", link_type: "gre", linkinfo: { info_kind: "gre" } }
        ]);
        const sturdyKinds = (named) => {
            const out = {};
            for (const link of sturdyIp)
                out[link.ifname] = named(link);
            return out;
        };
        const sturdyAddresses = sturdyKinds(link => Interfaces.kind(link,
            link.ifname === "wlp0s20f3"));
        machine("the tunnels by what they are", [sturdyAddresses.wg0, sturdyAddresses.vxlan0,
            sturdyAddresses.gre1], ["tunnel", "tunnel", "tunnel"]);
        machine("each connection named alike from sysfs",
            sturdyKinds(link => Interfaces.sysKind(link.ifname, sturdySys[link.ifname])),
            sturdyAddresses);
        const networkLink = name => networkLinks.find(each => each.name === name);
        machine("a card's address, past its own link only", networkLink("wlp0s20f3").four,
            [{ address: "192.168.4.48", prefix: 22 }]);
        machine("no link-local IPv6 address", networkLink("wlp0s20f3").six, []);
        machine("a tunnel's IPv6 address", networkLink("tailscale0").six[0].address,
            "fd7a:115c:a1e0::1");
        machine("a tunnel has no hardware address", networkLink("tailscale0").mac, "");
        machine("a card on its own address", networkLink("wlp0s20f3").own, "");
        machine("a card on another address than its own", networkLink("enp1s0").own,
            "3c:22:fb:00:00:01");
        machine("how each stands", ["wlp0s20f3", "enp1s0"].map(name => networkLink(name).state),
            ["up", "no link"]);
        machine("a card turned off", Interfaces.state({ flags: ["BROADCAST"] }), "off");
        machine("what isn't ip's", Interfaces.interfaces({}, []), []);
        machine("where a hardware address came from", ["0", "1", "3", ""].map(Interfaces.assigned),
            ["own", "random", "set", ""]);
        const networkRoute = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t"
            + "\tMTU\tWindow\tIRTT\n"
            + "wlp0s20f3\t00000000\t0104A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0\n"
            + "enp1s0\t00000000\t0100000A\t0003\t0\t0\t100\t00000000\t0\t0\t0\n"
            + "wlp0s20f3\t0004A8C0\t00000000\t0001\t0\t0\t600\t00FCFFFF\t0\t0\t0\n";
        const networkGateways = Interfaces.gateways(networkRoute);
        machine("each default gateway", networkGateways, {
            wlp0s20f3: { address: "192.168.4.1", metric: 600 },
            enp1s0: { address: "10.0.0.1", metric: 100 }
        });
        machine("the way out, costing least", Interfaces.wayOut(networkGateways), "enp1s0");
        machine("no way out", Interfaces.wayOut({}), "");
        const networkZeros = "00000000000000000000000000000000";
        const networkSix = `${networkZeros} 00 ${networkZeros} 00 `
            + "fe800000000000000000000000000001 00000400 00000001 00000000 00450003 wlp0s20f3\n"
            + "fd7a115ca1e000000000000000000000 30 00000000000000000000000000000000 00 "
            + "00000000000000000000000000000000 00000100 00000001 00000000 00000001 tailscale0\n";
        machine("an IPv6 default gateway", Interfaces.gateways6(networkSix), {
            wlp0s20f3: { address: "fe80:0000:0000:0000:0000:0000:0000:0001", metric: 1024 }
        });
        const networkResolv = "# written by resolved\nnameserver 127.0.2.2\nnameserver 127.0.2.3\n"
            + "search .\noptions edns0\n";
        machine("the name servers", Interfaces.servers(networkResolv), ["127.0.2.2", "127.0.2.3"]);
        machine("resolved's stub",
            Interfaces.stubbed(Interfaces.servers("nameserver 127.0.0.53\n")), true);
        machine("name servers of their own", Interfaces.stubbed(["127.0.2.2"]), false);

        // A cable's link as its folder in /sys/class/net says it, each
        // file's text as read, newline and all.
        const cableSys = {
            speed: "1000\n", duplex: "full\n", carrier: "1\n",
            carrier_down_count: "3\n", carrier_up_count: "4\n",
            "statistics/rx_errors": "12\n", "statistics/rx_crc_errors": "11\n",
            "statistics/rx_frame_errors": "1\n", "statistics/rx_length_errors": "0\n",
            "statistics/rx_over_errors": "0\n", "statistics/rx_missed_errors": "0\n",
            "statistics/rx_fifo_errors": "0\n", "statistics/tx_errors": "2\n",
            "statistics/tx_aborted_errors": "0\n", "statistics/tx_carrier_errors": "2\n",
            "statistics/tx_fifo_errors": "0\n", "statistics/tx_heartbeat_errors": "0\n",
            "statistics/tx_window_errors": "0\n", "statistics/collisions": "0\n",
            "statistics/rx_dropped": "40\n", "statistics/tx_dropped": "0\n"
        };
        const cable = Interfaces.cable(name => cableSys[name] ?? "");
        machine("a cable's speed and duplex", [cable.speed, cable.duplex], [1000, "full"]);
        machine("how a cable stands, from its carrier", [cable.state,
            Interfaces.cable(name => name === "carrier" ? "0\n" : "").state,
            Interfaces.cable(() => "").state], ["up", "no link", "off"]);
        machine("how often a cable's link was lost", cable.lost, 3);
        machine("a cable's errors each way", [cable.errors.in, cable.errors.out], [12, 2]);
        machine("only the kinds of error it had, in order",
            cable.errors.kinds.map(kind => [kind.id, kind.count]),
            [["damaged", 11], ["misaligned", 1], ["no-link", 2]]);
        machine("what a cable set aside each way", [cable.errors.droppedIn,
            cable.errors.droppedOut], [40, 0]);
        // A driver that doesn't know the speed says -1, and one that doesn't
        // know the duplex says so; a link that's down can't be read at all.
        const unknownCable = Interfaces.cable(name => ({ speed: "-1\n",
            duplex: "unknown\n" })[name] ?? "");
        machine("a speed and duplex nobody knows", [unknownCable.speed, unknownCable.duplex,
            unknownCable.lost, unknownCable.errors.in], [null, null, null, null]);
        machine("a half-duplex cable", Interfaces.cable(name => name === "duplex" ? "half" : "")
            .duplex, "half");
        const linkRates = [10, 100, 1000, 2500, 10000].map(mbps => Interfaces.linkRate(mbps,
            (value, places) => value.toFixed(places)));
        machine("a link's rate in its own units", linkRates,
            ["10 Mb/s", "100 Mb/s", "1 Gb/s", "2.5 Gb/s", "10 Gb/s"]);

        // A DHCP lease as networkd's Describe says it over varlink: its
        // times in microseconds since boot, the server's offer as options,
        // each its bytes in hex.
        const described = {
            Interfaces: [{
                Name: "cable0",
                DHCPv4Client: {
                    Lease: {
                        LeaseTimestampUSec: 100000000, Timeout1USec: 43300000000,
                        Timeout2USec: 75700000000, Hostname: "desk",
                        Message: {
                            options: [{ tag: 1, data: "ffffff00" }, { tag: 3, data: "0a090002" },
                                { tag: 6, data: "0a0900030a090004" },
                                { tag: 15, data: "686f6d652e6172706100" },
                                { tag: 42, data: "0a090005" }, { tag: 51, data: "00015180" },
                                { tag: 54, data: "0a090002" }]
                        }
                    }
                }
            }, { Name: "wlan0" }]
        };
        const leases = Networkd.leases(described);
        machine("a lease, by its connection", leases.cable0, {
            server: "10.9.0.2", lasts: 86400, got: 100, renews: 43300, rebinds: 75700,
            ends: 86500, nameServers: ["10.9.0.3", "10.9.0.4"], timeServers: ["10.9.0.5"],
            domain: "home.arpa", hostname: "desk"
        });
        machine("no lease where networkd holds none", "wlan0" in leases, false);
        // An older networkd says the times alone.
        const bare = Networkd.leases({ Interfaces: [{ Name: "eth0", DHCPv4Client: {
            Lease: { LeaseTimestampUSec: 5000000, Timeout1USec: 6000000 } } }] }).eth0;
        machine("a lease whose offer isn't said", [bare.server, bare.lasts, bare.got,
            bare.renews, bare.rebinds, bare.ends, bare.nameServers], [null, null, 5, 6, null,
            null, []]);
        machine("nothing from a reply that isn't one", Networkd.leases(null), {});

        // What's at a cable's other end, as networkd's GetLLDPNeighbors says.
        const neighbours = Networkd.neighbours({
            Neighbors: [{
                InterfaceIndex: 3, InterfaceName: "cable0",
                Neighbors: [{
                    ChassisID: "52:54:00:aa:bb:cc", RawChassisID: [], PortID: "gi1/0/3",
                    RawPortID: [], PortDescription: "desk drop 3", SystemName: "closet",
                    SystemDescription: "a switch", EnabledCapabilities: 20, VlanID: 10
                }]
            }]
        });
        machine("a cable's neighbour", neighbours.cable0, [{
            system: "closet", description: "a switch", port: "gi1/0/3",
            portName: "desk drop 3", chassis: "52:54:00:aa:bb:cc", acts: ["bridge", "router"],
            vlan: 10, profile: null
        }]);
        machine("no neighbours from a reply that isn't one", Networkd.neighbours(undefined), {});
        machine("a neighbour's parts, joined and written, what it doesn't say left out",
            Networkd.neighbourParts(neighbours.cable0[0]).map(said => `${said.id}=${said.side}`), [
                "system=closet", "description=a switch", "port=gi1/0/3",
                "port-name=desk drop 3", "chassis=52:54:00:aa:bb:cc", "acts=bridge, router",
                "vlan=10"
            ]);
        machine("no acts said by a neighbour doing nothing",
            Networkd.neighbourParts({ system: "closet", acts: [] }).map(said => said.id),
            ["system"]);

        // How long a time lasted, and a signal's height, as Machine and Home
        // Network say them.
        machine("how long, in hours and minutes", Interfaces.lasted(11744), "3 h 15 min");
        machine("how long, in days", Interfaces.lasted(200000), "2 d 7 h");
        machine("how long, in whole hours", Interfaces.lasted(72000), "20 h");
        machine("how long, in whole days", Interfaces.lasted(172800), "2 d");
        machine("how long, only just", Interfaces.lasted(20), "under a minute");
        machine("a signal's height", [-90, -60, -30, -10].map(Interfaces.level), [0, 0.5, 1, 1]);
        machine("a whole reading takes the real minus and no decimals",
            [Numbers.wholeReading(-14), Numbers.wholeReading(-60.4), Numbers.wholeReading(3)],
            ["\u221214", "\u221260", "3"]);

        // How each Bluetooth link is doing, from what bluetooth-links writes.
        const btMap = `${"1".repeat(22)}${"0".repeat(18)}${"1".repeat(39)}`;
        const btClassic = { address: "00:1B:66:AA:BB:CC", rssi: -6, quality: 230, power: 4,
            channels: 61, map: btMap, le: false };
        const btFile = JSON.stringify({ at: 1, links: [btClassic] });
        machine("the links a reader writes", Links.links(btFile).length, 1);
        machine("no links connected", Links.links(JSON.stringify({ links: [] })), []);
        machine("no reader", Links.links(""), null);
        machine("a file that isn't the reader's", Links.links("{}"), null);
        machine("a link by its address", Links.of(Links.links(btFile), btClassic.address).rssi, -6);
        machine("a device with no link", Links.of(Links.links(btFile), "C3:9C:E3:2A:F3:6E"), null);
        machine("a classic link below its best range", Links.signal(btClassic),
            { side: "-6 dB", value: 0.85,
                note: "below the range its radio hears best, sending at 4 dBm" });
        machine("a classic link in its best range", Links.signal({ rssi: 0, le: false }),
            { side: "in range", value: 1, note: "where its radio hears best" });
        machine("a classic link above its best range", Links.signal({ rssi: 3, le: false }).side,
            "+3 dB");
        machine("a classic link far below", Links.signal({ rssi: -60, le: false }).value, 0);
        machine("a Low Energy link's signal", Links.signal({ rssi: -58, power: null, le: true }),
            { side: "-58 dBm", value: 0.6, note: "" });
        machine("a signal not said", Links.signal({ rssi: null, le: true }), null);
        machine("no link, no signal", Links.signal(null), null);
        machine("the link's quality", Links.quality(btClassic).side, "90%");
        machine("a perfect link", Links.quality({ quality: 255 }).value, 1);
        machine("a quality not said", Links.quality({ quality: null }), null);
        const btChannels = Links.channels(btClassic);
        machine("the channels clear", [btChannels.side, btChannels.clear, btChannels.note],
            ["61 of 79 clear", 61, "the link skips the other 18 as noisy"]);
        machine("the channels as a strip", btChannels.values, [1, 1, 1, 1, 1, 0.5, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
        machine("every channel clear", Links.channels({ map: "1".repeat(79) }).note,
            "the link skips none as noisy");
        machine("no channel map", Links.channels({ map: null }), null);
        machine("music's codec", Links.codec("aac"), { name: "AAC", note: "" });
        machine("aptX in its own case", Links.codec("aptx_hd").name, "aptX HD");
        machine("a call's codec", Links.codec("msbc"),
            { name: "mSBC", note: "call quality, while its microphone is in use" });
        machine("any Opus", Links.codec("opus_05_51").name, "Opus");
        machine("a codec not known", Links.codec("new_one").name, "NEW_ONE");
        machine("no codec", Links.codec(undefined), null);
        machine("what music's codec carries", Links.carried({ "api.bluez5.profile": "a2dp-sink",
            "node.rate": "1/44100" }, 2), { profile: "music", rate: "44.1 kHz",
            channels: "stereo" });
        machine("what a call's codec carries", Links.carried({
            "api.bluez5.profile": "headset-head-unit", "audio.rate": 16000,
            "audio.channels": 1 }, 0), { profile: "calls", rate: "16 kHz", channels: "mono" });
        machine("six channels", Links.carried({ "audio.channels": 6 }, 0).channels, "6 channels");
        machine("a node saying nothing", Links.carried(null, 0),
            { profile: "", rate: "", channels: "" });
        machine("an old reader's link has its one read", Links.signals(btClassic), [0.85]);
        machine("a link held unencrypted", Links.held({ le: true, held: { central: false,
            outgoing: true, security: "authenticated" } }), {
            side: "Low Energy, peripheral, not encrypted",
            note: "this machine started it, though the device proved who it is" });
        machine("an old reader's link says nothing of how", Links.held(btClassic), null);
        machine("the version of Bluetooth", Links.version(14), "Bluetooth 6.0");
        machine("a version not known", Links.version(99), "");
        machine("a chip's maker", [Links.maker(2), Links.maker(9999)], ["Intel", ""]);
        machine("an adapter by its name", Links.adapter(Links.adapters(JSON.stringify({
            adapters: [{ name: "hci0" }], links: [] })), "hci0").name, "hci0");
        machine("an old reader's file has no adapters", Links.adapters(btFile), []);
        machine("an adapter read once has no traffic", Links.traffic({ received: null,
            sent: null, traffic: [] }), null);
        machine("a busy adapter's traffic", Links.traffic({ received: 2.5e6, sent: 0,
            traffic: [2.5e6] }), { side: "20 Mb/s", values: [1],
            note: "20 Mb/s in, 0 b/s out" });

        // What Sound makes of what a device and a stream say of themselves.
        const soundTuning = Sounds.target("{\n  node.name = \"alsa_output.speaker\"\n}");
        machine("what a filter aims at", soundTuning, { "node.name": "alsa_output.speaker" });
        machine("a filter aiming at two things", Sounds.target(
            "{ alsa.components = \"USB14ed:1012\" media.class = \"Audio/Source\" }"),
            { "alsa.components": "USB14ed:1012", "media.class": "Audio/Source" });
        machine("a filter's aim written as JSON", Sounds.target('{"node.name":"mic"}'),
            { "node.name": "mic" });
        machine("the device a filter aims at",
            Sounds.aims(soundTuning, { "node.name": "alsa_output.speaker", "node.nick": "Speaker" }),
            true);
        machine("a device it doesn't", Sounds.aims(soundTuning, { "node.name": "headphones" }),
            false);
        machine("a filter aiming at nothing", Sounds.aims(Sounds.target(""), {}), false);
        machine("the laptop's chips", Sounds.components("HDA:80862822,80860101,00100000  "
            + "cfg-amp:4 iec61937-pcm:7,6,5 hs:cs42l45 mic:cs42l45-dmic spk:cs35l56"),
            { amplifiers: 4, speaker: "CS35L56", headset: "CS42L45", microphone: "CS42L45",
                digital: true, usb: "" });
        machine("a USB card's ids", Sounds.components("USB041e:329a").usb, "041e:329a");
        machine("a card that names nothing", Sounds.components(undefined).speaker, "");
        machine("a stream's rate", Sounds.rate("1/44100"), "44.1 kHz");
        machine("a rate in whole kilohertz", Sounds.kilohertz(48000), "48 kHz");
        machine("no rate said", Sounds.rate(undefined), "");
        const soundAsked = Sounds.fraction("8192/44100");
        machine("what a stream asks for at once", [soundAsked.frames, soundAsked.hertz,
            soundAsked.ms.toFixed(1)], [8192, 44100, "185.8"]);
        machine("nothing asked", Sounds.fraction(""), null);
        machine("a channel's name", Sounds.channel("FrontLeft"), "Front left");
        machine("a lone channel", Sounds.channel("Mono"), "Mono");
        machine("each program's streams together", Sounds.grouped(["a1", "b1", "a2"],
            stream => stream.charAt(0)), [{ name: "a", streams: ["a1", "a2"] },
            { name: "b", streams: ["b1"] }]);
        machine("a step louder", Sounds.stepped(0.42, 0.05, 0.05), 0.45);
        machine("never past the whole", Sounds.stepped(0.98, 0.05, 0.05), 1);
        machine("never below nothing", Sounds.stepped(0.02, -0.05, 0.05), 0);
        machine("a level in decibels", [Sounds.decibels(1), Sounds.decibels(0.5),
            Sounds.decibels(0.35), Sounds.decibels(1.2), Sounds.decibels(0)],
            ["0 dB", "-18.1 dB", "-27.4 dB", "+4.8 dB", "-\u221e dB"]);
        // The machine's own are called what they are, by their port, and a
        // headset's microphone as its; any other keeps its own name.
        const soundOut = "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__";
        const soundIn = "alsa_input.pci-0000_00_1f.3-platform-sof_sdw.HiFi__";
        machine("what a device is called", [`${soundOut}Speaker__sink`, `${soundIn}Mic__source`,
            `${soundOut}Headphones__sink`, `${soundIn}Headset__source`,
            "bluez_input.00:11:22:33:44:55", "bluez_output.00_11_22_33_44_55.1",
            `${soundOut}HDMI1__sink`, undefined].map(name => Sounds.called(name)),
            ["speakers", "microphones", "jack", "jack-microphone", "its-microphone", "", "", ""]);
        // A USB microphone named as its own headphone socket is, its
        // device's microphone; one named alone keeps its name.
        const usb = "alsa_input.usb-Shure_Inc_Shure_MV7-00.mono-fallback";
        machine("a microphone named as an output", [Sounds.called(usb, true),
            Sounds.called(usb, false)], ["its-microphone", ""]);

        // What Sound says of Bluetooth headphones, as the sound server has
        // them while they play music in AAC: their output, and WirePlumber's
        // stand-in for their microphone, which names them only by address.
        const soundHeadphones = { "api.bluez5.address": "12:34:56:78:9A:BC",
            "api.bluez5.codec": "aac", "api.bluez5.profile": "a2dp-sink",
            "node.description": "Muse", "device.id": 65, "card.profile.device": 1 };
        const soundStandIn = { "bluez5.loopback": "true", "node.description": "Muse" };
        machine("a Bluetooth output's address", Sounds.address("bluez_output.12_34_56_78_9A_BC.1",
            soundHeadphones), "12:34:56:78:9A:BC");
        machine("its microphone's stand-in's, from its name",
            Sounds.address("bluez_input.12:34:56:78:9A:BC", soundStandIn), "12:34:56:78:9A:BC");
        machine("a card's own output has none", Sounds.address("alsa_output.pci", {}), "");
        machine("the stand-in is one", [Sounds.standIn(soundStandIn),
            Sounds.standIn(soundHeadphones)], [true, false]);
        machine("music, not a2dp", Sounds.carries(soundHeadphones["api.bluez5.profile"]),
            "music");
        machine("calls, not hfp", [Sounds.carries("headset-head-unit"),
            Sounds.carries("hfp-hf")], ["calls", "calls"]);
        machine("no profile, nothing carried", Sounds.carries(undefined), "");

        // The format the graph moves a node's audio in, as the patched
        // PwNodeAudio says it: its rate to three significant digits and its
        // sample's bits, a float said so; none while it's suspended. It's
        // the card's own said again where the card's running stream opens
        // with the same rate and bits, a float graph over an integer card
        // not.
        machine("the graph's format, said", [
            Sounds.graphFormat({ rate: 48000, channels: 2, sample: "F32LE" }),
            Sounds.graphFormat({ rate: 44100, channels: 2, sample: "S16LE" }),
            Sounds.graphFormat({ rate: 176400, channels: 2, sample: "S24_32LE" }),
            Sounds.graphFormat({ rate: 48000, channels: 2, sample: "ALAW" }),
            Sounds.graphFormat({})], [
            "48 kHz \u00b7 32 bit float", "44.1 kHz \u00b7 16 bit",
            "176 kHz \u00b7 24 bit", "48 kHz \u00b7 ALAW", ""]);
        // How a node keeps time, from a line of pw-top's batch mode: only a
        // running one has a cycle, its busy time a share of it, a time
        // pw-top can't say is none, a follower runs in its driver's cycle,
        // and a header opens the next frame.
        const running = Sounds.timing(
            "R   79   1024  48000  20.1us 665.6us  0.00  0.03    2    S32LE 2 48000 "
            + "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Speaker__sink", null);
        const four = share => share === null ? null : Number(share.toFixed(4));
        // Its cycle and load are measures, said by the one rule.
        const threeDigits = value => Numbers.significant(value);
        machine("a running node's timing", [running?.id, running?.quantum, running?.rate,
            running ? threeDigits(Sounds.cycleMs(running)) : null, four(running?.busy ?? null),
            running?.errors, running ? threeDigits(Sounds.load(running.busy)) : null,
            running?.drives],
        [79, 1024, 48000, "21.3", 0.0312, 2, "3.12", true]);
        const busyIn = (state, busy) => Sounds.timing(
            `${state} 80 1024 48000 20.1us ${busy} 0.00 0.07 0 F32LE 2 48000 a`, null);
        machine("busy in each unit pw-top says, unfinished past its cycle, a transport running", [
            four(busyIn("R", "1.5ms")?.busy ?? null),
            four(busyIn("R", "0.1s")?.busy ?? null),
            busyIn("R", "+++")?.busy, busyIn("R", "+++")?.unfinished,
            busyIn("R", "---")?.unfinished, busyIn("R", "1.5ms")?.unfinished,
            four(busyIn("T", "1.5ms")?.busy ?? null),
            four(busyIn("t", "1.5ms")?.busy ?? null)],
        [0.0703, 4.6875, null, true, false, false, 0.0703, 0.0703]);
        const follower = "R   81   0   0   1.0us   2.1ms  0.00  0.10   0  F32LE 2 48000 + pw-cat";
        const followed = Sounds.timing(follower, running);
        machine("a follower in its driver's cycle, and none without one", [followed?.quantum,
            followed ? threeDigits(Sounds.cycleMs(followed)) : null,
            followed ? threeDigits(Sounds.load(followed.busy)) : null,
            followed?.drives, Sounds.timing(follower, null)],
        [1024, "21.3", "9.84", false, null]);
        // An async follower is marked "=" rather than "+" (pw-top.c).
        const async = "R   82  256  48000   1.0us   2.1ms  0.00  0.10   0  F32LE 2 48000 = a";
        machine("an async follower in its driver's cycle, and what's a follower", [
            Sounds.timing(async, running)?.quantum, Sounds.timing(async, running)?.drives,
            Sounds.follower(async), Sounds.follower(follower),
            Sounds.follower("S   47      0      0    ---     ---   ---   ---     0   speaker")],
        [1024, false, true, true, false]);
        machine("one not running, one pw-top can't measure, and the header", [
            Sounds.timing("S   47      0      0    ---     ---   ---   ---     0   speaker", null),
            Sounds.timing("I   48     0      0   0.0us   0.0us  ???   ???   0  F32P 2 0 out", null),
            Sounds.timing("R   50   256  44100   1.0us   ---  ???   ???   0  S16LE 2 44100 a", null)
                ?.busy,
            Sounds.heading("S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT  NAME "),
            Sounds.heading("S   47      0      0    ---     ---   ---   ---     0       speaker")],
        [null, null, null, true, false]);
        machine("the card says it already, or not", [
            Sounds.sameFormat("48 kHz \u00b7 32 bit",
                "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)"),
            Sounds.sameFormat("48 kHz \u00b7 32 bit float",
                "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "48 kHz \u00b7 16 bit"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "44.1 kHz \u00b7 16 bit \u00b7 period 1024"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "")], [true, false, true, false, false]);

        // Whether a provider's rows can be seen, so it watches its daemon
        // only then: never once only asked, and never with the launcher
        // closed unless a terminal just showed them.
        const watcher = {};
        const its = { live: watcher };
        const closed = { open: false, answer: its, held: [], matches: [], placed: ["heat"] };
        const opened = fields => Object.assign({ open: true, answer: null, held: [], matches: [],
            placed: [] }, fields);
        machine("unseen with the launcher closed, whatever it last held",
            Seen.seen(closed, watcher, ["heat"]), false);
        machine("unseen with the launcher open on something else",
            Seen.seen(opened({ answer: { live: {} }, placed: ["power"] }), watcher, ["heat"]),
            false);
        machine("seen as its answer", Seen.seen(opened({ answer: its }), watcher, []), true);
        machine("seen stepped into", Seen.seen(opened({ held: [{ row: its }] }), watcher, []),
            true);
        machine("seen among what's found", Seen.seen(opened({ matches: [its] }), watcher, []),
            true);
        // A collection's own row, and a member of it found from outside,
        // are the collection's (Launcher.qml's into and gathered rows).
        machine("seen stepped into as a collection",
            Seen.seen(opened({ held: [{ row: { from: watcher } }] }), watcher, []), true);
        machine("seen as a collection's member found from outside",
            Seen.seen(opened({ matches: [{ from: watcher }] }), watcher, []), true);
        machine("unseen inside another collection",
            Seen.seen(opened({ held: [{ row: { from: {} } }] }), watcher, []), false);
        machine("seen in a place holding its row",
            Seen.seen(opened({ placed: ["heat"] }), watcher, ["heat"]), true);
        machine("seen as the place it says how things stand in",
            Seen.seen(opened({ matches: [{ category: { id: "energy" } }] }), watcher,
                ["energy", "power"]), true);
        machine("seen by a terminal's show with the launcher closed",
            Seen.seen(Object.assign({}, closed, { terminalSees: { walked: [], listed: [its] } }),
                watcher, []), true);
        machine("no launcher sees nothing", Seen.seen(null, watcher, ["heat"]), false);
        // Its answer is made from what the daemon says, so words that ask
        // it watch it first, and a row a condition holds back until the
        // daemon says is still watched where its place holds it.
        const asking = typed => typed === "heat";
        machine("seen as what's typed asks it",
            Seen.seen(opened({ typed: "heat" }), watcher, [], asking), true);
        machine("unseen as what's typed asks another",
            Seen.seen(opened({ typed: "battery" }), watcher, [], asking), false);
        machine("unseen as words ask it with the launcher closed",
            Seen.seen(Object.assign({}, closed, { answer: null, typed: "heat" }), watcher, [],
                asking), false);
        machine("a question is asked whole", Seen.asks(["battery"], "battery"), true);
        machine("a question is asked in the plural", Seen.asks(["fan"], "fans"), true);
        machine("a question is asked by its start", Seen.asks(["battery"], "batt"), true);
        machine("two letters start no question", Seen.asks(["battery"], "ba"), false);
        machine("another word asks nothing", Seen.asks(["battery"], "heat"), false);
        machine("seen in a place holding it before its condition holds it back",
            Seen.seen(opened({ category: { holds: ["energy", "heat"] } }), watcher, ["heat"]),
            true);
        // What each camera says of itself, from sysfs and desk-facts.
        machine("a loopback's format at a fraction of frames",
            Cameras.loopback("NV12:1920x1080@300000/10000\n"), "NV12 1920 \u00d7 1080 at 30");
        machine("a loopback's format at whole frames", Cameras.loopback("GREY:648x368@30"),
            "GREY 648 \u00d7 368 at 30");
        machine("a loopback's format as usually written", Cameras.loopback("MJPG:640x480@15/1"),
            "MJPEG 640 \u00d7 480 at 15");
        machine("no loopback format", Cameras.loopback(""), "");
        machine("a sensor on the bus", Cameras.sensor("i2c-OVTI08F4:00"),
            { id: "OVTI08F4", lights: "OVTI08F4_00::" });
        machine("not a sensor", Cameras.sensor("i2c-17"), null);
        const cameraColour = "OVTI08F4_00::";
        const cameraInfrared = "HIMX1092_00::";
        machine("a sensor's privacy light",
            Cameras.light(`${cameraColour}privacy_led`, cameraColour), "privacy");
        machine("a sensor's IR light",
            Cameras.light(`${cameraInfrared}ir_flood_led`, cameraInfrared), "flood");
        machine("another sensor's light",
            Cameras.light(`${cameraInfrared}ir_flood_led`, cameraColour), "");
        machine("the keyboard's light", Cameras.light("dell::kbd_backlight", "OVTI08F4_00::"), "");
        machine("where the laptop's cameras sit", Cameras.place({ panel: "front", vertical: "center",
            horizontal: "right", lid: "no" }), "front, centre right");
        machine("a camera dead centre, in the lid", Cameras.place({ panel: "front",
            vertical: "center", horizontal: "center", lid: "yes" }), "front, centre, in the lid");
        machine("a camera the firmware doesn't place", Cameras.place({ panel: "unknown",
            vertical: "", horizontal: "", lid: "" }), "");
        machine("powered a moment", Cameras.lasted(999), "0 s");
        machine("powered for minutes", Cameras.lasted(678291), "11 min");
        machine("powered for hours", Cameras.lasted(7500000), "2 h 5 min");
        machine("powered for whole hours", Cameras.lasted(7200000), "2 h");
        const cameraFacts = [{ name: "streaming", value: "MJPEG 1920 \u00d7 1080 at 30" },
            { name: "MJPEG", value: "3840 \u00d7 2160 at 30; 1920 \u00d7 1080 at 60" },
            { name: "YUYV", value: "640 \u00d7 480 at 30" },
            { name: "H.264", value: "1920 \u00d7 1080 at 30" },
            { name: "field of view", value: "90\u00b0" }, { name: "USB", value: "5 Gbit/s" },
            { name: "firmware", value: "release 3.17" }];
        const cameraSplit = Cameras.formats(cameraFacts);
        machine("a camera's formats", cameraSplit.modes.map(fact => fact.name),
            ["MJPEG", "YUYV", "H.264"]);
        machine("a camera's other facts", cameraSplit.rest.map(fact => fact.name),
            ["streaming", "field of view", "USB", "firmware"]);
        machine("a camera with no facts", Cameras.formats(undefined), { modes: [], rest: [] });

        // A camera's drawer says its parts, what moves first and what
        // identifies it last, its holders left to its row's side, how long
        // its sensor's been on to the second and its modes parted by dots.
        const cameraWords = { builtIn: "built in \u00b7 camera", plugged: "on USB \u00b7 camera",
            powered: "{state} \u00b7 {lasted} since start" };
        const cameraParts = [
            { key: "camera:camera:holders", name: "In Use", side: "Zoom" },
            { key: "camera:camera:privacy", name: "Privacy Light", side: "on" },
            { key: "camera:camera:sensor", name: "Sensor", side: "OmniVision OV08X40" },
            { key: "camera:camera:where", name: "Where", side: "front, centre" },
            { key: "camera:camera:powered", name: "Powered", side: "on", active: 192400 },
            { key: "camera:camera:modes", name: "Formats", side: "MJPEG, YUYV",
                listed: ["MJPEG", "YUYV"] },
            { key: "fact:cameras:camera:usb", name: "Connection", side: "5 Gbit/s" },
            { key: "fact:cameras:camera:field of view", name: "Field of View", side: "90\u00b0" },
            { key: "camera:camera:streaming", name: "Streaming",
                side: "NV12 1920 \u00d7 1080 at 30" },
            { key: "camera:camera:flood", name: "IR Light", side: "" }
        ];
        const cameraDrawn = Cameras.drawer(cameraParts,
            { name: "Built-In Camera", builtIn: true, said: cameraWords });
        machine("a camera's drawer, in its order", cameraDrawn.facts.map(fact => fact.label),
            ["Streaming", "Privacy Light", "Powered", "Formats", "Sensor", "Where",
                "Field of View", "Connection"]);
        machine("a camera's drawer, as it says each", [cameraDrawn.eyebrow,
            cameraDrawn.facts[2].value, cameraDrawn.facts[3].value],
            ["built in \u00b7 camera", "on \u00b7 3 min 12 s since start", "MJPEG \u00b7 YUYV"]);
        machine("a plugged-in camera's eyebrow", Cameras.drawer([],
            { name: "Logitech BRIO", builtIn: false, said: cameraWords }).eyebrow,
            "on USB \u00b7 camera");
        machine("powered for hours, to its two largest units", Cameras.drawer([
            { key: "camera:camera:powered", name: "Powered", side: "on", active: 7500000 }],
            { builtIn: true, said: cameraWords }).facts[0].value,
            "on \u00b7 2 h 5 min since start");

        // The keyboards connected, the laptop's and an ErgoDox's, as the
        // machine daemon lists them from the kernel's list of input devices
        // (modules/machine/src/keyboards.rs, whose tests say which it
        // leaves out).
        const keyboardsFound = [{
            name: "AT Translated Set 2 keyboard",
            phys: "isa0060/serio0/input0",
            sysfs: "/devices/platform/i8042/serio0/input/input0",
            uniq: "",
            handlers: "sysrq kbd event0 leds ",
            bus: "0011",
            vendor: "0001",
            product: "0001",
            leds: 7
        }, {
            name: "ZSA Technology Labs ErgoDox EZ",
            phys: "usb-0000:00:14.0-3.4/input0",
            sysfs: "/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4/3-3.4:1.0/"
                + "0003:3297:4974.0001/input/input5",
            uniq: "",
            handlers: "sysrq kbd leds event4 ",
            bus: "0003",
            vendor: "3297",
            product: "4974",
            leds: 31
        }];
        const [keyboardsLaptop, keyboardsErgodox] = keyboardsFound;
        machine("how each is joined", keyboardsFound.map(device => Keyboards.joined(device)),
            ["built in", "on USB"]);
        machine("on Bluetooth", Keyboards.joined({ phys: "f4:4e:fd:00:11:22" }), "on Bluetooth");
        machine("the built-in one", keyboardsFound.map(device => Keyboards.builtIn(device)),
            [true, false]);
        machine("ZSA's board", keyboardsFound.map(device => Keyboards.zsa(device)), [false, true]);
        machine("the laptop's light on its keyboard", Keyboards.lightHolder(keyboardsFound).name,
            "AT Translated Set 2 keyboard");
        // A laptop whose keyboard isn't on the keyboard port still has its
        // light, on the first keyboard neither paired nor ZSA's.
        const keyboardsInside = { name: "Inside keyboard", phys: "i2c-PNP0C50:00", bus: "0018" };
        machine("the laptop's light with no keyboard port",
            Keyboards.lightHolder([keyboardsErgodox, keyboardsInside]).name, "Inside keyboard");
        machine("no light with only a board", Keyboards.lightHolder([keyboardsErgodox]), null);
        // Super+K shows the layout of a keyboard connected now: the board's
        // while it's connected, the laptop's own with it alone, and the
        // laptop's too for a board whose layout isn't known.
        const boardLayers = [{ name: "Coal" }, { name: "Punk" }];
        const laptopLayers = [{ name: "Colemak" }];
        machine("the laptop's layout with it alone",
            Keyboards.layout([keyboardsLaptop], boardLayers, laptopLayers), laptopLayers);
        machine("the board's layout with both connected",
            Keyboards.layout(keyboardsFound, boardLayers, laptopLayers), boardLayers);
        machine("the laptop's layout for a board without one",
            Keyboards.layout(keyboardsFound, [], laptopLayers), laptopLayers);
        machine("the board's acts only while it's connected", [[keyboardsLaptop], keyboardsFound,
            [keyboardsErgodox], []].map(listed => Keyboards.boarded(listed, boardLayers)),
            [false, true, true, false]);
        machine("the laptop's locks", Keyboards.locks(keyboardsLaptop), [
            { id: "caps", light: "input0::capslock" },
            { id: "num", light: "input0::numlock" },
            { id: "scroll", light: "input0::scrolllock" }
        ]);
        machine("caps lock alone", Keyboards.locks(Object.assign({}, keyboardsLaptop, { leds: 2 }))
            .map(lock => lock.id), ["caps"]);
        machine("no USB folder for the laptop's", Keyboards.usbFolder(keyboardsLaptop), "");
        machine("a USB board's folder", Keyboards.usbFolder(keyboardsErgodox),
            "/sys/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4");
        machine("the laptop's connection", Keyboards.connection(keyboardsLaptop, null), "PS/2");
        machine("a USB board's connection", Keyboards.connection(keyboardsErgodox,
            { version: " 2.00", speed: "12" }), "USB 2.0 at 12 Mb/s");
        machine("a fast USB board's", Keyboards.connection(keyboardsErgodox,
            { version: " 3.20", speed: "5000" }), "USB 3.2 at 5 Gb/s");
        machine("a USB board unread", Keyboards.connection(keyboardsErgodox, null), "USB");
        machine("a Bluetooth board's", Keyboards.connection({ bus: "0005",
            uniq: "f4:4e:fd:00:11:22" }, null), "Bluetooth, F4:4E:FD:00:11:22");
        machine("lights at a level", Keyboards.lit(2, 2), "2 of 2");
        machine("lights off", Keyboards.lit(0, 2), "off");
        machine("a minute's timeout", Keyboards.timeout("1m\n"), "1 minute");
        machine("seconds' timeout", Keyboards.timeout("30s"), "30 seconds");
        machine("no timeout", Keyboards.timeout("0"), "never");
        machine("what lights them again", Keyboards.wakers("+keyboard +touchpad"),
            "lit again by a key or the touchpad");
        machine("nothing lights them again", Keyboards.wakers("-keyboard -touchpad"), "");
        machine("how a held key repeats", Keyboards.repeats({ delay: 200, rate: 40 },
            "after {delay} ms \u00b7 {rate} Hz"), "after 200 ms \u00b7 40 Hz");
        // A board's layers to move to: the one it's locked on let go, then
        // each above the first but that one locked on; held by a key, every
        // layer above the first.
        const keyboardsLayers = ["Coal", "Punk", "Numb"];
        machine("a locked board's layers", Keyboards.layerChoices(keyboardsLayers, "Numb", true), [
            { id: "keyboard:release", key: "keyboard:release", layer: "Numb", index: 0 },
            { id: "keyboard:lock", key: "keyboard:lock:Punk", layer: "Punk", index: 1 }
        ]);
        machine("a held board's layers", Keyboards.layerChoices(keyboardsLayers, "Numb", false)
            .map(choice => `${choice.layer} ${choice.index}`), ["Punk 1", "Numb 2"]);

        // What key-counts tallied of the board, as the machine daemon sums
        // it (modules/machine/src/counts.rs, whose tests sum it from the
        // file): its keys on two layers, some under modifiers.
        const counts = {
            key: "counts:ergodox", kind: "counts", board: "ergodox",
            presses: 20000, holds: 3240,
            most: [{ name: "e", presses: 9100 }, { name: "space", presses: 5000 },
                { name: "shift", presses: 3200 }, { name: "f23", presses: 1000 },
                { name: "a", presses: 900 }],
            layers: [{ name: "coal", presses: 18200 }, { name: "numb", presses: 1800 }],
            modified: 1900,
            modifiers: [{ name: "shift", presses: 1000 }, { name: "ctrl", presses: 900 }],
            began_s: 1793898549
        };
        // The theme's faces, by the key map's spelling, as Shortcuts.qml
        // gives them (modules/theme/keys.nix).
        const keyFaces = { shift: { text: "\u21e7" }, f23: { text: "Copilot", wide: false } };
        const tally = KeyCounts.board(counts, keyFaces);
        machine("a board's presses and holds", [tally.presses, tally.holds], [20000, 3240]);
        machine("the keys pressed most, as a chord draws them", tally.most,
            [{ key: "E", presses: 9100 }, { key: "Space", presses: 5000 },
                { key: "\u21e7", presses: 3200 }, { key: "Copilot", presses: 1000 },
                { key: "A", presses: 900 }]);
        const keyNamed = id => `[${id}]`;
        const keyWords = { since: "since {day}", began: "since counting began" };
        machine("a board's counts as facts", KeyCounts.facts(tally, keyNamed, "since 5 Nov"), [
            { label: "[presses]", value: "20,000 since 5 Nov" },
            { label: "[held]", value: "3,240" },
            { label: "[layers]", value: "coal 91% \u00b7 numb 9%" },
            { label: "[modified]", value: "9.5% \u00b7 shift 5% \u00b7 ctrl 4.5%" }
        ]);
        machine("the keys under their heading", KeyCounts.most(tally, keyNamed).slice(0, 2),
            [{ heading: "[most]" }, { keys: [["E"]], value: "9.1k" }]);
        machine("a board with nothing counted", [KeyCounts.board(null, keyFaces),
            KeyCounts.facts(null, keyNamed, ""), KeyCounts.most(null, keyNamed)], [null, [], []]);
        machine("counts short", [870, 9100, 12400].map(KeyCounts.short), ["870", "9.1k", "12k"]);
        machine("counting began", KeyCounts.since(counts.began_s, keyWords,
            new Date(2026, 10, 7)), "since 5 Nov");
        machine("counting began in another year", KeyCounts.since("1762362549", keyWords,
            new Date(2026, 10, 7)), "since 5 Nov 2025");
        machine("no start written", KeyCounts.since("", keyWords), "since counting began");

        // A keyboard's drawer: where it is, then what moves, what it
        // counted, how it's set, what describes it, its identifiers, and
        // its keys last; what it doesn't say is left out.
        const boardSaid = { usb: "on USB \u00b7 keyboard", builtIn: "built in \u00b7 keyboard",
            caps: "caps", num: "num", scroll: "scroll", none: "none", locked: "locked" };
        const boardDrawer = Keyboards.drawer({
            device: keyboardsErgodox,
            name: "ErgoDox EZ",
            layer: { name: "Numb", locked: true },
            locks: [{ id: "caps", on: false }, { id: "num", on: true }],
            counted: [{ label: "[presses]", value: "20,000 since 5 Nov" }],
            desk: [{ name: "firmware", value: "1.2" }, { name: "polling rate",
                value: "1000 a second" }, { name: "keys", value: "76" }],
            repeat: "after 200 ms, 40 Hz",
            lights: "",
            connection: "USB 2.0 at 12 Mb/s",
            maker: "ZSA Technology Labs",
            model: "ErgoDox EZ",
            usbId: "3297:4976",
            serial: "",
            most: [{ heading: "[most]" }]
        }, keyNamed, boardSaid);
        machine("a board's eyebrow", boardDrawer.eyebrow, "on USB \u00b7 keyboard");
        machine("a board's facts in order", boardDrawer.facts.map(fact => fact.label
            ?? fact.heading), ["[layer]", "[locks]", "[presses]", "[polling-rate]",
            "[repeat]", "[connection]", "[maker]", "[model]", "[keys]", "[firmware]",
            "[usb-id]", "[most]"]);
        machine("a layer locked", boardDrawer.facts[0].value, "Numb \u00b7 locked");
        machine("only the locks on", boardDrawer.facts[1].value, "num");
        machine("its polling rate in Hz", boardDrawer.facts[3].value, "1,000 Hz");
        machine("its USB id in the Name face", boardDrawer.facts[10].identifies, true);
        const bareBoard = Keyboards.drawer({ device: keyboardsErgodox, name: "", layer: null,
            locks: [{ id: "caps", on: false }], connection: "USB" }, keyNamed, boardSaid);
        machine("no lock on, and a bus the eyebrow says", bareBoard.facts.map(fact =>
            `${fact.label} ${fact.value}`), ["[locks] none"]);
        machine("the laptop's eyebrow", Keyboards.drawer({ device: keyboardsLaptop, name: "",
            layer: null }, keyNamed, boardSaid).eyebrow, "built in \u00b7 keyboard");

        // The power profile's drawer: its holds one fact parted by dots,
        // why it's slowed in words, the firmware's and the processor's
        // settings as written, and the drivers last.
        const profileWords = { eyebrow: "on this machine \u00b7 power profile",
            held: "{app} for {reason}", lap: "on a lap", heat: "too hot" };
        const profileDrawn = Power.profileDrawer({
            holds: [{ app: "Steam", reason: "a game" }, { app: "Firefox", reason: "a video" }],
            slowed: "heat", firmware: "performance", offers: "cool quiet balanced performance\n",
            processor: "balance_performance", processorDriver: "intel_pstate",
            firmwareDrivers: ["SoC Power Slider", "dell-pc"]
        }, id => `[${id.slice("session:profile:".length)}]`, profileWords);
        machine("the power profile's drawer", [profileDrawn.eyebrow].concat(profileDrawn.facts
            .map(fact => `${fact.label} ${fact.value}`)), ["on this machine \u00b7 power profile",
            "[held] Steam for a game \u00b7 Firefox for a video", "[slowed] too hot",
            "[firmware] performance",
            "[offers] cool \u00b7 quiet \u00b7 balanced \u00b7 performance",
            "[processor] balance_performance", "[processor-driver] intel_pstate",
            "[firmware-driver] SoC Power Slider \u00b7 dell-pc"]);
        machine("an unheld profile on a machine whose firmware says nothing",
            Power.profileDrawer({ holds: [{ app: "Steam", reason: "" }], slowed: "",
                processorDriver: "amd-pstate-epp" }, id => id, profileWords).facts
                .map(fact => fact.value), ["Steam", "amd-pstate-epp"]);
        // A profile picked is read back from the daemon, and one it didn't
        // take says why: nothing heard, a program holding another, or the
        // daemon keeping its own.
        const profileTold = holds => ({ holds,
            named: { "power-saver": "Power Saver", performance: "Performance" },
            said: { silent: "power profile not set \u00b7 not answering",
                held: "{app} holds {profile}", kept: "{profile} kept" } });
        machine("a profile taken says nothing",
            Power.profileRefusal("balanced", "balanced", profileTold([])), "");
        machine("a daemon that didn't answer is said so",
            Power.profileRefusal("balanced", "", profileTold([])),
            "power profile not set \u00b7 not answering");
        machine("a program holding the profile kept is named",
            Power.profileRefusal("balanced", "performance",
                profileTold([{ profile: "performance", app: "Steam" }])),
            "Steam holds Performance");
        machine("else the daemon kept its own",
            Power.profileRefusal("balanced", "power-saver",
                profileTold([{ profile: "performance", app: "Steam" }])),
            "Power Saver kept");
        // How bright a room is, from the light sensor's lux.
        machine("a dark room", Power.roomLevel(0), 0);
        machine("daylight", Power.roomLevel(10000), 1);
        machine("a room a hundredth of daylight", Math.round(Power.roomLevel(100) * 100) / 100,
            0.5);

        // WARP, from the record warp prints (modules/warp/warp.json), the
        // same one its own tests read warp-cli's answers into, in the
        // words the launcher's vocabulary gives it (launcher/words.json).
        const reading = file => {
            const request = new XMLHttpRequest();
            request.open("GET", Qt.resolvedUrl(file), false);
            request.send();
            return JSON.parse(request.responseText);
        };
        const warp = reading("warp.json");
        const vocabulary = reading("words.json").things;
        const warpSaid = vocabulary.find(thing => thing.id === "warp").said;
        const warpTold = record => Warp.told(record, {}, warpSaid);
        const figure = (id, side, note, copy) => ({ id, side, note, copy, warns: false });
        // An entry of a list, a host or an address, named by itself.
        const entry = (id, name, side) => ({ id, name, side, note: "", copy: name, warns: false });
        const linked = warpTold(warp);
        machine("WARP connected, as its row says it", linked && [linked.headline, linked.warns],
            ["connected \u00b7 127 ms", false]);
        machine("WARP's parts", linked && linked.parts, [
            figure("state", "connected", "", "connected"),
            figure("health", "healthy", "", "healthy"),
            Object.assign(figure("settings", "warp+doh", "always on \u00b7 split tunnel",
                "warp+doh"), { parts: [
                figure("settings:mode", "warp+doh",
                    "the tunnel with DNS over HTTPS \u00b7 set by local policy", "warp+doh"),
                figure("settings:always-on", "on", "the default", "on"),
                Object.assign(figure("settings:split-tunnel", "include",
                    "2 hosts \u00b7 2 networks \u00b7 set by network policy", "include"), { parts: [
                    entry("settings:split-tunnel:host", "intranet.example.com", "example"),
                    entry("settings:split-tunnel:host", "*.corp.example.com", "example"),
                    entry("settings:split-tunnel:network", "10.0.0.0/8", "lan"),
                    entry("settings:split-tunnel:network", "2001:db8:1::/48", "lan")
                ] }),
                figure("settings:switch-locked", "off", "set by an override", "off"),
                figure("settings:auto-connect", "2 min", "set by local policy", "2"),
                figure("settings:organisation", "example", "set by local policy", "example"),
                figure("settings:tunnel-protocol", "masque", "set by network policy", "masque"),
                figure("settings:post-quantum", "enabled with downgrades",
                    "set by network policy", "enabled_with_downgrades"),
                Object.assign(figure("settings:doh", "0000.doh.example.com", "4 addresses",
                    "0000.doh.example.com"), { parts: [
                    entry("settings:doh:address", "192.0.2.53", ""),
                    entry("settings:doh:address", "192.0.2.54", ""),
                    entry("settings:doh:address", "2001:db8::53", ""),
                    entry("settings:doh:address", "2001:db8::54", "")
                ] }),
                Object.assign(figure("settings:local-domains", "3 domains", "", "3"), { parts: [
                    entry("settings:local-domains:domain", "home.arpa", ""),
                    entry("settings:local-domains:domain", "local", ""),
                    entry("settings:local-domains:domain", "example.com", "192.0.2.1")
                ] }),
                figure("settings:sign-in-pages", "on", "opens its firewall for 5 min", "on"),
                figure("settings:path-mtu", "on", "", "on"),
                figure("settings:mode-switch", "on", "", "on"),
                figure("settings:may-leave", "off", "", "off"),
                figure("settings:local-network", "/24", "", "24"),
                figure("settings:override", "off", "", "off")
            ] }),
            figure("latency", "127 ms", "", "127"),
            figure("loss", "1.24%", "", "0.012355098"),
            figure("protocol", "MASQUE", "HTTPS via UDP", "MASQUE (HTTPS via UDP)"),
            figure("colo", "YYZ", "metal 0a1b2c", "YYZ"),
            figure("endpoint", "198.51.100.2", "no IPv6", "198.51.100.2"),
            figure("handshake", "2 h ago", "", "8751"),
            figure("traffic", "10.2 GB in \u00b7 9.26 GB out", "",
                "10.2 GB in \u00b7 9.26 GB out"),
            Object.assign(figure("tls", "TLSv1.3", "post-quantum", "TLSv1.3"), { parts: [
                figure("tls:version", "TLSv1.3", "", "TLSv1.3"),
                figure("tls:curve", "P256Kyber768Draft00", "", "P256Kyber768Draft00"),
                figure("tls:cipher", "TLS_AES_256_GCM_SHA384", "", "TLS_AES_256_GCM_SHA384"),
                figure("tls:post-quantum", "on", "", "on"),
                figure("tls:server-name", "masque.example.com", "", "masque.example.com")
            ] }),
            Object.assign(figure("dns", "134 ms", "8 of 12 answered", "134.08333333333334"),
                { parts: [
                    figure("dns:proxy", "on", "", "on"),
                    figure("dns:average", "134 ms", "", "134.08333333333334"),
                    figure("dns:asked", "12", "", "12"),
                    figure("dns:answered", "8", "", "8"),
                    figure("dns:no-record", "4", "", "4"),
                    figure("dns:timed-out", "0", "", "0"),
                    figure("dns:failed", "0", "", "0")
                ] })
        ]);
        // Every part is named by the vocabulary, inside the part that holds
        // it, so none reads as its id.
        const nameless = [];
        const partsNamed = (parts, within) => {
            for (const part of parts ?? []) {
                // An entry of a list is named by itself, as a host is.
                const word = vocabulary.find(thing => thing.id === `warp:${part.id}`);
                if (part.name === undefined && (!word || !word.name || word.in !== within))
                    nameless.push(part.id);
                partsNamed(part.parts, `warp:${part.id}`);
            }
        };
        partsNamed(linked && linked.parts, "warp");
        machine("every part of WARP named where it is", nameless, []);
        // Named as Network names the same figures elsewhere, Traffic's
        // Round Trip among them (VKCF3Z and YI77D3, 2026-11-08).
        machine("WARP's figures named as Network names them",
            ["latency", "colo", "traffic"].map(id =>
                (vocabulary.find(thing => thing.id === `warp:${id}`) ?? {}).name),
            ["Round Trip", "Data Centre", "Traffic"]);
        // A run of items joins with dots, never commas (docs/design.md,
        // Typesetting).
        const commas = [];
        const noCommas = parts => {
            for (const part of parts ?? []) {
                if (`${part.side}${part.note}`.includes(","))
                    commas.push(part.id);
                noCommas(part.parts);
            }
        };
        noCommas(linked && linked.parts);
        machine("no part of WARP joins its words with a comma", commas, []);
        const slight = warpTold(Object.assign({}, warp, { tunnel: Object.assign({}, warp.tunnel,
            { loss_fraction: 0.0053708, received_bytes: null }) }));
        machine("a slight loss keeps three digits, and one way's traffic says only it",
            slight && slight.parts.filter(part => ["loss", "traffic"].includes(part.id))
                .map(part => part.side),
            ["0.537%", "9.26 GB out"]);
        const degraded = warpTold(Object.assign({}, warp, { status: { state: "Connected",
            reason: "NetworkDegraded", reason_figures: { loss_percent: 5.3708224, rtt_ms: 170 } } }));
        machine("WARP degraded, as its row says it", degraded && [degraded.headline, degraded.warns],
            ["degraded \u00b7 127 ms", true]);
        // Its state alone at the side, which never gives way; its figures in
        // its note, whose they are first, since the note gives way from its
        // end and Round Trip and Loss below are the tunnel's own (VKCF3Z).
        machine("WARP's health degraded, saying whose measure first",
            degraded && degraded.parts.find(part => part.id === "health"),
            { id: "health", side: "degraded",
                note: "WARP's measure \u00b7 5.37% loss \u00b7 170 ms round trip",
                copy: "degraded", warns: true });
        const down = warpTold({ status: { state: "Disconnected", reason: "Manual",
            reason_figures: null }, tunnel: null, settings: null, dns: null });
        machine("WARP disconnected, its tunnel unknown", down && [down.headline, down.parts], [
            "disconnected",
            [figure("state", "disconnected", "manual", "disconnected")]
        ]);
        const connecting = warpTold({ status: { state: "Connecting",
            reason: "PerformingHappyEyeballs", reason_figures: null }, tunnel: null });
        machine("WARP connecting, saying why", connecting && connecting.parts[0].note,
            "performing happy eyeballs");
        const thin = warpTold(Object.assign({}, warp, { tunnel: { on: true, latency_ms: null,
            loss_fraction: null, protocol: "WireGuard", edge: null, sent_bytes: null,
            received_bytes: null }, settings: null, dns: null }));
        machine("a figure warp didn't give is left out",
            thin && thin.parts.map(part => part.id), ["state", "health", "protocol"]);
        const off = warpTold(Object.assign({}, warp, { tunnel: { on: false, v4_endpoint: null,
            v6_endpoint: "2001:db8::2" } }));
        machine("a tunnel that's off, its one endpoint IPv6",
            off && off.parts.filter(part => ["tunnel", "endpoint"].includes(part.id)), [
            figure("tunnel", "off", "", "off"),
            figure("endpoint", "2001:db8::2", "no IPv4", "2001:db8::2")
        ]);
        const policed = warpTold(Object.assign({}, warp, { settings: { mode: "tunnel_only",
            always_on: false, split_tunnel: { mode: "exclude", hosts: 1, ips: 0 },
            set_by: { mode: "someone_else" } }, override: null,
            dns: { proxy_enabled: false, average_ms: null,
            total: 0, success: 0, no_records_found: 0, timed_out: 0, other_error: 0 } }));
        machine("settings warp gave some of, each said as it's set",
            policed && policed.parts.find(part => part.id === "settings"),
            Object.assign(figure("settings", "tunnel only", "split tunnel", "tunnel_only"),
                { parts: [
                    figure("settings:mode", "tunnel only",
                        "the tunnel alone \u00b7 set by someone else", "tunnel_only"),
                    figure("settings:always-on", "off", "", "off"),
                    figure("settings:split-tunnel", "exclude", "1 host", "exclude")
                ] }));
        // An organisation disconnecting every device says so right after
        // the state, warning; an override in force says how long it has.
        const emergency = warpTold(Object.assign({}, warp, {
            settings: Object.assign({}, warp.settings, { emergency_disconnect: true }),
            override: { set: true, ends_in_secs: 700 } }));
        machine("an emergency disconnect said after the state, warning",
            emergency && emergency.parts[1], Object.assign(figure("emergency", "on",
                "every device disconnected", "on"), { warns: true }));
        machine("an override in force, with the minutes it has left",
            emergency && emergency.parts.find(part => part.id === "settings").parts
                .find(part => part.id === "settings:override"),
            figure("settings:override", "on", "12 min left", "on"));
        machine("a DNS proxy that's off says so in its place",
            policed && (policed.parts.find(part => part.id === "dns") ?? {}).side, "proxy off");
        // Not answering, its note says what warp-cli's error means, and the
        // error as written is kept for the drawer (VKCF3Z, 2026-11-09).
        const notAnswering = said => Warp.failed({ error: "not-answering", said: said }, warpSaid);
        machine("WARP's service not answering, with no socket to reach",
            notAnswering("No such file or directory (os error 2)"), { headline: "not answering",
                warns: true, note: "its service isn't running",
                error: "No such file or directory (os error 2)" });
        machine("WARP's service not answering, its socket refusing",
            notAnswering("Connection refused (os error 111)").note, "its service isn't running");
        machine("WARP's service not answering, its socket shut to this account",
            notAnswering("Permission denied (os error 13)").note, "its service won't let you in");
        machine("WARP's service not answering in time",
            notAnswering("warp-cli status didn't answer within 5 s").note,
            "its service is too slow to answer");
        machine("WARP's service not answering for a reason warp has no words for",
            notAnswering("Address not available (os error 99)"), { headline: "not answering",
                warns: true, note: "its service can't be reached",
                error: "Address not available (os error 99)" });
        machine("warp-cli saying what warp can't read", Warp.failed({ error: "unreadable",
            said: "warp-cli status said InvalidAccountType: Invalid account type." }, warpSaid),
        { headline: "unreadable", warns: true,
            note: "warp-cli status said InvalidAccountType: Invalid account type." });
        machine("no warp-cli, so no WARP", Warp.failed({ error: "not-installed", said: "warp-cli" },
            warpSaid), null);
        machine("warp-cli's words as said", Warp.spoken("NetworkHealthy"), "network healthy");
        // What the shell keeps of WARP's reads while it's shown, two seconds
        // apart, sparks the tunnel's Round Trip, Loss and Traffic, each with
        // a drawer saying once whose figures they are (VKCF3Z, 2026-11-08).
        const warpRead = (latency, loss, received, sent) => Object.assign({}, warp, {
            tunnel: Object.assign({}, warp.tunnel, { latency_ms: latency, loss_fraction: loss,
                received_bytes: received, sent_bytes: sent }) });
        const reads = [warpRead(120, 0.01, 1000, 500), warpRead(140, 0.02, 3000, 1500),
            warpRead(130, 0.015, 7000, 2500)];
        const seen = reads.reduce((kept, each, at) => WarpSeen.saw(kept, each, at * 2000), null);
        machine("WARP keeps what it read while shown, and the speed each way between",
            [seen.latency, seen.loss, seen.inward, seen.outward],
            [[120, 140, 130], [0.01, 0.02, 0.015], [1000, 2000], [500, 500]]);
        const spark = values => ({ shape: "spark", values, warn: false });
        const over = (values, least) => values.map(value => value / Math.max(least, ...values));
        const sparked = WarpSeen.drawn(Warp.told(reads[2], {}, warpSaid).parts, seen, {}, warpSaid);
        const drawn = id => (sparked.find(part => part.id === id) ?? {});
        const tunnel = "via WARP \u00b7 tunnel";
        const trip = spark(over([120, 140, 130], 100));
        machine("Round Trip sparks what was read, its drawer saying whose",
            [drawn("latency").graph, drawn("latency").drawer], [trip, { eyebrow: tunnel, facts: [
                { id: "range", value: "120 ms to 140 ms", figure: trip },
                { id: "reads", value: "3 over 4 s" }] }]);
        // A range whose low and high read the same at three digits says it's
        // steady, as a run, rather than "127 ms to 127 ms" (VKCF3Z, 2026-11-08).
        const still = [warpRead(127, 0.01, 1000, 500), warpRead(127.2, 0.01, 2000, 1000)];
        const steadily = still.reduce((kept, each, at) => WarpSeen.saw(kept, each, at * 2000), null);
        const steady = WarpSeen.drawn(Warp.told(still[1], {}, warpSaid).parts, steadily, {}, warpSaid)
            .find(part => part.id === "latency") ?? {};
        machine("a Round Trip that reads the same throughout says it's steady",
            steady.drawer?.facts?.[0]?.value, "127 ms \u00b7 steady");
        const lost = spark(over([0.01, 0.02, 0.015], 0.05));
        machine("Loss sparks against five percent at least, so a calm link reads small",
            [drawn("loss").graph, drawn("loss").drawer], [lost, { eyebrow: tunnel, facts: [
                { id: "range", value: "1.00% to 2.00%", figure: lost },
                { id: "reads", value: "3 over 4 s" }] }]);
        machine("Traffic sparks both ways together, its drawer the speed each way now",
            [drawn("traffic").graph, drawn("traffic").drawer], [spark(over([1500, 2500], 0)), {
                eyebrow: tunnel, facts: [
                    { id: "in", value: "16.0 kb/s", figure: spark(over([1000, 2000], 0)) },
                    { id: "out", value: "4.00 kb/s", figure: spark(over([500, 500], 0)) }] }]);
        machine("one read sparks nothing, so no drawer says only the row again",
            WarpSeen.drawn(Warp.told(reads[0], {}, warpSaid).parts, WarpSeen.saw(null, reads[0], 0),
                {}, warpSaid).filter(part => part.graph !== undefined || part.drawer !== undefined),
            []);
        const anew = WarpSeen.saw(seen, { status: { state: "Disconnected", reason: "Manual" },
            tunnel: null }, 6000);
        machine("a state changed starts the reads again",
            [anew.latency, anew.loss, anew.inward], [[], [], []]);
        const back = WarpSeen.saw(seen, warpRead(125, 0.01, 10, 5), 6000);
        machine("a counter gone back, as a link made again starts it, gives no speed",
            [back.latency.length, back.inward], [4, [1000, 2000]]);
        // WARP's switch, :connect and :disconnect: who holds it, where the
        // organisation does, and when WARP always on comes back, as warp's
        // own switch says it (modules/warp/src/switch.rs).
        const locked = Object.assign({}, warp, {
            settings: Object.assign({}, warp.settings, { switch_locked: true }) });
        machine("WARP's switch says when, always on, it comes back, who holds it, or nothing",
            [WarpSwitch.switchNote(warp, true, warpSaid),
                WarpSwitch.switchNote(locked, true, warpSaid),
                WarpSwitch.switchNote(warp, false, warpSaid)],
            ["comes back by itself in 2 min", "example holds the switch", ""]);
        // A switch the organisation holds draws no chord, since the foot
        // teaches only what works; Return stays and is refused with why
        // (VKCF3Z, 2026-11-08).
        machine("WARP's switch ends the link while up, unless the organisation holds it",
            [WarpSwitch.ends(warp, true), WarpSwitch.ends(locked, true),
                WarpSwitch.ends(warp, false)], [true, false, false]);
        machine("WARP's switch names who holds it, so it ends nothing, or nobody",
            [WarpSwitch.holder(locked), WarpSwitch.holder(warp)], ["example", ""]);
        machine("WARP up or on its way, so its switch takes it down",
            ["Connected", "Connecting", "Disconnected"].map(state =>
                WarpSwitch.up({ status: { state } })), [true, true, false]);
        machine("a switch done, held, refused or unread, as warp says each", [
            WarpSwitch.switched({ act: "disconnect", done: true, refused: null,
                comes_back_after_min: 2 }, warpSaid),
            WarpSwitch.switched({ act: "disconnect", done: false, refused: "held",
                organisation: "example" }, warpSaid),
            WarpSwitch.switched({ act: "connect", done: false, refused: "refused",
                said: "warp-cli wouldn't connect: Invalid account type." }, warpSaid),
            WarpSwitch.switched({ error: "not-answering", said: "its service didn't answer" },
                warpSaid)
        ], [
            { refused: "", note: "comes back by itself in 2 min" },
            { refused: "example holds the switch", note: "" },
            { refused: "warp-cli wouldn't connect: Invalid account type.", note: "" },
            { refused: "its service didn't answer", note: "" }
        ]);

        // How sleep stands, from the daemon's sleep:main record, in Sleep's
        // words: a machine that failed twice freezing and once resuming,
        // last freezing, held by a device.
        const sleepWords = { failed: "failed while {step}", at: "{count} while {step}",
            time: "1 time", times: "{count} times", freeze: "freezing", resume: "resuming" };
        const slept = Power.sleepOf({ key: "sleep:main", mode: "s2idle", modes: ["s2idle", "deep"],
            states: ["freeze", "mem", "disk"], successes: 12, failures: 3,
            failed_at: { freeze: 2, prepare: 0, resume: 1, suspend: null },
            last_failed_device: "0000:00:14.0", last_failed_step: "freeze",
            last_failed_errno: -16, last_hardware_sleep_us: 3_600_000_000,
            total_hardware_sleep_us: 0, hardware_sleep_counts_to_us: 523_986_009_990 });
        machine("sleep's failures by step, in the order taken", slept.failedAt,
            [{ step: "freeze", count: 2 }, { step: "resume", count: 1 }]);
        machine("sleep's other modes", slept.others, ["deep"]);
        machine("sleep's hardware time in seconds", [slept.lastHardware, slept.countsTo],
            [3600, 523986.00999]);
        const sleepErrors = { EBUSY: "a device was busy" };
        const sleepTold = Power.sleepSaid(slept, sleepWords, Interfaces.lasted, sleepErrors);
        machine("sleep's mode and the step it last failed at", [sleepTold.side, sleepTold.note],
            ["s2idle", "failed while freezing \u00b7 2 times"]);
        machine("sleep's drawer, the failures the note doesn't say, the device last",
            sleepTold.facts, [
            { id: "sleep:others", value: "deep" },
            { id: "sleep:ways", value: "freeze \u00b7 mem \u00b7 disk" },
            { id: "sleep:slept", value: "12 times" },
            { id: "sleep:failed", value: "1 while resuming" },
            { id: "sleep:error", value: "a device was busy" },
            { id: "sleep:hardware", value: "1 h" },
            { id: "sleep:counts-to", value: "6 d 1 h" },
            { id: "sleep:device", value: "0000:00:14.0", identifies: true },
            { id: "sleep:code", value: "EBUSY", identifies: true }
        ]);
        const neverFailed = Power.sleepSaid(Power.sleepOf({ mode: "s2idle", modes: ["s2idle"],
            successes: 1, failures: 0, failed_at: {}, last_failed_step: "",
            last_failed_errno: null, last_hardware_sleep_us: null }), sleepWords, Interfaces.lasted,
            sleepErrors);
        machine("sleep that never failed says only its mode",
            [neverFailed.note, neverFailed.facts],
            ["", [{ id: "sleep:slept", value: "1 time" }]]);
        machine("sleep before the daemon answers", Power.sleepOf(null), null);
        machine("an error Linux names", Power.errorOf(-5), "EIO");

        // What keeps the machine awake, from the daemon's awake:main:
        // a call holding idle off, a download holding sleep, Keep Awake's
        // own two locks left out, two delays, one of them twice, and a
        // sleep of six hours. Its words are Keep Awake's own, so a word
        // the drawer needs and the vocabulary lacks fails here.
        const awakeWords = vocabulary.find(thing => thing.id === "session:awake").said;
        const awakeAt = 1_790_000_000_000;
        const clock = at => `${(at - awakeAt) / 60_000} min`;
        const logind = { idle: false, idle_since_ms: awakeAt - 600_000, preparing: false,
            idle_action: "suspend", idle_action_ms: 300_000, blocked: ["idle", "sleep"],
            delayed: ["sleep"], inhibitors: [
                { what: ["idle"], who: "Google Chrome", why: "Playing video", mode: "block" },
                { what: ["sleep", "idle"], who: "mycelium", why: "holding work",
                    mode: "block" },
                { what: ["idle"], who: "Keep Awake", why: "Kept awake from the launcher",
                    mode: "block" },
                { what: ["handle-lid-switch"], who: "Keep Awake", why: "", mode: "block" },
                { what: ["sleep"], who: "NetworkManager", why: "NetworkManager needs to",
                    mode: "delay" },
                { what: ["sleep"], who: "swayidle", why: "Lock before sleep", mode: "delay" },
                { what: ["sleep"], who: "swayidle", why: "Again", mode: "delay" },
                { what: ["sleep"], who: "1password", why: "Locking", mode: "delay" }],
            last_sleep: { began_at_ms: awakeAt - 7_200_000 - 21_780_000,
                ended_at_ms: awakeAt - 7_200_000 } };
        const awakeTold = plugged => ({ plugged, when: clock,
            lasted: Interfaces.lasted, named: who => ({ "1password": "1Password" })[who] ?? who });
        machine("keep awake's drawer, others' locks first, its own left out",
            Awake.facts(logind, awakeWords, awakeTold(false)), [
            { id: "awake:idle-held",
                value: "Google Chrome: Playing video \u00b7 mycelium: holding work" },
            { id: "awake:sleep-held", value: "mycelium: holding work" },
            { id: "awake:idle-sleep", value: "suspends 5 min after the screens go off" },
            { id: "awake:put-off", value: "NetworkManager \u00b7 swayidle \u00b7 1Password" },
            { id: "awake:slept", value: "-483 min \u00b7 6 h 3 min" }
        ]);
        const idleNow = Object.assign({}, logind, { idle: true, idle_since_ms: awakeAt - 60_000,
            inhibitors: [], last_sleep: { began_at_ms: awakeAt, ended_at_ms: null } });
        // Nobody else holding a lock says none, and a sleep still going
        // says nothing: only how long after the screens go off it sleeps.
        machine("with no other locks it says its span alone, a sleep still going nothing",
            Awake.facts(idleNow, awakeWords, awakeTold(false)),
            [{ id: "awake:idle-sleep", value: "suspends 5 min after the screens go off" }]);
        const idleSaid = (action, plugged) => Awake.facts({ idle_action: action,
            idle_action_ms: 300_000 }, awakeWords, awakeTold(plugged))[0]?.value ?? "";
        machine("idle sleep as logind is set", [idleSaid("lock", true), idleSaid("lock", false),
            idleSaid("ignore", true), idleSaid("hibernate", false), idleSaid("halt", false),
            idleSaid("kexec", false), idleSaid("", false)],
            ["never on the charger \u00b7 it only locks", "never \u00b7 it only locks", "never",
                "hibernates 5 min after the screens go off", "halts 5 min after the screens go off",
                "reboots without its firmware 5 min after the screens go off", ""]);
        machine("an error with no name kept", Power.errorOf(-71), "\u221271");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
