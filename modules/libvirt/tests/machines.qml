// What virsh says of the VMs, as Programs › VMs shows it, run in Qt's own
// JavaScript engine, the one the shell runs it in: a list of names or
// UUIDs read one a line, blank lines left out; dominfo read for every
// figure it gives, a running machine's and one shut off, one with no limit
// on its memory and one with messages and a security label, several in one
// batch, one virsh couldn't find left out and a label it doesn't know
// passed over, a name's own spaces kept; each state virsh says read as its
// word, every word words.json having one, and one it doesn't know as
// unknown; what can be done in each state, in order, and what virsh is
// told for each; the figures a drawer shows in their order and units, a
// figure dominfo left out left out; the order and count Programs shows;
// an event read as a change; and virsh's error read as the foot says it.
// The `vms-read-as-virsh-says` check copies Machines.js and the launcher's
// words beside this file and runs `qml -platform offscreen machines.qml`.
import QtQuick
import "Machines.js" as Machines

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

    // The launcher's own words, as the check copies them beside this.
    function words() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const all = JSON.parse(request.responseText);
        return all.things.find(thing => thing.id === "vm");
    }

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const vm = words();
        const said = vm.said;

        check("a list, one a line", Machines.listed("web\ndb\n\n"), ["web", "db"]);
        check("an empty list", Machines.listed("\n"), []);
        check("nothing at all", Machines.listed(""), []);
        check("a list of UUIDs",
            Machines.listed("3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10\n\n"),
            ["3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10"]);

        // As virsh 12.7 writes them, with LC_ALL=C.
        const running = [
            "Id:             3",
            "Name:           web",
            "UUID:           3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10",
            "OS Type:        hvm",
            "State:          running",
            "CPU(s):         4",
            "CPU time:       754.2s",
            "Max memory:     4194304 KiB",
            "Used memory:    2097152 KiB",
            "Persistent:     yes",
            "Autostart:      disable",
            "Managed save:   no",
            "Security model: apparmor",
            "Security DOI:   0",
            "Security label: libvirt-3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10 (enforcing)",
            "Messages:       tainted: custom-monitor",
            "                tainted: host-cpu",
            ""
        ].join("\n");
        const off = [
            "Id:             -",
            "Name:           build box",
            "UUID:           9b2d6c1e-0f4a-4a55-8b0e-6d2f1c3a7e44",
            "OS Type:        hvm",
            "State:          shut off",
            "CPU(s):         2",
            "Max memory:     no limit",
            "Used memory:    524288 KiB",
            "Wanted later:   a label no libvirt here says",
            "Persistent:     yes",
            "Autostart:      enable",
            "Managed save:   unknown",
            "Security model: none",
            "Security DOI:   0",
            ""
        ].join("\n");
        const [web] = Machines.described(running);
        check("its Id", web.id, 3);
        check("its name", web.name, "web");
        check("its UUID", web.uuid, "3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10");
        check("its OS type", web.osType, "hvm");
        check("its state", web.state, "running");
        check("its processors", web.processors, 4);
        check("its processor time", web.processorTime, 754.2);
        check("its most memory", [web.mostMemory, web.noLimit], [4194304, false]);
        check("its memory in use", web.memory, 2097152);
        check("whether it's persistent", web.persistent, true);
        check("whether it starts by itself", web.autostart, false);
        check("whether it has a managed save", web.managedSave, false);
        check("its security", [web.securityModel, web.securityDoi, web.securityLabel],
            ["apparmor", "0",
                "libvirt-3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10 (enforcing)"]);
        check("its messages, a colon in each", web.messages,
            ["tainted: custom-monitor", "tainted: host-cpu"]);

        const [box] = Machines.described(off);
        check("no Id while it's off", box.id, null);
        check("a name's own space", box.name, "build box");
        check("no processor time while it's off", box.processorTime, null);
        check("no limit on its memory", [box.mostMemory, box.noLimit], [null, true]);
        check("an unknown managed save", box.managedSave, null);
        check("a label it doesn't know passed over, the rest still read",
            [box.persistent, box.autostart, box.securityModel], [true, true, "none"]);
        check("a name's spaces kept whole",
            Machines.described("Id:             -\nName:            spaced \nUUID:           "
                + "a\n")[0].name, " spaced ");

        const both = Machines.described(`${running}${off}`);
        check("two in one batch", both.map(one => one.name), ["web", "build box"]);
        check("a batch with blank lines between",
            Machines.described(`${running}\n\n${off}`).length, 2);
        check("nothing described", Machines.described(""), []);
        check("a machine with no UUID left out", Machines.described("Id: -\nName: x\n"), []);

        const states = {
            "running": "running",
            "idle": "blocked",
            "paused": "paused",
            "in shutdown": "stopping",
            "shut off": "off",
            "crashed": "crashed",
            "pmsuspended": "asleep",
            "no state": "unknown",
            "": "unknown"
        };
        for (const [virsh, state] of Object.entries(states)) {
            check(`${virsh || "nothing"}'s state`, Machines.stateOf(virsh), state);
            const word = Machines.side(state, said);
            check(`${state} has a word`, typeof word === "string" && word !== "", true);
        }
        check("running's word", Machines.side("running", said), "running");
        check("paused's word", Machines.side("paused", said), "paused");
        check("shut off's word", Machines.side("off", said), "shut off");
        check("crashed's word", Machines.side("crashed", said), "crashed");
        check("a state with no word says unknown's",
            Machines.side("elsewhere", said), said.unknown);

        check("running's acts", Machines.acts("running"), ["console", "power-off", "force-off"]);
        check("blocked's acts", Machines.acts("blocked"), ["console", "power-off", "force-off"]);
        check("paused's acts", Machines.acts("paused"), ["console", "resume", "force-off"]);
        check("powering off's acts", Machines.acts("stopping"), ["console", "force-off"]);
        check("shut off's acts", Machines.acts("off"), ["start"]);
        check("crashed's acts", Machines.acts("crashed"), ["console", "force-off"]);
        check("asleep's acts", Machines.acts("asleep"), ["console", "force-off"]);
        check("an unknown state's acts", Machines.acts("unknown"), []);
        const uuid = web.uuid;
        check("start told", Machines.told("start", uuid), ["start", "--domain", uuid]);
        check("resume told", Machines.told("resume", uuid), ["resume", "--domain", uuid]);
        check("pause told, to take back resume", Machines.told("pause", uuid),
            ["suspend", "--domain", uuid]);
        check("power off told", Machines.told("power-off", uuid), ["shutdown", "--domain", uuid]);
        check("force off told", Machines.told("force-off", uuid), ["destroy", "--domain", uuid]);
        check("the console is no virsh act", Machines.told("console", uuid), []);
        check("what start becomes", Machines.becomes("start"), "running");
        check("what force off becomes", Machines.becomes("force-off"), "off");
        check("what pausing becomes", Machines.becomes("pause"), "paused");
        check("power off foretells nothing", Machines.becomes("power-off"), "");

        const kept = "\u00a0";
        check("a running machine's figures", Machines.figures(web, said), [
            { id: "processors", value: "4" },
            { id: "memory", value: `2${kept}GiB` },
            { id: "most-memory", value: `4${kept}GiB` },
            { id: "processor-time", value: `12${kept}min 34${kept}s` },
            { id: "autostart", value: said.no },
            { id: "persistent", value: said.yes },
            { id: "managed-save", value: said.no },
            { id: "os-type", value: "hvm" },
            { id: "id", value: "3" },
            { id: "uuid", value: uuid },
            { id: "security-model", value: "apparmor" },
            { id: "security-doi", value: "0" },
            { id: "security-label", value: web.securityLabel },
            { id: "messages", value: "tainted: custom-monitor; tainted: host-cpu" }
        ]);
        check("one shut off leaves out what it gave none of",
            Machines.figures(box, said).map(one => one.id),
            ["processors", "memory", "most-memory", "autostart", "persistent", "os-type",
                "uuid", "security-model", "security-doi"]);
        check("no limit said", Machines.figures(box, said)[2].value, said.noLimit);
        check("each figure has a name", Machines.figures(web, said)
            .filter(one => !vm.parts.includes(`vm:${one.id}`)), []);
        check("half a GiB", Machines.memory(524288), `512${kept}MiB`);
        check("a GiB and a half", Machines.memory(1572864), `1.5${kept}GiB`);
        check("under a MiB", Machines.memory(512), `512${kept}KiB`);
        check("seconds", Machines.duration(12.34), `12.3${kept}s`);
        check("whole seconds", Machines.duration(5), `5${kept}s`);
        check("an hour on the dot", Machines.duration(3600), `1${kept}h`);
        check("hours and minutes", Machines.duration(11_100.5), `3${kept}h 5${kept}min`);
        check("days and hours", Machines.duration(187_200), `2${kept}d 4${kept}h`);

        const listedAs = (name, state, system) => ({ name, state, system });
        check("on before off, each A to Z, session first", Machines.ordered([
            listedAs("web", "shut off", false),
            listedAs("db", "running", true),
            listedAs("db", "running", false),
            listedAs("Build", "paused", false),
            listedAs("app 10", "shut off", false),
            listedAs("app 9", "shut off", false)
        ]).map(one => `${one.name}${one.system ? " (system)" : ""}`),
        ["Build", "db", "db (system)", "app 9", "app 10", "web"]);
        check("the count by state", Machines.tally([
            listedAs("a", "shut off"), listedAs("b", "running"), listedAs("c", "shut off")
        ], said), `1 running \u00b7 2 shut off`);
        check("no machines, no count", Machines.tally([], said), "");

        check("a change heard",
            Machines.changed("event 'lifecycle' for domain 'web': Started Booted"), true);
        check("one defined heard",
            Machines.changed("event 'lifecycle' for domain 'new': Defined Added"), true);
        check("anything else not", Machines.changed("events received: 0"), false);

        check("virsh's error, as the foot says it",
            Machines.reason("error: Failed to start domain 'web'\n"
                + "error: Requested operation is not valid: domain is already active\n"),
            "requested operation is not valid: domain is already active");
        check("no error, nothing said", Machines.reason("Domain 'web' started\n"), "");

        if (failures.length > 0) {
            console.warn(failures.join("\n"));
            Qt.exit(1);
            return;
        }
        console.info(`${asked} cases hold`);
        Qt.exit(0);
    }
}
