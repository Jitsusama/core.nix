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
        check("and taken back once it's charged", low.withdraws, true);
        check("and says its charge and what to do",
            [low.value, low.hint, low.context], ["18%", "18% left", "charge it soon"]);
        check("critical, it says to charge it now", critical.context, "charge it now");
        check("in Now it's its own row in Bluetooth", [low.place, critical.place],
            ["bluetooth", "bluetooth"]);

        check("its mark is its own kind's glyph",
            Devices.low([device("aa", 0.1, { icon: "input-keyboard" })], { aa: "critical" })
                .map(thing => thing.markGlyph), ["keyboard"]);
        check("and Bluetooth's for a kind it doesn't know",
            [Devices.kind("audio-headset"), Devices.kind("something-else")],
            ["headset", "bluetooth"]);

        // What bluetooth-links says has stopped answering, as its
        // stopped.json says it.
        const intel = { name: "hci0", device: "0000:00:14.7",
            driver: "/sys/bus/pci/drivers/btintel_pcie" };
        const file = adapters => JSON.stringify({ adapters: adapters });
        check("the reader's file, read", Devices.halted(file([intel])), [intel]);
        check("no reader, or a file not its own, says nothing stopped",
            [Devices.halted(""), Devices.halted("{}")], [[], []]);
        check("the line to type unbinds its driver and binds it back", Devices.restart(intel),
            "echo 0000:00:14.7 | sudo tee /sys/bus/pci/drivers/btintel_pcie/unbind; "
            + "echo 0000:00:14.7 | sudo tee /sys/bus/pci/drivers/btintel_pcie/bind");
        check("a USB adapter's by its interface", Devices.restart({ name: "hci1",
            device: "1-10:1.0", driver: "/sys/bus/usb/drivers/btusb" }),
            "echo 1-10:1.0 | sudo tee /sys/bus/usb/drivers/btusb/unbind; "
            + "echo 1-10:1.0 | sudo tee /sys/bus/usb/drivers/btusb/bind");
        check("none where the reader couldn't say what it sits on",
            Devices.restart({ name: "hci0", device: null, driver: null }), "");
        check("none for a device or driver that isn't sysfs's, since it's typed as root", [
            Devices.restart(Object.assign({}, intel, { device: "x; rm -rf /" })),
            Devices.restart(Object.assign({}, intel, { driver: "/tmp/drivers/x" })),
            Devices.restart(Object.assign({}, intel, { driver: "/sys/bus/pci/drivers/../x" }))
        ], ["", "", ""]);

        const [dead] = Devices.stopped([intel], "hci0");
        check("the adapter stopped is the Adapter's own row in Bluetooth",
            [dead.key, dead.place], ["adapter", "bluetooth"]);
        check("it wants Joel, said as it comes and taken back once it answers",
            [dead.active, dead.wants, dead.arrives, dead.withdraws], [true, true, true, true]);
        check("its notice says it stopped answering, plainly",
            [dead.app, dead.hint, dead.value], ["Bluetooth", "Bluetooth stopped answering",
                "not answering"]);
        check("and gives the line to type, and that the mouse may need pairing again",
            dead.message, "Type this in a terminal to start it again:\n"
                + `${Devices.restart(intel)}\n`
                + "Afterwards, your mouse may need pairing again.");
        check("which it holds for its row to copy", dead.line, Devices.restart(intel));
        check("where there's no line, restarting the machine is what's left",
            Devices.stopped([{ name: "hci0" }], "hci0")[0].message,
            "Restarting the machine starts it again. "
                + "Afterwards, your mouse may need pairing again.");
        check("another adapter is a fact of its own, with no row",
            Devices.stopped([Object.assign({}, intel, { name: "hci1" })], "hci0")
                .map(thing => [thing.key, thing.place]), [["adapter:hci1", ""]]);
        check("nothing stopped, nothing said", Devices.stopped([], "hci0"), []);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
