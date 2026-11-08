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
import "Colours.js" as Colours
import "Asking.js" as Asking
import "Machine.js" as Machine
import "Heat.js" as Heat
import "Power.js" as Power
import "Links.js" as Links
import "Keyboards.js" as Keyboards
import "KeyCounts.js" as KeyCounts
import "Standing.js" as Standing
import "Traffic.js" as Traffic
import "Sounds.js" as Sounds
import "Cameras.js" as Cameras
import "Firmware.js" as Firmware
import "Interfaces.js" as Interfaces
import "Networkd.js" as Networkd
import "WifiLink.js" as WifiLink
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
                "why:tooFewSamples": "too few samples yet" }
        };
        const evening = at("2026-11-04T22:00");
        const day = new Date(2026, 10, 4);
        const moon = Moment.moon(evening);
        const season = Moment.season(evening, 43.67);
        const figures = [
            ["UTC", Moment.utcClock(evening + 7000), "22:00:07"],
            ["Unix time", Moment.unix(evening), 1793829600],
            ["TAI", Moment.tai(evening), "22:00:37"],
            ["GPS time", quoted(Moment.gps(evening)), quoted({ week: 2443, seconds: 338418 })],
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
            // many polls were answered; and where a server stands.
            ["nanoseconds", Keeping.seconds(2.98e-8, false), "30 ns"],
            ["microseconds, signed", Keeping.seconds(-0.000388, true), "\u2212388 \u00b5s"],
            ["milliseconds", Keeping.seconds(0.036112, false), "36.1 ms"],
            ["a microsecond short of a millisecond", Keeping.seconds(0.0009996, false),
                "1.0 ms"],
            ["seconds", Keeping.seconds(1.5, true), "+1.500 s"],
            ["a clock behind", Keeping.out(-0.000012, KeepingCases.clock), "12 \u00b5s slow"],
            ["a clock ahead", Keeping.out(0.31, KeepingCases.clock), "310.0 ms fast"],
            ["a clock on time", Keeping.out(0, KeepingCases.clock), "on time"],
            ["a clock running fast", Keeping.runs(6.267, KeepingCases.clock), "6.267 ppm fast"],
            ["a clock running slow", Keeping.runs(-3.225, KeepingCases.clock), "3.225 ppm slow"],
            ["a span in minutes", Keeping.span(1024), "17 min 4 s"],
            ["a span in hours", Keeping.span(86400), "24 h"],
            ["a span under a second", Keeping.span(0.5), "0.5 s"],
            ["a drift in a day", Keeping.daily(2.1, KeepingCases.clock), "gains 181.4 ms a day"],
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
                KeepingCases.servers), "set aside: too variable"]
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
        const cores = loads => loads.map((load, n) => ({ n: n, load: load }));
        machine("no core maxed", Machine.maxed(cores([0.5, 0.94])), "none");
        machine("one core maxed", Machine.maxed(cores([0.5, 0.95])), "core 1");
        machine("three cores maxed", Machine.maxed(cores([1, 0.2, 0.99, 0.97])), "cores 0, 2 and 3");
        machine("most cores maxed", Machine.maxed(cores([1, 1, 1, 1, 1, 0])), "5 of 6");
        machine("a list of CPUs", Machine.cpus("0-3,8\n"), [0, 1, 2, 3, 8]);
        machine("no list of CPUs", Machine.cpus(""), []);
        machine("one core named", Machine.cores([4]), "core 4");
        machine("cores unbroken", Machine.cores([0, 1, 2, 3]), "cores 0 to 3");
        machine("cores broken", Machine.cores([0, 2, 5]), "cores 0, 2 and 5");
        machine("no cores", Machine.cores([]), "");
        // Where the processor's time went, interrupts both kinds together
        // and guests' both kinds, each field the kernel keeps.
        const timeSpent = { user: 0.4, nice: 0, system: 0.1, idle: 0.4, iowait: 0.05, irq: 0.01,
            softirq: 0.02, steal: 0.02, guest: 0.01, guest_nice: 0 };
        machine("the processor's time spent inside another machine",
            Machine.spent(timeSpent, true).map(each => [each.id, Math.round(each.share * 100)]),
            [["user", 40], ["system", 10], ["iowait", 5], ["interrupts", 3], ["niced", 0],
                ["stolen", 2], ["guests", 1]]);
        machine("the processor's time spent on its own hardware, none stolen",
            Machine.spent(timeSpent, false).map(each => each.id),
            ["user", "system", "iowait", "interrupts", "niced", "guests"]);
        machine("an old kernel's time, without steal or guests",
            Machine.spent({ user: 0.5, nice: 0, system: 0.1, idle: 0.4, iowait: 0, irq: 0, softirq: 0 },
                true).map(each => each.id), ["user", "system", "iowait", "interrupts", "niced"]);
        machine("no time said", Machine.spent(null, false), []);
        // What keeps the memory, biggest first; files are the page cache
        // less what's shared, which the kernel counts in it.
        const gib = 1073741824;
        machine("what keeps the memory", Machine.keptBy({ AnonPages: 20 * gib, Cached: 12 * gib,
            Shmem: 2 * gib, Slab: gib / 2, KernelStack: gib / 2, PageTables: gib / 2, Buffers: gib / 4,
            Dirty: gib / 8, Writeback: 0 }).map(part => [part.id, part.bytes / gib]),
            [["programs", 20], ["files", 10], ["shared", 2], ["kernel", 1.5], ["buffers", 0.25],
                ["unwritten", 0.125]]);
        machine("no memory said", Machine.keptBy(null), []);
        machine("what zswap holds", Machine.compressed({ Zswap: gib, Zswapped: 4 * gib }),
            { stored: 4 * gib, pool: gib, ratio: 4 });
        machine("zswap holding nothing", Machine.compressed({ Zswap: 0, Zswapped: 0 }), null);
        // Modules alike counted once, where they sit gathered, blanks left
        // blank.
        const module = { locator: "Controller0-ChannelA", size: "8 GB", type: "LPDDR5X",
            speed: "7467 MT/s", manufacturer: "SK Hynix", part: "" };
        machine("the memory modules alike", Machine.modules([module,
            Object.assign({}, module, { locator: "Controller0-ChannelB" })]),
            [{ type: "LPDDR5X", speed: "7467 MT/s", size: "8 GB", manufacturer: "SK Hynix", part: "",
                locator: "Controller0-ChannelA", count: 2,
                locators: ["Controller0-ChannelA", "Controller0-ChannelB"] }]);
        machine("modules of two speeds", Machine.modules([module,
            Object.assign({}, module, { speed: "6400 MT/s" })]).length, 2);
        machine("no memory modules", Machine.modules(null), []);
        machine("a disk's warnings", Machine.warnings(["spare", "readonly", "novel"]),
            ["spare", "read-only", "novel"]);
        machine("a disk warning of nothing", Machine.warnings([]), []);
        machine("the machine's disk, the biggest", Machine.biggest([{ key: "disk:sda", size_b: 1 },
            { key: "disk:nvme0n1", size_b: 10 }]).key, "disk:nvme0n1");
        machine("no disk", Machine.biggest([]), null);
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

        // Heat's parts, from the chips as Sensors.qml reads the laptop's,
        // in degrees: the processor's, its cores numbered by the logical
        // processor each is, the disk's with its hidden second sensor,
        // dell_ddv's named sensors and fans, dell_smm's unnamed fans
        // telling their most and power, and the battery's.
        const heatTemp = (n, label, value, high, crit) => ({ n, label, value, high, crit });
        const heatFan = (n, label, value, more) => Object.assign({ n, label, value }, more ?? {});
        const heatChips = [
            { part: "disk", temps: [heatTemp(1, "Composite", 52.85, 82.85, 84.85),
                heatTemp(2, "Sensor 1", 53.85, null, null),
                heatTemp(3, "Sensor 2", 68.85, null, null)], fans: [] },
            { part: "processor", temps: [heatTemp(1, "Package id 0", 83, 100, 100),
                heatTemp(2, "Core 0", 82, 100, 100), heatTemp(6, "Core 8", 84, 100, 100),
                heatTemp(10, "Core 22", 79, 100, 100)], fans: [] },
            { part: "", temps: [heatTemp(1, "CPU", 84, null, null),
                heatTemp(6, "SODIMM", 56, null, null), heatTemp(7, "Ambient", 45, null, null)],
                fans: [heatFan(1, "Video Fan", 5420), heatFan(2, "CPU Fan", 5385)] },
            { part: "", temps: [], fans: [
                heatFan(1, "", 5420, { max: 4800, target: 4800, power: 1 }),
                heatFan(2, "", 5385, { max: 4800, target: 4800, power: 0.5 })] },
            { part: "battery", temps: [heatTemp(1, "temp", 36.9, null, null)], fans: [] }
        ];
        const heatParts = Heat.gather(heatChips, [], Heat.coreCpus([{ cpu: 0, core: 0 },
            { cpu: 1, core: 22 }, { cpu: 8, core: 8 }]));
        machine("heat's parts, the processor first", heatParts.temps.map(part => part.id),
            ["processor", "disk", "memory", "battery", "air"]);
        const heatProcessor = heatParts.temps[0];
        machine("the processor's heat, as its package reads",
            [heatProcessor.value, heatProcessor.high, heatProcessor.crit], [83, 100, 100]);
        machine("each core's heat, by its logical processor", heatProcessor.cores,
            [{ cpu: 0, value: 82 }, { cpu: 1, value: 79 }, { cpu: 8, value: 84 }]);
        const heatDisk = heatParts.temps[1];
        machine("the disk's heat, as its composite reads",
            [heatDisk.value, heatDisk.high, heatDisk.crit], [52.85, 82.85, 84.85]);
        machine("the disk's other sensors, the hotter hidden one too",
            heatDisk.sensors.map(s => [s.label, s.value]),
            [["Sensor 1", 53.85], ["Sensor 2", 68.85]]);
        machine("the fans by name, with what the unnamed chip says of them", heatParts.fans,
            [{ id: "video-fan", n: 1, label: "Video Fan", value: 5420, max: 4800, target: 4800,
                power: 1 },
                { id: "cpu-fan", n: 2, label: "CPU Fan", value: 5385, max: 4800, target: 4800,
                    power: 0.5 }]);
        machine("fans no chip names, by number", Heat.fans([heatChips[3]]).map(f => [f.id, f.n]),
            [["fan", 1], ["fan", 2]]);
        machine("no fans", Heat.fans([heatChips[0]]), []);
        machine("a thermal zone for a part no chip gives",
            Heat.gather([heatChips[4]], [{ part: "processor", value: 78 }], {}).temps
                .map(p => p.id), ["processor", "battery"]);
        machine("a thermal zone for a part a chip gives",
            Heat.gather(heatChips, [{ part: "processor", value: 78 }], {}).temps
                .filter(p => p.id === "processor").map(p => p.value), [83]);
        machine("whether a chip gives a part", [Heat.gives(heatChips, "air"),
            Heat.gives(heatChips, "wifi")], [true, false]);
        machine("a thermal zone by its type", ["x86_pkg_temp", "acpitz", "iwlwifi_1", "SEN1",
            "INT3400 Thermal"].map(Heat.zonePart), ["processor", "board", "wifi", "", ""]);
        // The processor's telemetry, as its reader says it on the laptop:
        // the parts Heat lists come after the graphics, each held to the
        // processor's points, and the rest are left out.
        const heatOwn = [{ part: "chipset", celsius: 62 }, { part: "npu", celsius: 62 },
            { part: "display", celsius: 60 }, { part: "imageProcessor", celsius: 63 },
            { part: "systemAgent", celsius: 65 }, { part: "media", celsius: 59 }];
        machine("the processor's own parts, after the processor",
            Heat.gather(heatChips, [], {}, heatOwn).temps.map(p => [p.id, p.value]),
            [["processor", 83], ["npu", 62], ["image-processor", 63], ["media-engine", 59],
                ["display-engine", 60], ["disk", 52.85], ["memory", 56], ["battery", 36.9],
                ["air", 45]]);
        machine("the processor's own parts, held to its points",
            Heat.gather(heatChips, [], {}, heatOwn).temps.filter(p => p.id === "npu")
                .map(p => [p.high, p.crit]), [[100, 100]]);
        machine("the processor's own parts, with no processor said",
            Heat.gather([heatChips[4]], [], {}, [{ part: "npu", celsius: 62 }]).temps
                .map(p => [p.id, p.crit]), [["npu", null], ["battery", null]]);
        // What held the processor back, as its telemetry says it: the whole
        // processor's limit first, and each core by its cluster's.
        const heldReading = {
            heldBack: [{ by: "pl2", share: 0.4 }, { by: "pl1", share: 1 }],
            clusters: [{ cpus: "4-5", heldBack: [{ by: "power", share: 0.98 },
                { by: "heat", share: 0.1 }] }, { cpus: "0", heldBack: [] },
                { cpus: "2", heldBack: [{ by: "heat", share: 0.5 }] }],
            npu: []
        };
        machine("held back, by the whole processor's limit", Machine.heldBack(heldReading),
            { share: 1, by: "pl1", whole: true, cores: [{ n: 2, share: 0.5, by: "heat" },
                { n: 4, share: 0.98, by: "power" }, { n: 5, share: 0.98, by: "power" }] });
        machine("held back by a cluster alone", Machine.heldBack({ heldBack: [],
            clusters: [{ cpus: "12-13", heldBack: [{ by: "heat", share: 0.3 }] }] }),
            { share: 0.3, by: "heat", whole: false, cores: [{ n: 12, share: 0.3, by: "heat" },
                { n: 13, share: 0.3, by: "heat" }] });
        machine("held back by nothing", [Machine.heldBack({ heldBack: [], clusters: [
            { cpus: "0", heldBack: [] }] }), Machine.heldBack(null)], [null, null]);
        machine("what held it back, as a note says it", ["pl1", "platformPl1", "prochot",
            "heat", "novel"].map(Machine.limit), ["its power limit, PL1",
            "the whole machine's power limit, PL1", "being told to slow down", "heat", "novel"]);
        // What kept the package from resting, over the readings held: each
        // part's share averaged over all of them, a part under a twentieth
        // left out, longest first.
        const rested = [{ kept: 0.2, by: [{ by: "display", share: 0.2 }] },
            { kept: 0.1, by: [{ by: "display", share: 0.06 }, { by: "npu", share: 0.1 }] }];
        const restedKept = Machine.kept(rested);
        machine("kept from resting, on average", [restedKept.by.map(p => p.by),
            restedKept.by.map(p => Math.round(p.share * 100)), Math.round(restedKept.share * 100),
            restedKept.quiet], [["display", "npu"], [13, 5], 15, false]);
        // Under a twentieth on average, nothing keeps it from resting: one
        // second's 30% among six quiet ones is just over 5%, among seven
        // just under.
        const quiet = { kept: 0.01, by: [] };
        const once = { kept: 0.3, by: [{ by: "cores", share: 0.3 }] };
        machine("kept from resting, quiet under a twentieth on average", [
            Machine.kept([quiet, quiet, quiet, quiet, quiet, quiet, once]).quiet,
            Machine.kept([quiet, quiet, quiet, quiet, quiet, quiet, quiet, once]).quiet],
            [false, true]);
        machine("kept from resting, said by nothing", Machine.kept([]), null);
        machine("what kept it from resting, as a note says it", ["display", "internal",
            "novel"].map(by => Machine.keeper(by).said), ["the display engine",
            "the processor's own power controller", "novel"]);
        // The NPU's clock to three significant digits, and "resting"
        // while it's powered down.
        machine("the NPU's clock", Machine.npuClock({ clock_mhz: 1400, max_mhz: 2050 }),
            "at 1.40 GHz of 2.05 GHz");
        // Bytes in decimal units to three significant digits, each
        // converted from bytes, stepping up past 999, whole below a kB.
        machine("bytes, each converted", [873 * 1048576, 6.4 * 1048576, 1.52e9, 873, 0, 999.7e3,
            4096].map(Machine.bytes), ["915 MB", "6.71 MB", "1.52 GB", "873 B", "0 B",
            "1.00 MB", "4.10 kB"]);
        machine("a rate", Machine.rate(12000), "12.0 kB/s");
        machine("a clock from kilohertz", [4700000, 400000].map(Machine.hertz),
            ["4.70 GHz", "400 MHz"]);
        machine("three significant digits", [0.04, 1.234, 12.34, 123.4].map(Machine.digits),
            ["0.0400", "1.23", "12.3", "123"]);
        // Two rates say which is which, and none while both are still.
        machine("rates both ways", Machine.both(12000, 0), "12.0 kB/s in \u00b7 0 B/s out");
        machine("rates both still", Machine.both(0, 0), "none");
        machine("a note's items, the empty given way", Machine.items(["btrfs", "", null, "dm-0"]),
            "btrfs \u00b7 dm-0");
        machine("the NPU resting", Machine.npuClock({ clock_mhz: 0, max_mhz: 2050 }), "resting");
        machine("no telemetry, no parts of its own", Heat.gather(heatChips, [], {}, null).temps
            .length, 5);
        machine("two disks, numbered", Heat.gather([heatChips[0], heatChips[0]], [], {}).temps
            .map(p => p.nth), [1, 2]);
        machine("a lone disk, unnumbered", heatDisk.nth, 0);
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
        machine("a steady temperature's spark", Heat.spark([83, 83]), [0.5, 0.5]);
        machine("a climbing temperature's spark", Heat.spark([80, 85, 90]), [0, 0.5, 1]);
        machine("a spark with nothing read", Heat.spark([]), []);
        machine("a fan's spark, drawn across its span", Heat.fanSpark([5000, 5250]), [0.25, 0.75]);
        // The laptop's own counts: the package's events as each processor
        // saw them, and two processors sharing a core, counting it once.
        const heatThrottled = Heat.throttled([
            { cpu: 0, core: 0, cores: 24, package: 3191 },
            { cpu: 1, core: 22, cores: 8, package: 265 },
            { cpu: 2, core: 0, cores: 0, package: 3202, packageMs: 19016, packageLongest: 125 }
        ]);
        machine("the processor throttled, as the package", [heatThrottled.times, heatThrottled.ms,
            heatThrottled.longest], [3202, 19016, 125]);
        machine("each core throttled, once a core", [heatThrottled.cores, heatThrottled.total],
            [[{ cpu: 0, count: 24 }, { cpu: 1, count: 8 }], 32]);
        machine("no throttling counted", Heat.throttled([]), null);
        machine("each core's first processor", Heat.coreCpus([{ cpu: 0, core: 0 },
            { cpu: 1, core: 0 }, { cpu: 2, core: 4 }]), { 0: 0, 4: 2 });
        machine("a part's name in a sentence", ["Processor", "Wi-Fi Card", "Air Inside", "HDR"]
            .map(Heat.phrase), ["processor", "Wi-Fi card", "air inside", "HDR"]);
        machine("a fan's id", Heat.fanId("Video Fan"), "video-fan");
        machine("a sensor whose label says nothing", [Heat.labelled("CPU"), Heat.labelled("Other"),
            Heat.labelled("SODIMM")], ["", "", "memory"]);

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

        // A Wi-Fi card's link as iw reads it.
        const networkIw = "@@ link\nConnected to 02:00:00:00:02:00 (on wlp0s20f3)\n"
            + "\tSSID: Lantern\n\tfreq: 5745.0\n\tsignal: -43 dBm\n"
            + "@@ station\nStation 02:00:00:00:02:00 (on wlp0s20f3)\n"
            + "\tpreamble:\tlong\n\tWMM/WME:\tyes\n"
            + "\tMFP:\t\tno\n\ttx retries:\t12\n\ttx failed:\t1\n\tbeacon loss:\t0\n"
            + "\trx drop misc:\t5451\n\tsignal:  \t-43 [-52, -43] dBm\n\tsignal avg:\t-40 dBm\n"
            + "\tbeacon signal avg:\t-35 dBm\n"
            + "\ttx bitrate:\t1134.2 MBit/s 80MHz HE-MCS 11 HE-NSS 2 HE-GI 1 HE-DCM 0\n"
            + "\trx bitrate:\t1200.9 MBit/s 80MHz HE-MCS 11 HE-NSS 2 HE-GI 0 HE-DCM 0\n"
            + "\tDTIM period:\t2\n\tbeacon interval:100\n\tshort slot time:yes\n"
            + "\tconnected time:\t11744 seconds\n"
            + "@@ info\nInterface wlp0s20f3\n\tifindex 2\n"
            + "\tchannel 149 (5745 MHz), width: 80 MHz, center1: 5775 MHz\n\ttxpower 22.00 dBm\n";
        const networkRadio = WifiLink.radio(networkIw);
        machine("the signal at each antenna", [networkRadio.signal, networkRadio.antennas],
            [-43, [-52, -43]]);
        machine("the signal on average and of the beacons",
            [networkRadio.average, networkRadio.beacons], [-40, -35]);
        machine("what the card sends at", networkRadio.power, 22);
        machine("what was sent again, never sent, missed and dropped", [networkRadio.retries,
            networkRadio.failed, networkRadio.missed, networkRadio.dropped], [12, 1, 0, 5451]);
        machine("how long it's been on", networkRadio.connected, 11744);
        machine("its management frames unprotected", networkRadio.protected, false);
        machine("how often the beacons come, and which say what's held",
            [networkRadio.interval, networkRadio.dtim], [100, 2]);
        machine("its preamble, slot time and traffic sorted by kind",
            [networkRadio.preamble, networkRadio.shortSlot, networkRadio.wmm], ["long", true, true]);
        machine("an access point that says none of its timing",
            [WifiLink.radio("@@ station\n\tsignal: -43 dBm\n").dtim,
                WifiLink.radio("@@ station\n\tsignal: -43 dBm\n").wmm], [null, null]);
        machine("its channel and frequency", [networkRadio.channel, networkRadio.frequency],
            [149, 5745]);
        machine("a Wi-Fi 6 rate", networkRadio.receive,
            { mbps: 1200.9, width: 80, generation: 6, mcs: 11, streams: 2, guard: 0.8,
                dcm: false });
        machine("a Wi-Fi 6 rate's longer guard", networkRadio.send.guard, 1.6);
        machine("a Wi-Fi 7 rate", WifiLink.rate("2882.4 MBit/s 160MHz EHT-MCS 13 EHT-NSS 2")
            .generation, 7);
        machine("a Wi-Fi 4 rate's streams", WifiLink.rate("144.4 MBit/s MCS 15 short GI"),
            { mbps: 144.4, width: null, generation: 4, mcs: 7, streams: 2, guard: 0.4,
                dcm: null });
        machine("a Wi-Fi 5 rate's usual guard",
            WifiLink.rate("866.7 MBit/s 80MHz VHT-MCS 9 VHT-NSS 2").guard, 0.8);
        machine("an older rate", WifiLink.rate("54.0 MBit/s"), { mbps: 54, width: null,
            generation: null, mcs: null, streams: null, guard: null, dcm: null });
        machine("a card on no network", WifiLink.radio("@@ link\nNot connected.\n@@ station\n"),
            null);
        machine("how long, in hours and minutes", Interfaces.lasted(11744), "3 h 15 min");
        machine("how long, in days", Interfaces.lasted(200000), "2 d 7 h");
        machine("how long, in whole hours", Interfaces.lasted(72000), "20 h");
        machine("how long, in whole days", Interfaces.lasted(172800), "2 d");
        machine("how long, only just", Interfaces.lasted(20), "under a minute");
        machine("a signal's height", [-90, -60, -30, -10].map(Interfaces.level), [0, 0.5, 1, 1]);
        // One way of a link, as wifid says it, iw filling in what it doesn't.
        const wifidWay = { mbps: 1081, generation: 6, mcs: 10, streams: 2, guard: 0.8,
            dcm: null, bytes: 23661070224, packets: 27210952 };
        machine("a way of a link, as wifid says it, with iw's width",
            WifiLink.way(wifidWay, networkRadio.receive), { mbps: 1081, width: 80,
                generation: 6, mcs: 10, streams: 2, guard: 0.8, dcm: false,
                bytes: 23661070224, packets: 27210952 });
        machine("a Wi-Fi 7 way, its coding as iw says it", WifiLink.way(
            { mbps: 2882, generation: null, mcs: null, streams: null, guard: null },
            WifiLink.rate("2882.4 MBit/s 160MHz EHT-MCS 13 EHT-NSS 2")), { mbps: 2882,
                width: 160, generation: 7, mcs: 13, streams: 2, guard: null, dcm: null,
                bytes: null, packets: null });
        machine("a way neither says", WifiLink.way(null, null).mcs, null);
        const mcs = value => `MCS ${value}`;
        machine("a figure the same each way, said once", WifiLink.eachWay(10, 10, mcs),
            "MCS 10");
        machine("a figure each way", WifiLink.eachWay(10, 9, mcs), "MCS 10 in \u00b7 MCS 9 out");
        machine("a figure only sent", WifiLink.eachWay(null, 9, mcs), "MCS 9 out");
        machine("a figure only heard", WifiLink.eachWay(0, undefined, mcs), "MCS 0 in");
        machine("a figure neither way", WifiLink.eachWay(null, null, mcs), "");
        const tenths = (value, decimals) => value.toFixed(decimals);
        machine("idle for milliseconds", WifiLink.idle(12, tenths), "12 ms");
        machine("idle for seconds", WifiLink.idle(2430, tenths), "2.4 s");
        machine("idle for tens of seconds", WifiLink.idle(42000, tenths), "42 s");
        machine("idle for minutes", WifiLink.idle(3 * 60000, tenths), "3 min");

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
        machine("what a jack has plugged in", Sounds.plugged([{ name: "codec", value: "CS42L45" },
            { name: "plugged in", value: "headphones" }]), "headphones");
        machine("a jack with nothing in it", Sounds.unplugged([{ name: "plugged in",
            value: "nothing" }]), true);
        machine("speakers have no jack", [Sounds.plugged([{ name: "amplifiers", value: "4" }]),
            Sounds.unplugged(undefined)], ["", false]);
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
        // A jack's "nothing" hides it only while desk-facts still runs to say
        // otherwise; once it's gone, headphones may have come since.
        const sturdyNothing = [{ name: "plugged in", value: "nothing" }];
        machine("a jack hidden only while what's said is current",
            [Sounds.hidden(sturdyNothing, true), Sounds.hidden(sturdyNothing, false),
                Sounds.hidden([{ name: "plugged in", value: "headphones" }], true)],
            [true, false, false]);
        machine("whether desk-facts still runs", [Sounds.populated("populated 1\nfrozen 0\n"),
            Sounds.populated("populated 0\nfrozen 0\n"), Sounds.populated(undefined)],
            [true, false, false]);
        // pw-dump is given a card's object id, and nothing it could take for
        // an option or a pattern.
        machine("what pw-dump is given", ["65", 65, "--monitor", "65 -m", "alsa*", "", undefined]
            .map(id => Sounds.dumpable(id)), ["65", "65", "", "", "", "", ""]);
        // Port reads, one at a time: one asked mid-read waits, and each run's
        // output is its own device's, whether it prints or ends first.
        const sturdyOut = { name: "out" };
        const sturdyIn = { name: "in" };
        let sturdyReads = Sounds.portReads(null, { asked: sturdyOut });
        machine("a port read starts", sturdyReads.start, sturdyOut);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { asked: sturdyIn });
        machine("one asked mid-read waits", sturdyReads.start, null);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { printed: "out's" });
        machine("nothing taken before it ends", sturdyReads.take, null);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { ended: true });
        machine("the first read is the first device's", sturdyReads.take,
            { node: sturdyOut, text: "out's" });
        machine("the one waiting starts after it", sturdyReads.start, sturdyIn);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { ended: true });
        machine("nothing taken before it prints", sturdyReads.take, null);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { printed: "in's" });
        machine("the second read is the second device's", sturdyReads.take,
            { node: sturdyIn, text: "in's" });
        machine("nothing waiting, nothing starts", [sturdyReads.start, sturdyReads.reads.run],
            [null, null]);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { asked: sturdyOut });
        sturdyReads = Sounds.portReads(sturdyReads.reads, { asked: sturdyIn });
        sturdyReads = Sounds.portReads(sturdyReads.reads, { printed: "half" });
        sturdyReads = Sounds.portReads(sturdyReads.reads, { ended: false });
        machine("a read stopped at its limit gives nothing, and the next starts",
            [sturdyReads.take, sturdyReads.start], [null, sturdyIn]);
        sturdyReads = Sounds.portReads(sturdyReads.reads, { gaveUp: true });
        machine("one that never ended is given up", [sturdyReads.take, sturdyReads.reads.run],
            [null, null]);

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

        // A card's ports, as pw-dump says them: one analog output with its
        // speakers and a headphone jack nothing's plugged into, and its
        // microphone, in the profile it's in.
        const soundCard = JSON.stringify([{ id: 7, type: "PipeWire:Interface:Device", info: {
            params: {
                Profile: [{ index: 1, name: "output:analog-stereo+input:analog-stereo" }],
                Route: [{ index: 3, device: 4, direction: "Output" }],
                EnumRoute: [
                    { index: 3, name: "analog-output-speaker", description: "Speakers",
                        direction: "Output", available: "unknown", devices: [4],
                        profiles: [1] },
                    { index: 4, name: "analog-output-headphones", description: "Headphones",
                        direction: "Output", available: "no", devices: [4], profiles: [1] },
                    { index: 5, name: "analog-input-mic", description: "Microphone",
                        direction: "Input", available: "unknown", devices: [5],
                        profiles: [1] },
                    { index: 6, name: "hdmi-output-0", description: "HDMI",
                        direction: "Output", available: "no", devices: [4], profiles: [2] }
                ]
            } } }]);
        machine("an output's ports, the one in use", Sounds.ports(soundCard,
            { "device.id": "7", "card.profile.device": "4" }, true), [
            { index: 3, name: "Speakers", available: "unknown", active: true },
            { index: 4, name: "Headphones", available: "no", active: false }]);
        machine("an input's, the other way", Sounds.ports(soundCard,
            { "device.id": "7", "card.profile.device": "5" }, false).map(port => port.name),
            ["Microphone"]);
        machine("another card's, none", Sounds.ports(soundCard, { "device.id": "8" }, true), []);
        machine("no dump, no ports", Sounds.ports("", { "device.id": "7" }, true), []);

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
        machine("the card says it already, or not", [
            Sounds.sameFormat("48 kHz \u00b7 32 bit",
                "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)"),
            Sounds.sameFormat("48 kHz \u00b7 32 bit float",
                "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "48 kHz \u00b7 16 bit"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "44.1 kHz \u00b7 16 bit \u00b7 period 1024"),
            Sounds.sameFormat("48 kHz \u00b7 16 bit", "")], [true, false, true, false, false]);

        // The firmware fwupd knows, as its three commands' --json say it: a
        // device it hides, one with no version and one with no name are
        // left out; the one with an update offered comes first; two of one
        // name are numbered; and when each was last updated is from the
        // history's update that went in.
        const fwupdDevices = { Devices: [
            { DeviceId: "ssd", Name: "PVC10 SK hynix 1024GB", Vendor: "SK hynix",
                Version: "61001171", Flags: ["internal", "updatable", "needs-reboot"] },
            { DeviceId: "bios", Name: "System Firmware", Vendor: "Dell", Version: "1.12.0",
                Flags: ["internal", "updatable", "needs-reboot", "require-ac"] },
            { DeviceId: "region", Name: "Internal SPI Controller (BIOS)", Version: "1.12.0",
                Flags: ["internal", "updatable-hidden"] },
            { DeviceId: "spi", Name: "Internal SPI Controller", Flags: ["internal"] },
            { DeviceId: "nameless", Version: "1" },
            { DeviceId: "pk", Name: "Platform Key", Vendor: "Unknown", Version: "2026" },
            { DeviceId: "dock1", Name: "USB-C Controller", Vendor: "HP", Version: "69.52",
                Flags: ["updatable"] },
            { DeviceId: "dock2", Name: "USB-C Controller", Vendor: "HP", Version: "69.64",
                Flags: ["updatable"] }
        ] };
        const fwupdOffered = { Devices: [{ DeviceId: "bios", Releases: [
            { Version: "1.13.0",
                Description: "<p>This release fixes:</p><ul><li>a &amp; b</li><li>c</li></ul>" },
            { Version: "1.12.5" }] }] };
        const fwupdHistory = { Devices: [
            { DeviceId: "bios", UpdateState: 2, Created: 1793234644, Modified: 1793278224 },
            { DeviceId: "ssd", UpdateState: 3, Created: 1793000000 }
        ] };
        const firmware = Firmware.devices(fwupdDevices, fwupdOffered, fwupdHistory);
        machine("the firmware listed, an update first", firmware.map(device => device.name),
            ["System Firmware", "PVC10 SK hynix 1024GB", "Platform Key", "USB-C Controller 1",
                "USB-C Controller 2"]);
        machine("the firmware with an update offered", firmware[0], {
            id: "bios", name: "System Firmware", vendor: "Dell", version: "1.12.0",
            offered: "1.13.0", notes: "This release fixes:\na & b\nc", updated: 1793278224,
            restarts: true, waits: null, charger: true
        });
        machine("firmware never updated, or not successfully", firmware[1].updated, null);
        machine("firmware's version with an update offered", Firmware.versionOf(firmware[0]),
            "1.12.0, 1.13.0 ready");
        machine("firmware's version alone", Firmware.versionOf(firmware[1]), "61001171");
        machine("one firmware update ready", Firmware.standing(firmware), "1 update ready");
        machine("firmware up to date", Firmware.standing(firmware.slice(1)), "up to date");
        machine("nothing offered, said in words", Firmware.devices(fwupdDevices, {}, {})
            .every(device => device.offered === null), true);
        machine("fwupd with no devices", Firmware.devices({}, {}, {}), []);
        machine("a vendor fwupd doesn't know is none",
            firmware.find(device => device.id === "pk").vendor, "");
        // An update in but finishing at the restart, as the history says a
        // capsule's is: offered until then, and said to wait for it; once
        // the restart has flashed it, nothing waits.
        const waiting = state => ({ Devices: [{ DeviceId: "bios", UpdateState: state,
            Releases: [{ Version: "1.13.0" }] }] });
        const waits = Firmware.devices(fwupdDevices, {}, waiting(1))[0];
        machine("firmware waiting for the restart", [waits.offered, waits.waits],
            ["1.13.0", "1.13.0"]);
        machine("firmware's version waiting for the restart", Firmware.versionOf(waits),
            "1.12.0, 1.13.0 at the restart");
        machine("firmware needing a reboot waits too",
            Firmware.devices(fwupdDevices, {}, waiting(4))[0].waits, "1.13.0");
        machine("firmware that failed doesn't wait",
            Firmware.devices(fwupdDevices, {}, waiting(3)).find(device => device.id === "bios")
                .waits, null);
        const flashed = { Devices: [Object.assign({}, fwupdDevices.Devices[1],
            { Version: "1.13.0" })] };
        machine("firmware flashed at the restart waits no more",
            Firmware.devices(flashed, {}, waiting(1))[0].waits, null);
        machine("why an install didn't go in", Firmware.reason(
            "Decompressing\u2026\nSystem Firmware cannot be updated: on battery\n\n"),
            "System Firmware cannot be updated: on battery");
        machine("an install that said nothing", Firmware.reason(""), "");
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

        // The keyboards in the kernel's list of input devices, as the
        // laptop's and an ErgoDox's read, with the hotkeys and keyd's own
        // keyboard left out, and the ErgoDox's second keyboard, its keypad's
        // on the same USB device, listed as the one board it is.
        const keyboardsListed = [
            "I: Bus=0011 Vendor=0001 Product=0001 Version=ab83",
            "N: Name=\"AT Translated Set 2 keyboard\"",
            "P: Phys=isa0060/serio0/input0",
            "S: Sysfs=/devices/platform/i8042/serio0/input/input0",
            "U: Uniq=",
            "H: Handlers=sysrq kbd event0 leds ",
            "B: EV=120013",
            "B: LED=7",
            "",
            "I: Bus=0019 Vendor=0000 Product=0006 Version=0000",
            "N: Name=\"Video Bus\"",
            "P: Phys=LNXVIDEO/video/input0",
            "S: Sysfs=/devices/pci0000:00/acpi.video_bus.0/input/input2",
            "H: Handlers=kbd event1 ",
            "",
            "I: Bus=0003 Vendor=3297 Product=4974 Version=0111",
            "N: Name=\"ZSA Technology Labs ErgoDox EZ\"",
            "P: Phys=usb-0000:00:14.0-3.4/input0",
            "S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4/3-3.4:1.0/"
                + "0003:3297:4974.0001/input/input5",
            "U: Uniq=",
            "H: Handlers=sysrq kbd leds event4 ",
            "B: LED=1f",
            "",
            "I: Bus=0003 Vendor=3297 Product=4974 Version=0111",
            "N: Name=\"ZSA Technology Labs ErgoDox EZ Keyboard\"",
            "P: Phys=usb-0000:00:14.0-3.4/input2",
            "S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb3/3-3/3-3.4/3-3.4:1.2/"
                + "0003:3297:4974.0003/input/input8",
            "U: Uniq=",
            "H: Handlers=sysrq kbd leds event7 ",
            "B: LED=1f",
            "",
            "I: Bus=0003 Vendor=0fac Product=0ade Version=0000",
            "N: Name=\"keyd virtual keyboard\"",
            "S: Sysfs=/devices/virtual/input/input42",
            "H: Handlers=sysrq kbd event12 leds rfkill ",
            "B: LED=1ff",
            ""
        ].join("\n");
        const keyboardsFound = Keyboards.keyboards(keyboardsListed);
        machine("the keyboards", keyboardsFound.map(device => device.name),
            ["AT Translated Set 2 keyboard", "ZSA Technology Labs ErgoDox EZ"]);
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

        // What key-counts tallied, as its file says it: the board's keys
        // on two layers, some under modifiers, the two shifts one key, and
        // the laptop's apart.
        const counts = "board\tlayer\tmodifiers\tkey\tpresses\tholds\n"
            + "ergodox\tcoal\t-\te\t9100\t0\n"
            + "ergodox\tcoal\t-\tspace\t5000\t40\n"
            + "ergodox\tcoal\tctrl\ta\t900\t0\n"
            + "ergodox\tnumb\t-\tkp1\t800\t0\n"
            + "ergodox\tcoal\t-\tleftshift\t2000\t2000\n"
            + "ergodox\tcoal\t-\trightshift\t1200\t1200\n"
            + "ergodox\tnumb\tshift\tcode193\t1000\t0\n"
            + "laptop\tbase\t-\tj\t70\t0\n";
        // The theme's faces, by the key map's spelling, as Shortcuts.qml
        // gives them (modules/theme/keys.nix).
        const keyFaces = { shift: { text: "\u21e7" }, f23: { text: "Copilot", wide: false } };
        const tally = KeyCounts.board(counts, "ergodox", keyFaces);
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
        machine("a board with nothing counted", [KeyCounts.board(counts, "other", keyFaces),
            KeyCounts.facts(null, keyNamed, ""), KeyCounts.most(null, keyNamed)], [null, [], []]);
        machine("counts short", [870, 9100, 12400].map(KeyCounts.short), ["870", "9.1k", "12k"]);
        machine("counting began", KeyCounts.since("1793898549\n", keyWords,
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
        const bareBoard = Keyboards.drawer({ device: keyboardsErgodox, name: "", layer: null,
            locks: [{ id: "caps", on: false }], connection: "USB" }, keyNamed, boardSaid);
        machine("no lock on, and a bus the eyebrow says", bareBoard.facts.map(fact =>
            `${fact.label} ${fact.value}`), ["[locks] none"]);
        machine("the laptop's eyebrow", Keyboards.drawer({ device: keyboardsLaptop, name: "",
            layer: null }, keyNamed, boardSaid).eyebrow, "built in \u00b7 keyboard");

        // The battery's and the room's figures, from their files as the
        // kernel writes them.
        machine("the charge type in use", Power.chosen("Trickle Fast Standard Adaptive [Custom]"),
            { now: "Custom", all: ["Trickle", "Fast", "Standard", "Adaptive", "Custom"] });
        machine("an older kernel's one charge type", Power.chosen("Fast").now, "Fast");
        machine("no charge type", Power.chosen(""), { now: "", all: [] });
        machine("power delivery", Power.protocol("C [PD] PD_PPS"), "USB PD");
        machine("programmable power", Power.protocol("C PD [PD_PPS]"), "USB PD PPS");
        machine("Type-C's own current", Power.protocol("[C] PD PD_PPS"), "USB-C");
        machine("a plain USB port", Power.protocol("[SDP] DCP CDP"), "USB");
        const powerPort = { online: true, volts: 20, amps: 5, protocol: "USB PD" };
        machine("a port charging", Power.port(powerPort, number), "100 W, 20 V at 5.0 A, USB PD");
        machine("a port charging, its figures unsaid",
            Power.port({ online: true, volts: null, amps: null, protocol: "" }, number),
            "charging");
        machine("a port powering a phone", Power.port({ online: false, giving: true }, number),
            "powering what's plugged in");
        machine("a port with something in", Power.port({ online: false, partner: true }, number),
            "plugged in, giving nothing");
        machine("an empty port", Power.port({ online: false }, number), "empty");
        machine("made on a day", Power.made(2026, 7, 20), "2026-07-20");
        machine("made in a month", Power.made(2026, 7, 0), "2026-07");
        machine("a date that isn't", Power.made(0, 13, 40), "");
        machine("a limit raised moves its stop first", Power.limitWrites(75, 80, 90, 5),
            [["charge_control_end_threshold", "90"], ["charge_control_start_threshold", "85"]]);
        machine("a limit lowered moves its start first", Power.limitWrites(75, 80, 60, 5),
            [["charge_control_start_threshold", "55"], ["charge_control_end_threshold", "60"]]);
        machine("a limit with no start", Power.limitWrites(null, 80, 90, 5),
            [["charge_control_end_threshold", "90"]]);
        machine("the room's lux", Power.lux("20000", "0.001000000", "0"), 20);
        machine("a sensor with no scale", Power.lux("17", "", ""), 17);
        machine("a sensor not read", Power.lux("", "0.001", "0"), null);
        machine("a dark room", Power.roomLevel(0), 0);
        machine("daylight", Power.roomLevel(10000), 1);
        machine("a room a hundredth of daylight", Math.round(Power.roomLevel(100) * 100) / 100,
            0.5);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
