// Drives the launcher's Sound (modules/quickshell/launcher/Sound.qml)
// through Quickshell itself, where it decides, over stand-in devices as
// the sound server would give them, with what desk-facts would say of
// their jacks and niri's workspaces on the machine's own panel alone: a
// jack with nothing plugged into it isn't listed, and says so where it's
// shown; one with headphones in is; a card starting or stopping to play
// lists nothing again, since only its jack says whether it's listed;
// speakers desk-facts says no jack of are; a display's output waits for a display; and a headset's microphone
// isn't listened to while the headset plays music, since listening would
// switch it to calls, though it is once it's in a call. The sandbox has no
// sound server, so which device is in use, and which one is watched, can't
// be stood in for here. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
import "launcher" as Launcher
import "launcher/Sounds.js" as Sounds
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

    Node {
        id: deskMicrophone
        name: "usb-desk-microphone"
        isSink: false
    }

    // The launcher as Sound asks it, shut, and whether Sound's rows are
    // wanted (Provider.qml's wanted).
    QtObject {
        id: shutLauncher

        property bool open: false
        property bool wanting: true
        readonly property var held: []

        function wants(source) {
            return wanting;
        }
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

    // The machine's own card, its headphones' jack its device 0, and an
    // HDMI output on the same card that desk-facts says nothing of.
    readonly property var headphones: device("alsa_output.pci.headphones", true,
        { "alsa.id": "PCH", "alsa.device": "0" })
    readonly property var speakers: device("alsa_output.pci.speakers", true,
        { "alsa.id": "PCH", "alsa.device": "1" })
    readonly property var hdmi: device("alsa_output.pci.hdmi-stereo", true,
        { "alsa.id": "PCH", "alsa.device": "3" })
    // WirePlumber's stand-in for a headset's microphone, carrying the
    // headset's profile, as the headset node beside it would say.
    function microphone(profile) {
        return device("bluez_input.AA:BB:CC:DD:EE:FF", false,
            { "bluez5.loopback": "true", "api.bluez5.profile": profile });
    }

    function jack(plugged) {
        return { "PCH,0:out": [{ name: "Plugged in", value: plugged }] };
    }

    // The same jack, its card now playing, as desk-facts says once sound
    // starts through it.
    function playingThrough(plugged) {
        return { "PCH,0:out": [{ name: "Plugged in", value: plugged },
            { name: "playing", value: "48 kHz \u00b7 32 bit \u00b7 period 21.3 ms (1024)" }] };
    }

    // Each time whether the headphones are listed is worked out again, as
    // Sound's devices are: a new object each time, so each time is heard.
    property int listings: 0
    readonly property var headphonesListed: ({ listed: sound.listed(headphones) })
    onHeadphonesListedChanged: listings += 1

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

    function run() {
        sound.facts = jack("nothing");
        expect("an empty jack isn't listed", !sound.listed(headphones), "listed");
        expect("an empty jack says so", sound.made(headphones).note
            .includes(sound.said("unplugged")), sound.made(headphones).note);
        expect("speakers with no jack are listed", sound.listed(speakers), "not listed");
        expect("a display's output waits for a display", !sound.listed(hdmi), "listed");
        sound.facts = jack("headphones");
        expect("a jack with headphones in is listed", sound.listed(headphones), "not listed");
        expect("and says nothing of being empty", !sound.made(headphones).note
            .includes(sound.said("unplugged")), sound.made(headphones).note);
        const before = listings;
        sound.facts = playingThrough("headphones");
        sound.facts = jack("headphones");
        expect("a card starting and stopping lists nothing again", listings === before,
            `listed ${listings - before} more times`);
        sound.facts = playingThrough("nothing");
        expect("its jack emptying does", listings > before && !sound.listed(headphones),
            `listed ${listings - before} more times`);

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

        drawers();
        shut();
        spotsMake();
    }

    // What Sound's spots are made of holds while a volume or a mute moves,
    // so a closing remakes none of them (Launcher.qml's spotsFrom), and
    // moves once a device comes or goes.
    function spotsMake() {
        const made = shutSound.spotsStamp;
        expect("it says what its spots are made of", typeof made === "string" && made !== "",
            String(made));
        deskSpeakers.audio = { volume: 0.9, muted: true };
        deskMicrophone.audio = { volume: 0.1, muted: false };
        expect("a volume and a mute moving leave it as it was", shutSound.spotsStamp === made,
            shutSound.spotsStamp);
        shutSound.serverNodes = [deskSpeakers];
        expect("a device going changes it", shutSound.spotsStamp !== made,
            shutSound.spotsStamp);
        shutSound.serverNodes = [deskSpeakers, deskMicrophone];
        expect("and coming back puts it back", shutSound.spotsStamp === made,
            shutSound.spotsStamp);
    }

    // With the launcher shut, a volume moving, a device coming and niri's
    // workspaces said again make none of Sound's rows, nor does a pick
    // asking every collection what it holds (Launcher.qml's knowing): each
    // made every device's row again, 137 in four minutes on Joel's shell
    // (YQZGPC, 2026-11-10). Home's count reads the devices themselves, and
    // Now's Volume borrows its device's row as it reads now. Wanted again,
    // they're made once, as they stand, and then a device's volume moving
    // remakes its own row alone.
    function shut() {
        const names = rows => rows.map(row => row.key).join(", ");
        expect("wanted, it lists the device there is",
            names(shutSound.everything()) === "sound:usb-desk-speakers",
            names(shutSound.everything()));
        shutLauncher.wanting = false;
        const before = shutSound.tally.rows;
        deskSpeakers.audio = { volume: 0.3, muted: false };
        shutSound.serverNodes = [deskSpeakers, deskMicrophone];
        Niri.workspaces = { "1": { id: 1, output: "eDP-1" } };
        const held = shutSound.everything();
        expect("shut, a volume, a device and niri moving make none of its rows",
            shutSound.tally.rows === before && names(held) === "sound:usb-desk-speakers",
            `${shutSound.tally.rows - before} made, holding ${names(held)}`);
        expect("and home's count says them as they are",
            shutSound.counted() === "1 out \u00b7 1 in", shutSound.counted());
        const now = shutSound.rowNow("sound:usb-desk-speakers");
        expect("a device's row borrowed elsewhere is made as it reads now",
            now?.note.includes("30%") && shutSound.tally.rows === before + 1
            && shutSound.rowNow("sound:gone") === undefined, now?.note);
        const made = shutSound.tally.rows;
        shutLauncher.wanting = true;
        const wanted = shutSound.everything();
        expect("wanted again, they're made once, as they stand",
            names(wanted) === "sound:usb-desk-speakers, sound:usb-desk-microphone"
            && wanted[0].note.includes("30%") && shutSound.tally.rows - made === 2,
            `${shutSound.tally.rows - made} made, ${names(wanted)}: ${wanted[0]?.note}`);
        const moved = shutSound.tally.rows;
        deskMicrophone.audio = { volume: 0.7, muted: false };
        const heard = shutSound.everything();
        expect("wanted, a device's volume moving remakes its row alone",
            heard[0] === wanted[0] && heard[1].note.includes("70%")
            && shutSound.tally.rows - moved === 1,
            `${shutSound.tally.rows - moved} made, ${heard[1]?.note}`);
    }

    // A device's drawer says where it is and which way it goes, then what
    // pipewire says it's doing, each of its parts the step holds, what the
    // kernel says, the format the graph moves where the card doesn't say
    // the same, and what the sound server knows it by last; never whether
    // it's muted, its row's, nor its Level, which reading would open a
    // stream on it.
    function drawers() {
        const card = device("alsa_output.pci.speakers", true, { "alsa.id": "PCH",
            "alsa.device": "1", "device.bus": "pci", "api.alsa.path": "hw:PCH,1",
            "priority.session": "1000", "alsa.resolution_bits": "24" });
        card.id = 52;
        card.state = "running";
        card.audio.format = { rate: 48000, channels: 2, sample: "S32LE" };
        sound.facts = { "PCH,1:out": [{ name: "playing", value: "48 kHz \u00b7 32 bit" }] };
        const plate = sound.drawerOf(card);
        const volume = plate.facts.find(fact => fact.label === "Volume");
        expect("a built-in output says so over its name, as it's written",
            plate.eyebrow === "built in \u00b7 output" && plate.eyebrowNames !== true,
            plate.eyebrow);
        const made = device("stand-in-speakers", true, {});
        expect("one the machine makes is on this machine",
            sound.drawerOf(made).eyebrow === "on this machine \u00b7 output",
            sound.drawerOf(made).eyebrow);
        const said = plate.facts.map(fact => `${fact.label}=${fact.value}`).join("; ");
        const labels = plate.facts.map(fact => fact.label).join(", ");
        // The kernel's fact is named by the launcher's vocabulary, which
        // there's none of here, so by its own name.
        // The graph moves the format the card already says, so it isn't
        // said again.
        expect("its state, its parts, the kernel's, then what it's known by",
            labels === "State, Volume, Resolution, Playing, Priority, ALSA Device, Graph ID", said);
        expect("what identifies it is marked so, its Graph ID last", plate.facts
            .filter(fact => fact.identifies).map(fact => fact.label).join(", ")
            === "ALSA Device, Graph ID", said);
        expect("its state is pipewire's word", plate.facts[0].value === "running", said);
        expect("its volume says its decibels, its share being its row's",
            volume?.value === Sounds.decibels(0.5), volume?.value);
        const levels = [Sounds.channelLevels(["Front Left", "Front Right"], ["100%", "100%"], "even"),
            Sounds.channelLevels(["Front Left", "Front Right"], ["100%", "80%"], "even"),
            Sounds.channelLevels(["Front Left", "Front Right", "Rear Left"], ["9%", "8%", "7%"],
                "even"), Sounds.channelLevels([], [], "even")].map(each => each.join(", "));
        expect("channels at one level are balanced, a pair's otherwise named without front",
            levels.join("; ") === "even; left 100%, right 80%; "
            + "front left 9%, front right 8%, rear left 7%; ", levels.join("; "));
        expect("and draws its share as a stroke", volume?.figure?.shape === "stroke"
            && volume.figure.values[0] === 0.5, JSON.stringify(volume?.figure));
        const formats = node => sound.drawerOf(node).facts
            .filter(fact => fact.label.endsWith("Format") || fact.label === "Playing")
            .map(fact => `${fact.label}=${fact.value}`).join("; ");
        card.audio.format = { rate: 48000, channels: 2, sample: "F32LE" };
        expect("a float graph over an integer card is the Graph Format beside it",
            formats(card) === "Playing=48 kHz \u00b7 32 bit; "
            + "Graph Format=48 kHz \u00b7 32 bit float", formats(card));
        sound.facts = { "PCH,1:out": [{ name: "playing", value: "44.1 kHz \u00b7 16 bit" }] };
        expect("as is one at another rate",
            formats(card) === "Playing=44.1 kHz \u00b7 16 bit; "
            + "Graph Format=48 kHz \u00b7 32 bit float", formats(card));
        sound.facts = {};
        expect("one the card doesn't say is its Format",
            formats(card) === "Format=48 kHz \u00b7 32 bit float", formats(card));
        card.state = "suspended";
        card.audio.format = {};
        expect("a suspended one says so, and no format", formats(card) === ""
            && sound.drawerOf(card).facts[0].value === "suspended", formats(card));
        sound.facts = { "PCH,1:out": [{ name: "playing", value: "48 kHz \u00b7 32 bit" }] };
        expect("what it's known by is as the server says it",
            said.endsWith("Priority=1000; ALSA Device=hw:PCH,1; Graph ID=52"), said);
        card.audio.muted = true;
        expect("a muted one doesn't say so again, nor its level", !sound.drawerOf(card).facts
            .some(fact => fact.label === "Muted" || fact.label === "Level"), said);
        // How it keeps time, while pw-top says it runs: ahead of its
        // Format, its Dropouts warned only past none.
        card.state = "running";
        card.audio.format = { rate: 48000, channels: 2, sample: "F32LE" };
        sound.timings = { 52: { id: 52, quantum: 1024, rate: 48000, ms: 21.333, busy: 0.0312,
            errors: 0 } };
        const keeping = node => sound.drawerOf(node).facts
            .filter(fact => ["Dropouts", "Load", "Cycle", "Graph Format"].includes(fact.label));
        expect("its timing, ahead of its format", keeping(card)
            .map(fact => `${fact.label}=${fact.value}`).join("; ") === "Dropouts=none; "
            + "Load=3.12% of each cycle; Cycle=1,024 frames \u00b7 21.3 ms; "
            + "Graph Format=48 kHz \u00b7 32 bit float", JSON.stringify(keeping(card)));
        expect("its load a stroke, its dropouts calm at none", keeping(card)[1].figure?.values[0]
            === 0.0312 && keeping(card)[0].warns === false, JSON.stringify(keeping(card)));
        sound.timings = { 52: Object.assign({}, sound.timings[52], { errors: 2 }) };
        expect("a dropout warns", keeping(card)[0].value === "2" && keeping(card)[0].warns,
            JSON.stringify(keeping(card)[0]));
        const dropouts = errors => {
            sound.timings = { 52: Object.assign({}, sound.timings[52], { errors }) };
            return keeping(card)[0].value;
        };
        expect("dropouts counted whole", [5, 12, 1500].map(dropouts).join("; ")
            === "5; 12; 1,500", [5, 12, 1500].map(dropouts).join("; "));
        sound.timings = { 52: Object.assign({}, sound.timings[52], { busy: 0.000938 }) };
        expect("a light load keeps its three significant digits",
            keeping(card)[1].value === "0.0938% of each cycle", JSON.stringify(keeping(card)[1]));
        // A measure keeps its zeros, as every measure does (VKCF3Z,
        // 2026-11-08): 960 frames at 48 kHz take 20.0 ms.
        sound.timings = { 52: Object.assign({}, sound.timings[52],
            { busy: 0.031, quantum: 960, ms: 20 }) };
        const zeros = keeping(card).slice(1, 3).map(fact => fact.value).join("; ");
        expect("a load and a cycle keep their zeros",
            zeros === "3.10% of each cycle; 960 frames \u00b7 20.0 ms", zeros);
        sound.timings = { 52: Object.assign({}, sound.timings[52],
            { busy: null, unfinished: true }) };
        expect("one that ran past its cycle is over all of it, its stroke full, warned",
            keeping(card)[1].value === "over 100% of each cycle" && keeping(card)[1].warns
            && keeping(card)[1].figure?.values[0] === 1 && keeping(card)[1].figure?.warn,
            JSON.stringify(keeping(card)[1]));
        // A frame of pw-top as it comes: a driver, then its follower in
        // the driver's cycle, whole once the next header opens.
        const header = "S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT  NAME ";
        [header, "R  52  1024  48000  20.1us 665.6us  0.00  0.03  0  F32LE 2 48000 speakers",
            "R  90  0  0  1.0us  2.1ms  0.00  0.10  0  F32LE 2 48000 + pw-cat", header]
            .forEach(line => sound.heardTiming(line));
        expect("a frame read whole, its follower in its driver's cycle",
            sound.timings[52]?.quantum === 1024 && sound.timings[90]?.quantum === 1024
            && keeping(card)[1]?.value === "3.12% of each cycle",
            JSON.stringify(sound.timings));
        // A driver that isn't running ends the one before, so the
        // follower after it has no cycle rather than the last driver's.
        ["R  52  1024  48000  20.1us 665.6us  0.00  0.03  0  F32LE 2 48000 speakers",
            "S  53  0  0  ---  ---  ---  ---  0  headphones",
            "R  91  0  0  1.0us  2.1ms  0.00  0.10  0  F32LE 2 48000 = player", header]
            .forEach(line => sound.heardTiming(line));
        expect("a follower of a driver not running has no cycle",
            sound.timings[52]?.quantum === 1024 && sound.timings[91] === undefined,
            JSON.stringify(sound.timings));
        sound.timings = {};
        expect("none while pw-top doesn't say it runs", keeping(card).length === 1,
            JSON.stringify(keeping(card)));
        expect("a headset's microphone is over Bluetooth",
            sound.drawerOf(calling()).eyebrow === "Bluetooth \u00b7 input",
            sound.drawerOf(calling()).eyebrow);
        programs();
    }

    // A stream as the sound server gives one: its id, what it plays, its
    // level and the format it moves its audio in.
    function stream(id, plays, volume) {
        return {
            id: id,
            name: "firefox",
            properties: { "media.name": plays, "media.role": "Movie", "node.rate": "1/48000",
                "node.latency": "1024/48000", "application.process.id": "4123",
                "client.id": "77" },
            audio: { volume: volume, muted: false,
                format: { rate: 48000, channels: 2, sample: "F32LE" } }
        };
    }

    // A program's drawer says what its stream plays, how it keeps time,
    // its format, delay and role, and what identifies it last; never its
    // name nor its level, its row's, nor a rate its format opens with.
    // With several streams each is a group, headed by what it plays, the
    // loudest first.
    function programs() {
        const said = drawn => drawn.facts.map(fact => fact.heading !== undefined
            ? `# ${fact.heading}` : `${fact.label}=${fact.value}${fact.identifies ? "*" : ""}`)
            .join("; ");
        sound.timings = { 90: { id: 90, quantum: 1024, rate: 48000, ms: 21.333, busy: 0.098,
            errors: 0 } };
        const one = sound.programPlate(speakers,
            { name: "Firefox", streams: [stream(90, "Big Buck Bunny - YouTube", 0.8)] });
        expect("a program's one stream", one.eyebrow === "on alsa_output.pci.speakers \u00b7 stream"
            && one.eyebrowNames === true && one.title === "Firefox"
            && said(one) === "Stream=Big Buck Bunny - YouTube; Dropouts=none; "
            + "Load=9.80% of each cycle; Cycle=1,024 frames \u00b7 21.3 ms; "
            + "Format=48 kHz \u00b7 32 bit float; Delay=21.3 ms; Role=Movie; "
            + "Process ID=4123*; Graph ID=90*; Client ID=77*",
            `${one.eyebrow} ${one.title}: ${said(one)}`);
        const asking = stream(91, "Ad", 0.4);
        asking.properties["node.rate"] = "1/44100";
        const two = sound.programPlate(speakers,
            { name: "Firefox", streams: [asking, stream(90, "Big Buck Bunny - YouTube", 0.8)] });
        expect("each of its streams a group, the loudest first, its rate where it differs",
            said(two) === "# Big Buck Bunny - YouTube; Dropouts=none; "
            + "Load=9.80% of each cycle; Cycle=1,024 frames \u00b7 21.3 ms; "
            + "Format=48 kHz \u00b7 32 bit float; Delay=21.3 ms; Role=Movie; "
            + "Process ID=4123*; Graph ID=90*; Client ID=77*; # Ad; "
            + "Format=48 kHz \u00b7 32 bit float; Sample Rate=44.1 kHz; Delay=21.3 ms; "
            + "Role=Movie; Process ID=4123*; Graph ID=91*; Client ID=77*", said(two));
        // One of several that names nothing goes unheaded, never an empty
        // heading.
        const nameless = stream(92, "", 0.2);
        nameless.name = "";
        const three = sound.programPlate(speakers, { name: "Firefox",
            streams: [stream(90, "Big Buck Bunny - YouTube", 0.8), nameless] });
        expect("a stream that names nothing has no heading",
            three.facts.filter(fact => fact.heading !== undefined).length === 1, said(three));
        // A program gone from the device, its streams perhaps gone with
        // it, draws none of them, so its drawer shuts.
        const gone = sound.programDrawn(speakers,
            { name: "Firefox", streams: [stream(90, "Big Buck Bunny - YouTube", 0.8)] });
        expect("a program gone has nothing to draw", gone.facts.length === 0, said(gone));
        // What a stream plays goes where it says nothing the title doesn't.
        for (const plays of ["", "-", "firefox", "Firefox"]) {
            const bare = sound.programPlate(speakers,
                { name: "Firefox", streams: [stream(90, plays, 0.8)] });
            expect(`a stream playing "${plays}" says no Stream`,
                !bare.facts.some(fact => fact.label === "Stream"), said(bare));
        }
        expect("a program with no entry is named as it names itself",
            sound.programOf({ name: "pw-cat", properties: { "application.name": "pw-cat" } })
                .name === "pw-cat", "capitalised");
        // A stream that says no format, rate or what it plays still has a
        // drawer, of what it does say.
        const scant = sound.programPlate(speakers,
            { name: "Firefox", streams: [{ id: 93, name: "firefox", properties: {} }] });
        expect("a stream that says little says what it does",
            said(scant) === "Graph ID=93*", said(scant));
        // pw-top times a device's drawer and its programs', never its list's.
        const among = [speakers, headphones];
        for (const [key, timed] of [["sound:alsa_output.pci.speakers", speakers.name],
            ["sound:alsa_output.pci.speakers:program:Firefox", speakers.name],
            ["sound:alsa_output.pci.speakers:programs", ""], ["sound:elsewhere", ""]])
            expect(`${key} times ${timed || "nothing"}`, sound.timedBy(key, among) === timed,
                sound.timedBy(key, among));
        sound.timings = {};
        targets();
    }

    // Where a program's sound goes: what pw-metadata says each stream is
    // sent to, only by target.object, a deletion sending it nowhere; the
    // one a program's streams share; the default chosen where they're sent
    // to no device listed; and what sends them, or brings them back.
    function targets() {
        let sent = {};
        for (const line of ["Found \"default\" metadata 48",
            "update: id:0 key:'default.audio.sink' value:'{\"name\":\"x\"}' type:'Spa:String:JSON'",
            "update: id:90 key:'target.object' value:'stand-in-headphones' type:''",
            "update: id:91 key:'target.object' value:'61' type:''",
            "update: id:92 key:'target.object' value:'stand-in-headphones' type:''",
            "update: id:92 key:'target.object' value:(null) type:(null)",
            "update: id:91 key:(null) value:(null) type:(null)"])
            sent = Targets.heard(sent, line);
        expect("only a target is heard, and a deleted one goes",
            JSON.stringify(sent) === JSON.stringify({ 90: "stand-in-headphones" }),
            JSON.stringify(sent));
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
        const held = Targets.sentAs({ 90: "x", 91: "y" }, [ninety, { id: 92 }], "z");
        expect("what's sent is held as sent, and brought back, forgotten",
            JSON.stringify(held) === JSON.stringify({ 90: "z", 91: "y", 92: "z" })
            && JSON.stringify(Targets.sentAs(held, [ninety], ""))
            === JSON.stringify({ 91: "y", 92: "z" }), JSON.stringify(held));
        const commands = (streams, target) => JSON.stringify(Targets.sending("pw-metadata",
            streams, target).map(each => each.command));
        expect("each stream is sent, or brought back to the default, its options ended first",
            commands([ninety], "stand-in-headphones") === JSON.stringify([["pw-metadata", "-n",
                "default", "--", "90", "target.object", "stand-in-headphones"]])
            && commands([ninety], "") === JSON.stringify([["pw-metadata", "-n", "default",
                "-d", "--", "90", "target.object"]]), commands([ninety], ""));
        expect("only a stream with a whole id is sent",
            commands([{ id: "90 -d" }, { id: 9.5 }, { id: -1 }, {}], "x") === "[]"
            && Targets.sending("pw-metadata", [ninety], "x")[0].id === "90",
            commands([{ id: "90 -d" }, { id: 9.5 }, { id: -1 }, {}], "x"));
    }

    function calling() {
        return microphone("headset-head-unit");
    }
}
