// What virsh says of the VMs, as Programs › VMs shows it, run in Qt's own
// JavaScript engine, the one the shell runs it in: a list of names read
// one a line, blank lines left out and a name's own spaces kept; the one
// batch asking dominfo of them all, every name read back whole by virsh's
// command line however it's spelled; dominfo read for every
// figure it gives, a running machine's and one shut off, one with no limit
// on its memory and one with messages and a security label, several in one
// batch, one virsh couldn't find left out and a label it doesn't know
// passed over, a name's own spaces kept; each state virsh says read as the
// launcher's own word, never libvirt's, every word words.json having one,
// and one it doesn't know as unknown; what can be done in each state, in
// order, and what virsh is told for each; the figures a drawer shows in
// their order and units, memory in decimal units from its bytes and in use
// only while it's on, a figure dominfo left out left out and named by why;
// the order and count Programs shows; an event read as a change; and
// virsh's error read as the foot says it.
// The `vms-read-as-virsh-says` check copies Machines.js, the shell's
// formatter it says sizes with (HomeNetwork.js, and what that imports) and
// the launcher's words beside this file and runs
// `qml -platform offscreen machines.qml`.
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
        check("a name's own spaces kept", Machines.listed(" spaced \n\n"), [" spaced "]);
        check("a line of spaces is no name", Machines.listed("web\n  \n"), ["web"]);
        check("one asked", Machines.describing(["web"]), "dominfo --domain web");
        check("several asked in one batch", Machines.describing(["web", "db2"]),
            "dominfo --domain web; dominfo --domain db2");
        check("a name virsh's line would split, quote or end, kept whole",
            Machines.describing(["build box", "it's; \"odd\""]),
            "dominfo --domain build\\ box; dominfo --domain it\\'s\\;\\ \\\"odd\\\"");
        check("a name starting with a hyphen kept a name", Machines.quoted("-h"), "\\-h");
        check("nothing asked of no machine", Machines.describing([]), "");

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
        check("its OS type, which is machinery, is left out", web.osType, undefined);
        check("its state", web.state, "running");
        check("its processors", web.processors, 4);
        check("its processor time", web.processorTime, 754.2);
        check("its most memory", [web.mostMemory, web.noLimit], [4194304, false]);
        check("its memory in use", web.memory, 2097152);
        check("whether it's persistent", web.persistent, true);
        check("whether it starts by itself", web.autostart, false);
        check("whether it has a managed save", web.managedSave, false);
        check("its security", [web.securityModel, web.securityLabel],
            ["apparmor", "libvirt-3e8f0a52-1b7d-4c4e-9a7f-2c1f4f6b9d10 (enforcing)"]);
        check("its security DOI, which is machinery, is left out", web.securityDoi, undefined);
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
            "idle": "waiting",
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
        // The launcher's words, never libvirt's "shut off", "blocked" or
        // "no state" (VKCF3Z, 2026-11-08).
        check("off's word", Machines.side("off", said), "off");
        check("waiting's word", Machines.side("waiting", said), "waiting");
        check("powering off's word", Machines.side("stopping", said), "powering off");
        check("asleep's word", Machines.side("asleep", said), "asleep");
        check("unknown's word", Machines.side("unknown", said), "unknown");
        check("crashed's word", Machines.side("crashed", said), "crashed");
        check("a state with no word says unknown's",
            Machines.side("elsewhere", said), said.unknown);

        check("running's acts", Machines.acts("running"), ["console", "power-off", "force-off"]);
        check("waiting's acts", Machines.acts("waiting"), ["console", "power-off", "force-off"]);
        check("paused's acts", Machines.acts("paused"), ["console", "resume", "force-off"]);
        check("powering off's acts", Machines.acts("stopping"), ["console", "force-off"]);
        check("off's acts", Machines.acts("off"), ["start"]);
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
        // What moves most first, then what describes it, its identifiers last.
        check("a running machine's figures", Machines.figures(web, said), [
            { id: "processor-time", value: `12${kept}min 34${kept}s` },
            { id: "memory", value: "2.15 GB" },
            { id: "processors", value: "4" },
            { id: "most-memory", value: "4.29 GB" },
            { id: "autostart", value: said.no },
            { id: "persistent", value: said.yes },
            { id: "security-model", value: "apparmor" },
            { id: "security-label", value: web.securityLabel },
            { id: "messages", value: "tainted: custom-monitor; tainted: host-cpu" },
            { id: "id", value: "3" },
            { id: "uuid", value: uuid }
        ]);
        check("a running machine that gave every figure lacks none",
            Machines.leftOut(web, said), { stopped: [], unsaid: [] });
        // dominfo gives a Used memory while it's off, which is only what
        // it would start with, so memory in use is said only while it's on.
        // Its model is none, which says nothing, as an unsaved state does.
        check("one off leaves out what it gave none of, memory in use and a model of none",
            Machines.figures(box, said).map(one => one.id),
            ["processors", "most-memory", "autostart", "persistent", "uuid"]);
        check("no limit said", Machines.figures(box, said)[1].value, said.noLimit);
        // Its model is none, so it has no label to lack; what libvirt
        // couldn't tell is named apart from what only a running one has.
        check("one off names what it left out, by why", Machines.leftOut(box, said),
            { stopped: ["processor-time", "memory", "id"], unsaid: ["managed-save"] });
        const confined = Machines.described(off.replace("model: none", "model: apparmor"))[0];
        check("one off under a model lacks its label too",
            Machines.leftOut(confined, said).stopped,
            ["processor-time", "memory", "security-label", "id"]);
        // A saved state is said only where there is one.
        const saved = Machines.described(running.replace("Managed save:   no",
            "Managed save:   yes"))[0];
        check("one with a saved state says so",
            Machines.figures(saved, said).find(one => one.id === "managed-save")?.value,
            said.yes);
        const held = Machines.described(running.replace("State:          running",
            "State:          paused"))[0];
        check("one paused still says its memory in use",
            Machines.figures(held, said).find(one => one.id === "memory")?.value, "2.15 GB");
        const unread = Machines.described(running.replace(/Used memory:.*\n/, ""))[0];
        check("one running that didn't say its memory in use lacks it as unknown",
            Machines.leftOut(unread, said), { stopped: [], unsaid: ["memory"] });
        const quiet = Machines.described(running.replace(/CPU time:.*\n/, ""))[0];
        check("one running that didn't say a figure lacks it as unknown",
            Machines.leftOut(quiet, said), { stopped: [], unsaid: ["processor-time"] });
        check("one name said alone", Machines.together(["ID"], said.and), "ID");
        check("two names said together", Machines.together(["Processor Time", "ID"], said.and),
            `Processor Time ${said.and} ID`);
        check("three names said as a list",
            Machines.together(["Processor Time", "Security Label", "ID"], said.and),
            `Processor Time, Security Label ${said.and} ID`);
        check("no names said as nothing", Machines.together([], said.and), "");
        check("each reason has its words", [vm.tells.stopped, vm.tells.unsaid]
            .every(words => typeof words === "string" && words.includes("{figures}")), true);
        check("each figure has a name", Machines.figures(web, said)
            .filter(one => !vm.parts.includes(`vm:${one.id}`)), []);
        // Decimal units from its bytes, each KiB 1024 of them.
        check("4 GiB in decimal", Machines.memory(4194304), "4.29 GB");
        check("half a GiB in decimal", Machines.memory(524288), "537 MB");
        check("a GiB and a half in decimal", Machines.memory(1572864), "1.61 GB");
        check("under a MiB, in kB", Machines.memory(512), "524 kB");
        check("no memory said, nothing", Machines.memory(null), "");
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
        ], said), `1 running \u00b7 2 off`);
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
