// The Bluetooth devices running low, run in Qt's own JavaScript engine,
// the one the shell runs it in, with plain objects standing in for BlueZ's
// devices. The `a-low-device-is-its-own-notice` check copies
// modules/quickshell/Devices.js and Crossing.js beside this file and runs
// `qml -platform offscreen devices.qml`.
import QtQuick
import "Devices.js" as Devices

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function device(address, battery, more) {
        return Object.assign({
            address: address,
            name: `Device ${address}`,
            icon: "audio-headphones",
            connected: true,
            batteryAvailable: true,
            battery: battery
        }, more ?? {});
    }

    function run() {
        const failures = [];
        const check = (what, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const marks = { low: 20, critical: 10 };
        const keys = (devices, standing) => Devices.low(devices, standing).map(thing => thing.key);

        const several = [device("aa", 0.18), device("bb", 0.5), device("cc", 0.08)];
        const standing = Devices.standings(several, {}, marks);
        check("each connected device with a battery stands by its own charge", standing,
            { aa: "low", bb: "", cc: "critical" });
        check("each running low is a thing of its own, keyed by its address",
            keys(several, standing), ["aa", "cc"]);

        check("one disconnected stands for nothing",
            Devices.standings([device("aa", 0.18, { connected: false })], {}, marks), {});
        check("nor does one that reports no battery",
            Devices.standings([device("aa", 0, { batteryAvailable: false })], {}, marks), {});
        check("one gone is forgotten",
            Devices.standings([device("bb", 0.5)], { aa: "critical", bb: "" }, marks), { bb: "" });

        const later = Devices.standings([device("aa", 0.11)], { aa: "critical" }, marks);
        check("one critical rising short of the re-arm stays critical", later, { aa: "critical" });

        const [low, critical] = Devices.low(several, standing);
        check("a low one wants Joel, in the warning colour, but not now",
            [low.wants, low.role, low.now], [true, "warning", false]);
        check("a critical one wants him now, in the alert colour",
            [critical.wants, critical.role, critical.now], [true, "alert", true]);
        check("it's said as it comes true, signed by the device's name",
            [low.arrives, low.app, low.title], [true, "Device aa", "Device aa"]);
        check("and says its charge and what to do",
            [low.value, low.hint, low.context], ["18%", "18% left", "charge it soon"]);
        check("critical, it says to charge it now", critical.context, "charge it now");

        check("its mark is its own kind's glyph",
            Devices.low([device("aa", 0.1, { icon: "input-keyboard" })], { aa: "critical" })
                .map(thing => thing.markGlyph), ["keyboard"]);
        check("and Bluetooth's for a kind it doesn't know",
            [Devices.kind("audio-headset"), Devices.kind("something-else")],
            ["headset", "bluetooth"]);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
