// What the launcher's answers must make of what's typed, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-launcher-answers-what-is-asked` check copies Numbers.js and
// Addresses.js beside this file and runs `qml -platform offscreen
// answers.qml`. Each case names what's typed and what has to come back,
// or that nothing should.
import QtQuick
import "Numbers.js" as Numbers
import "Addresses.js" as Addresses
import "Units.js" as Units
import "Zones.js" as Zones
import "Colours.js" as Colours
import "Asking.js" as Asking
import "Machine.js" as Machine
import "Links.js" as Links
import "Standing.js" as Standing
import "Traffic.js" as Traffic
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
        "1/0": "\u221e"
    })
    readonly property var notSums: ["2", "-5", "pi", "kitty", "ws 3", "1password", "12*",
        "(3", "sqrt", "3.5 in hex", "2fa", "e"]

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
        "1 atm in psi": "14.6959 psi"
    })
    readonly property var notConversions: ["5 km in kg", "5 km in km", "100 usd in cad",
        "255 in hex", "km in mi", "5 parsecs in km", "kitty", "5 km"]

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
            const got = Numbers.answer(typed);
            if (!got || got.headline !== sums[typed])
                failures.push(`"${typed}" answered ${quoted(got && got.headline)}, `
                    + `not "${sums[typed]}"`);
        }
        for (const typed of notSums) {
            asked++;
            const got = Numbers.answer(typed);
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
        const bases = Numbers.answer("0x1f");
        asked++;
        if (!bases || quoted(bases.rows.map(r => r.text)) !== quoted(["0x1f", "0o37", "0b11111"]))
            failures.push(`"0x1f" gave the rows ${quoted(bases && bases.rows)}`);
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

        for (const typed in conversions) {
            asked++;
            const got = Units.answer(typed);
            if (!got || got.headline !== conversions[typed])
                failures.push(`"${typed}" converted to ${quoted(got && got.headline)}, `
                    + `not "${conversions[typed]}"`);
        }
        for (const typed of notConversions) {
            asked++;
            const got = Units.answer(typed);
            if (got)
                failures.push(`"${typed}" converted to ${quoted(got.headline)}, `
                    + "but nothing should");
        }
        const rated = Units.answer("5 km in mi");
        asked++;
        if (!rated || quoted(rated.rows.map(r => r.text))
                !== quoted(["1 km = 0.621371 mi", "1 mi = 1.60934 km"]))
            failures.push(`"5 km in mi" gave the rows ${quoted(rated && rated.rows)}`);

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
        const lines = answer => answer.rows.map(r => [r.text, r.side].filter(s => s).join(" "));
        if (!told || quoted(lines(told)) !== quoted(["Friday 16 January tomorrow",
                "18 h ahead of here", "Pacific/Auckland NZDT \u00b7 UTC+13"]))
            failures.push(`"time in auckland" gave the rows ${quoted(told && told.rows)}`);
        // Here, the day needs no "today", and UTC is said once.
        const cases = [
            ["time", ["Thursday 15 January", "America/Toronto EST \u00b7 UTC\u22125"]],
            ["time in utc", ["Thursday 15 January today", "5 h ahead of here", "UTC"]],
            ["time in kathmandu", ["Thursday 15 January today", "10 h 45 min ahead of here",
                "Asia/Kathmandu UTC+5:45"]]
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

        for (const [typed, headline, rows] of ColourCases.answers) {
            asked++;
            const got = Colours.answer(typed, ColourCases.palette, ColourCases.roles);
            const said = got && got.rows.map(r => [r.text, r.side].filter(s => s).join(" "));
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
        const before = Machine.counters("cpu  100 0 100 700 100 0 0 0 50 0\n"
            + "cpu0 50 0 50 400 0 0 0 0 0 0\ncpu1 50 0 50 300 100 0 0 0 0 0\nintr 5\n");
        const after = Machine.counters("cpu  200 0 200 800 100 0 0 0 90 0\n"
            + "cpu1 50 0 50 400 100 0 0 0 0 0\ncpu0 150 0 150 400 0 0 0 0 0 0\n");
        // Guest time is counted in user time already, and waiting on the
        // disk is idle.
        machine("the processors since they started", Machine.busy(before.whole, null), 0.2);
        machine("the processors between two reads", Machine.busy(after.whole, before.whole), 2 / 3);
        // Core 0 first, however the kernel lists them.
        machine("each core between two reads", Machine.loads(after, before),
            [{ n: 0, load: 1 }, { n: 1, load: 0 }]);
        const cores = loads => loads.map((load, n) => ({ n: n, load: load }));
        machine("no core maxed", Machine.maxed(cores([0.5, 0.94])), "none");
        machine("one core maxed", Machine.maxed(cores([0.5, 0.95])), "core 1");
        machine("three cores maxed", Machine.maxed(cores([1, 0.2, 0.99, 0.97])), "cores 0, 2 and 3");
        machine("most cores maxed", Machine.maxed(cores([1, 1, 1, 1, 1, 0])), "5 of 6");
        machine("a list of CPUs", Machine.cpus("0-3,8\n"), [0, 1, 2, 3, 8]);
        machine("no list of CPUs", Machine.cpus(""), []);
        // The laptop's: performance cores, then efficient, then low-power,
        // told apart by their most clock, the third performance core the
        // fastest of all.
        const clocked = [4.7, 4.7, 4.7, 4.8, 3.7, 3.7, 3.7, 3.7, 3.7, 3.7, 3.7, 3.7, 3.3, 3.3, 3.3, 3.3]
            .map((ghz, n) => ({ n: n, max: ghz * 1e6 }));
        const kinded = Machine.kinds(Machine.cpus("0-3"), clocked);
        machine("the laptop's kinds of core", ["performance", "efficient", "low-power"]
            .map(kind => Object.values(kinded).filter(each => each === kind).length), [4, 8, 4]);
        machine("a processor of one kind", Machine.kinds([], clocked), null);
        machine("a processor's model", Machine.model("Intel(R) Core(TM) Ultra X7 358H"),
            "Intel Core Ultra X7 358H");
        machine("an older processor's model", Machine.model("Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz"),
            "Intel Core i7-8550U");
        // Reads, sectors, their milliseconds; writes, the same; in flight;
        // busy.
        const disk = Machine.disk("10 0 2000 20 30 0 4000 60 0 100 0");
        const later = Machine.disk("20 0 4000 30 40 0 6048 80 1 600 0");
        machine("a disk between two reads", Machine.rates(later, disk, 2),
            { read: 512000, write: 524288, busy: 0.25, latency: 1.5 });
        machine("a disk left idle", Machine.rates(disk, disk, 1).latency, null);
        machine("what isn't a disk's counters", Machine.disk(""), null);
        for (const [name, known] of [["kitty", "kitty"], [".kitty-wrapped", "kitty"],
                [".firefox-wrappe", "firefox"], [".rust-analyzer-", "rust-analyzer"],
                [".hidden", "hidden"]])
            machine(`the program run as ${name}`, Machine.program(name), known);
        const stats = (ticks, lines) => `4096\ncpu  ${ticks} 0 0 0 0 0 0 0 0 0\nintr 1\n${lines.join("\n")}\n`;
        const fields = (time, pages) => `S 1 1 1 0 -1 0 0 0 0 0 ${time} 0 0 0 20 0 1 0 5 100 ${pages}`;
        const first = Machine.processes(stats(1000, [`1 (systemd) ${fields(10, 100)}`,
            `2 (.kitty-wrapped) ${fields(100, 1000)}`, `3 (kitty) ${fields(50, 1000)}`,
            `4 (cc1 (odd) name) ${fields(0, 10)}`]));
        const second = Machine.processes(stats(2000, [`1 (systemd) ${fields(10, 100)}`,
            `2 (.kitty-wrapped) ${fields(300, 1000)}`, `3 (kitty) ${fields(150, 1000)}`,
            `4 (cc1 (odd) name) ${fields(400, 10)}`, `5 (cat) ${fields(900, 10)}`]));
        // Each program's processes summed, one started between the reads
        // left out, and one at nothing left out.
        machine("the programs using the processors", Machine.using(second, first),
            [{ name: "cc1 (odd) name", value: 0.4 }, { name: "kitty", value: 0.3 }]);
        machine("the programs holding the memory", Machine.holding(first),
            [{ name: "kitty", value: 8192000 }, { name: "systemd", value: 409600 }, { name: "cc1 (odd) name", value: 40960 }]);
        machine("memory pressure", Machine.pressure("some avg10=12.50 avg60=1.48 avg300=2.33 total=26\n"
            + "full avg10=0.39 avg60=1.28 avg300=1.78 total=16\n"), 0.125);
        machine("no memory pressure said", Machine.pressure(""), null);
        machine("memory's fields", Machine.meminfo("MemTotal:       65000000 kB\nSwapFree:  0 kB\n"),
            { MemTotal: 65000000, SwapFree: 0 });
        const module = { locator: "Controller0-ChannelA", size: "8 GB", type: "LPDDR5X", speed: "7467 MT/s",
            manufacturer: "SK Hynix", part: "" };
        machine("the memory modules", Machine.modules(JSON.stringify({ modules: [module, module] })),
            { type: "LPDDR5X", speed: "7467 MT/s", size: "8 GB", maker: "SK Hynix", count: 2 });
        machine("modules of two speeds", Machine.modules(JSON.stringify({
            modules: [module, Object.assign({}, module, { speed: "6400 MT/s" })] })).speed, "");
        machine("no memory modules listed", Machine.modules(JSON.stringify({ modules: [] })), null);
        machine("no memory modules file", Machine.modules(""), null);
        machine("modules the firmware says nothing of", Machine.modules(JSON.stringify({
            modules: [Object.assign({}, module, { type: "" })] })), null);
        machine("a spark against its most", Machine.spark([1, 2, 4], 1), [0.25, 0.5, 1]);
        machine("a spark against the least it's drawn to", Machine.spark([1, 2], 4), [0.25, 0.5]);
        // As Heat says the processor: its package, else its hottest core.
        const sensors = [{ label: "Core 0", value: 60000 }, { label: "Package id 0", value: 55000 },
            { label: "Core 8", value: 70000 }];
        machine("the processor's package", Machine.processor(sensors).label, "Package id 0");
        machine("the processor's hottest core", Machine.processor(sensors.filter(s => s.label !== "Package id 0")).label,
            "Core 8");
        machine("a processor's only sensor", Machine.processor([{ label: "", value: 1 }]).value, 1);

        // How things stand in the foot, as room runs short: the least of
        // each, then each its glyph by rank, then each gives way, each a
        // character wide here, two between.
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
            [12, "17:30  B  N"],
            // The time never gives way.
            [3, "17:30"]
        ])
            machine(`how things stand in ${room}`, footFor(room), said);

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
        machine("a Wi-Fi card", Traffic.kind("wlp0s20f3", true), "wifi");
        machine("a cable's card", Traffic.kind("enp1s0", true), "cable");
        machine("a tunnel", Traffic.kind("tailscale0", false), "virtual");
        machine("the loopback, never listed", Traffic.kind("lo", false), "");
        const cardKinds = { lo: "", wlp0s20f3: "wifi", tailscale0: "virtual", shaper2: "" };
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
            channels: "2, stereo" });
        machine("what a call's codec carries", Links.carried({
            "api.bluez5.profile": "headset-head-unit", "audio.rate": 16000,
            "audio.channels": 1 }, 0), { profile: "calls", rate: "16 kHz", channels: "1, mono" });
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
            traffic: [2.5e6] }), { side: "2.5 MB/s", values: [1],
            note: "2.5 MB/s in, 0 B/s out" });

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
