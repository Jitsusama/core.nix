// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml)
// through Quickshell itself, where it still makes a device's parts and
// drawer, over stand-in devices as the sound server would give them: a
// headset's microphone isn't listened to while the headset plays music,
// since listening would switch it to calls, though it is once it's in a
// call; a drawer says what the device is doing; and a device's parts are
// kept while the launcher's shut and nothing they show moved. Which
// devices are listed, and in what order, is the sound daemon's
// (modules/sound's rows.rs), and Sound's rows as it serves them are held
// in tests/sound-served.qml. The sandbox has no sound server, so which
// device is in use, and which one is watched, can't be stood in for here.
// It exits 0 once every case holds, and otherwise 1, saying which case
// didn't.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
import "launcher" as Launcher
import "launcher/Targets.js" as Targets

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Sound {
        id: sound
    }

    // A device as the sound server holds one, an object as its nodes are.
    component Node: QtObject {
        property string name
        property string nickname: ""
        property string description: name
        property bool isSink: true
        property bool isStream: false
        property int type: isSink ? PwNodeType.AudioSink : PwNodeType.AudioSource
        property var properties: ({ "device.bus": "usb" })
        property var audio: ({ volume: 0.5, muted: false })
    }

    Node {
        id: deskSpeakers
        name: "usb-desk-speakers"
    }

    // The launcher as Sound asks it, shut, and whether Sound's rows are
    // wanted (Provider.qml's wanted).
    QtObject {
        id: shutLauncher

        property bool open: false
        property bool wanting: true
        readonly property var held: []
        // Where a row's Return last asked it to go.
        property var went: null
        // What it was told wasn't done, each as "name act: why".
        property var notDone: []

        function toldNot(name, act, why) {
            notDone = notDone.concat([`${name} ${act}: ${why}`]);
        }

        function wants(source) {
            return wanting;
        }

        function goTo(to) {
            went = to;
        }
    }

    Node {
        id: deskMicrophone
        name: "usb-desk-microphone"
        description: "Desk microphone"
        isSink: false
    }

    Node {
        id: deskTwin
        name: "noise-cancelled.usb-desk-microphone"
        isSink: false
    }

    Launcher.Sound {
        id: shutSound
        launcher: shutLauncher
        serverNodes: [deskSpeakers]
    }

    // A device as the sound server gives one: its name, which way it goes,
    // its properties and its audio.
    function device(name, sink, properties) {
        return {
            name: name,
            nickname: "",
            description: name,
            isSink: sink,
            properties: properties,
            audio: { volume: 0.5, muted: false }
        };
    }

    // The machine's own card, its headphones' jack its device 0.
    readonly property var headphones: device("alsa_output.pci.headphones", true,
        { "alsa.id": "PCH", "alsa.device": "0" })
    readonly property var speakers: device("alsa_output.pci.speakers", true,
        { "alsa.id": "PCH", "alsa.device": "1" })
    // WirePlumber's stand-in for a headset's microphone, carrying the
    // headset's profile, as the headset node beside it would say.
    function microphone(profile) {
        return device("bluez_input.AA:BB:CC:DD:EE:FF", false,
            { "bluez5.loopback": "true", "api.bluez5.profile": profile });
    }

    // Quickshell hears Qt.exit only once the shell has loaded, so the cases
    // run just after.
    Component.onCompleted: Qt.callLater(root.cases)

    function cases() {
        let status = 0;
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            status = 1;
        }
        Qt.callLater(() => Qt.exit(status));
    }

    // A row made again with getters (Provider.qml's gotten) keeps every
    // field it had, each getter read as it's asked, through rows of
    // shapes grown one from another, where turning a field into a getter
    // in place lost the field grown after it under Qt 6.12 (the shell's
    // crash of 2026-11-11). A program's row is made that way, its options
    // read as :setl asks (tests/sound-served.qml).
    function gotten() {
        let muted = true;
        const grown = sound.thing({ key: "grown", name: "Grown", hue: "action" });
        const first = sound.gotten(grown, { side: () => muted ? "yes" : "no" });
        grown.sideGrown = "kept";
        const second = sound.gotten(grown, { side: () => muted ? "yes" : "no" });
        expect("a row made again keeps the field it was given since",
            Object.assign({}, second).sideGrown === "kept", Object.assign({}, second).sideGrown);
        expect("and every field it had", first.key === "grown" && first.name === "Grown"
            && second.hue === "action" && first.chord.length === 0, JSON.stringify(first));
        muted = false;
        expect("its getter is read as it's asked", second.side === "no", second.side);
    }

    function run() {
        const playing = microphone("a2dp-sink");
        expect("a headset playing music would switch", sound.switches(playing), "it wouldn't");
        const unheard = sound.levelPart(playing);
        expect("so its Level isn't listened to", unheard.side === sound.said("unheard")
            && unheard.note === sound.said("level-switches"), `${unheard.side}, ${unheard.note}`);
        const calling = microphone("headset-head-unit");
        expect("a headset in a call wouldn't", !sound.switches(calling), "it would");
        const heard = sound.levelPart(calling);
        expect("so its Level is listened to", heard.side === sound.said("silent")
            && heard.note === sound.said("level-in"), `${heard.side}, ${heard.note}`);

        programs();
        gotten();
        partsKept();
        downDrawn();
        byEarFound();
        noiseStarted();
        startsRefused();
    }

    // Choose by Ear is found from the top by its name, once for each
    // microphone noise cancelling cleans, as For You's offer words it, and
    // its Return goes to that microphone's Choose by Ear, ready to record
    // (VKCF3Z's (d), 2026-11-12). A speaker has none, nor a cleaned twin.
    function byEarFound() {
        const found = shutSound.byEarFoundOf([deskSpeakers, deskMicrophone, deskTwin]);
        expect("one Choose by Ear for each microphone it cleans",
            JSON.stringify(found.map(row => row.name)) === '["Choose by Ear for Desk microphone"]',
            JSON.stringify(found.map(row => row.name)));
        shutLauncher.went = null;
        found[0].run();
        const device = "sound:usb-desk-microphone";
        expect("its Return goes to that microphone's Choose by Ear",
            JSON.stringify(shutLauncher.went) === JSON.stringify({
                trail: ["category:computer", "collection:sound", device, `${device}:by-ear`],
                spot: "sound:by-ear:now"
            }), JSON.stringify(shutLauncher.went));
        expect("and it stays open there", found[0].stays === true, found[0].stays);
    }

    // With noise-cancelled not answering, Return on a row that offers to
    // start it has the machine daemon restart its trial's service, which
    // starts it even when its socket has stopped or failed, where asking
    // the socket again finds nothing (1KZFHC, 2026-11-12).
    function noiseStarted() {
        const row = sound.startsRow("sound:mic:noise", Words.name("sound:noise"), "", "");
        Watchers.asked = [];
        row.run?.();
        expect("noise cancelling down, its row's Return restarts its service",
            row.verb === Words.verb("sound:by-ear:down")
            && JSON.stringify(Watchers.asked) === JSON.stringify(
                [{ act: "restart-unit", unit: "noise-cancelled-trial.service" }]),
            `${row.verb}: ${JSON.stringify(Watchers.asked)}`);
    }

    // A start the machine daemon refuses is said in its words, and one it
    // can't be asked says it had no answer, on each row whose Return starts
    // a daemon, rather than nothing; one it takes says nothing (VKCF3Z on
    // #4115, 2026-11-13).
    function startsRefused() {
        const noise = Words.name("sound:noise");
        const down = Words.name("sound:down");
        const none = Words.field.becoming?.noAnswer ?? "";
        const cases = [[{ ok: true }, []],
            [{ error: { said: "no such unit" } },
                [`${noise} start: no such unit`, `${down} start: no such unit`]],
            [null, [`${noise} start: ${none}`, `${down} start: ${none}`]]];
        for (const [answer, wanted] of cases) {
            shutLauncher.notDone = [];
            Watchers.answer = answer;
            shutSound.startsRow("sound:mic:noise", noise, "", "").run?.();
            shutSound.downRow().run?.();
            expect(`a start answered ${JSON.stringify(answer)} is said as wanted`,
                none !== "" && JSON.stringify(shutLauncher.notDone) === JSON.stringify(wanted),
                JSON.stringify(shutLauncher.notDone));
        }
        Watchers.answer = { ok: true };
    }

    // With the daemon not answering, its one row's drawer says a little
    // more than its line: what it serves, its service, and that the next
    // ask starts it again (Y2YBTS's drawer sweep, 2026-11-12).
    function downDrawn() {
        const drawer = sound.downRow().drawer;
        const facts = (drawer?.facts ?? []).map(fact => `${fact.label}=${fact.value}`);
        const wanted = [`${Words.name("sound:state")}=${sound.said("down")}`,
            `${Words.name("sound:down:serves")}=${sound.said("down-serves")}`,
            `${Words.name("sound:down:service")}=sound.service`,
            `${Words.name("sound:down:starts")}=${sound.said("down-starts")}`];
        expect("the daemon down, its row's drawer says so and what it serves",
            drawer?.title === Words.name("sound:down") && facts.join("; ") === wanted.join("; "),
            `${drawer?.title}: ${facts.join("; ")}`);
        // And its Return starts the daemon again, through the machine
        // daemon, as Spend's down row starts its watch: every row does
        // something on Return (Joel, 2026-10-26).
        const row = sound.downRow();
        Watchers.asked = [];
        row.run?.();
        expect("the daemon down, its row's Return starts it again",
            Words.verb("sound:down") === "start" && row.verb === "start"
            && JSON.stringify(Watchers.asked)
                === JSON.stringify([{ act: "restart-unit", unit: "sound.service" }]),
            `${row.verb}: ${JSON.stringify(Watchers.asked)}`);
    }

    // Shut, a device's parts asked for again with nothing moved, as its
    // spots are a minute after each closing (Launcher.qml's
    // spotsFreshened), make none of them again; nor does its beginning to
    // run, which gives its node a period and a latency no part says; and
    // shown, they're made as they're asked for. What its card says is
    // served (tests/sound-served.qml), so it makes nothing here.
    function partsKept() {
        shutLauncher.wanting = false;
        const properties = deskSpeakers.properties;
        const card = { "device.bus": "usb", "alsa.id": "Desk", "alsa.device": "0" };
        deskSpeakers.properties = card;
        const first = shutSound.verbs(deskSpeakers);
        const before = shutSound.tally.rows;
        const again = shutSound.verbs(deskSpeakers);
        expect("shut, with nothing moved, its parts are made no more",
            shutSound.tally.rows === before && again === first && first.length > 0,
            `${shutSound.tally.rows - before} made of ${first.length}`);
        deskSpeakers.properties = Object.assign({}, card, { "api.alsa.period-size": "512",
            "node.max-latency": "16384/48000" });
        const running = shutSound.verbs(deskSpeakers);
        expect("nor as it begins to run", shutSound.tally.rows === before && running === first,
            `${shutSound.tally.rows - before} made of ${first.length}`);
        deskSpeakers.properties = properties;
        shutLauncher.wanting = true;
        const shown = shutSound.tally.rows;
        shutSound.verbs(deskSpeakers);
        expect("shown, they're made as they're asked for", shutSound.tally.rows > shown,
            `${shutSound.tally.rows - shown} made`);
    }

    // A program with no desktop entry is named as it names itself, for
    // Now's open-programs list; the sound daemon serves a program's own
    // drawer (modules/sound's drawer.rs). How nodes keep time is watched
    // for a device's drawer and its programs', never its list's.
    function programs() {
        expect("a program with no entry is named as it names itself",
            sound.programOf({ name: "pw-cat", properties: { "application.name": "pw-cat" } })
                .name === "pw-cat", "capitalised");
        const among = [speakers, headphones];
        for (const [key, timed] of [["sound:alsa_output.pci.speakers", speakers.name],
            ["sound:alsa_output.pci.speakers:program:Firefox", speakers.name],
            ["sound:alsa_output.pci.speakers:programs", ""], ["sound:elsewhere", ""]])
            expect(`${key} times ${timed || "nothing"}`, sound.timedBy(key, among) === timed,
                sound.timedBy(key, among));
        targets();
    }

    // Where a program's sound goes: the one target the sound daemon says a
    // program's streams share; the default chosen where they're sent to no
    // device listed; and which streams a send moves.
    function targets() {
        const sent = { 90: "stand-in-headphones" };
        const ninety = { id: 90 };
        expect("a program's streams sent alike are sent there",
            Targets.sentTo([ninety, ninety], sent) === "stand-in-headphones"
            && Targets.sentTo([ninety, { id: 93 }], sent) === ""
            && Targets.sentTo([], sent) === "", "they weren't");
        const out = { name: "stand-in-headphones", properties: { "object.serial": "61" } };
        const chosen = choices => choices.map(each => `${each.device?.name ?? "default"}`
            + `${each.chosen ? "*" : ""}`).join(" ");
        expect("the default leads, chosen while it's sent nowhere listed",
            chosen(Targets.choices([speakers, out], "")) === "default* "
            + "alsa_output.pci.speakers stand-in-headphones"
            && chosen(Targets.choices([speakers], "stand-in-headphones")) === "default* "
            + "alsa_output.pci.speakers", chosen(Targets.choices([speakers, out], "")));
        expect("a device sent to is chosen, by its name or its serial",
            chosen(Targets.choices([speakers, out], "61")) === "default "
            + "alsa_output.pci.speakers stand-in-headphones*",
            chosen(Targets.choices([speakers, out], "61")));
        const ids = Targets.sendable([{ id: "90 -d" }, { id: 9.5 }, { id: -1 }, {}, ninety]);
        expect("only a stream with a whole id is sent", JSON.stringify(ids) === '["90"]',
            JSON.stringify(ids));
    }
}
