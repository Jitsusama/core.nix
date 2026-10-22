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

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
