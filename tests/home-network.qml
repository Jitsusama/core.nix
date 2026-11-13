// The figures the launcher says a network's rates and sizes in
// (launcher/HomeNetwork.js), which the Home Network, the Work Network,
// Machine and the virtual machines share, and the Home Network's holding
// at home, run in Qt's own JavaScript engine, the one the shell runs it
// in, over the record the eero daemon's parsers are tested against
// (modules/eero/home-network.json). The
// `the-home-network-says-each-figure` check copies HomeNetwork.js and what
// it imports into launcher/, as the shell lays them out, and the record
// beside this file, and runs `qml home-network.qml`. The place's rows and
// every drawer in it are the eero daemon's, held by its own tests
// (modules/eero's rows_tests.rs and drawers/).
import QtQuick
import "launcher/HomeNetwork.js" as Home

QtObject {
    function read(path) {
        const request = new XMLHttpRequest();
        request.open("GET", path, false);
        request.send();
        return JSON.parse(request.responseText);
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            const said = JSON.stringify(got);
            if (said !== JSON.stringify(wanted))
                failures.push(`${what} gave ${said}, not ${JSON.stringify(wanted)}`);
        };
        const model = read("home-network.json");

        // The figures, each in its own unit, said one way, to three
        // significant digits (VKCF3Z, 2026-11-07).
        expect("a rate past a thousand Mb/s", Home.rate(1200.9), "1.20 Gb/s");
        expect("a rate to three digits", Home.rate(369.86), "370 Mb/s");
        expect("a rate's zero kept", Home.rate(48.0), "48.0 Mb/s");
        expect("a rate that rounds up a unit", Home.rate(999.96), "1.00 Gb/s");
        expect("a speed in Mb/s", Home.rate(11.452299), "11.5 Mb/s");
        expect("a slow speed", Home.rate(0.0239604), "24.0 kb/s");
        expect("a crawl", Home.rate(0.0000982), "98.2 b/s");
        expect("a link speed", Home.rate(1000), "1.00 Gb/s");
        expect("an unknown rate", Home.rate(null), "");
        expect("bytes", Home.bytes(153124674), "153 MB");
        expect("few bytes", Home.bytes(53741), "53.7 kB");
        expect("bytes under ten", Home.bytes(2400000), "2.40 MB");
        expect("no bytes", Home.bytes(0), "0 B");
        expect("an unknown size", Home.bytes(null), "");
        expect("a figure grouped and pointed as the settings say",
            Home.significant(1234.5, { group: " ", point: "," }), "1 230");
        expect("a count of one", Home.counted(1, "node"), "1 node");

        // The place's holding, as its row says it at home.
        expect("the place's holding", Home.holding(model), "3 nodes \u00b7 15 devices");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
