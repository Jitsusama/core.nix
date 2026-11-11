// Sound, a collection: the devices sound goes out of and comes in through,
// outputs then inputs, the one chosen first in each, every glyph saying
// what kind of device it is, its volume drawn as a bar and said in the
// footer. A jack with nothing plugged into it, as the headphones' while
// none are, isn't listed unless it's the one chosen, which says so. With
// the sound daemon not answering, one row says so. Return
// uses one, or mutes or unmutes the one chosen, and the right arrow holds
// its parts, each named first and saying how it stands:
//
// - an output: its Level going out, live while it shows; its Volume, in
//   percent and in decibels, with Louder, Quieter and levels to set a step
//   in; whether it's Muted, Mute and Unmute a step in; the programs
//   Playing through it, each muted or turned a step in, or sent to play
//   on another device or the default (Targets.js); its Tuning, when a
//   filter puts itself in front of it; its Port, when its card offers it
//   several; its Channels; its Profile, over Bluetooth; its Hardware; and
//   what the kernel says of its card.
// - an input: the same, its Gain for a Volume, and the programs Recording
//   from it for those Playing, its Voice processing for a Tuning.
//
// A Bluetooth device's Codec and its link are Bluetooth's (Accessories.qml),
// so they're said there, and Sound says only what its sound is for now,
// music or calls, and points there.
//
// Its name, its parts, its acts, the words that find and ask it and those
// it says are the vocabulary's (launcher/words.json, Words.qml).
//
// It answers too: "volume" or "what's playing" says how loud the output in
// use is and through what, then the input, and each program playing or
// recording; with a number, "volume 40", Return sets the output to it, and
// unmutes it, since a level asked for is one to be heard.
//
// Its rows are the sound daemon's (modules/sound), served ready over its
// socket: which devices there's something to hear through, what each is
// called, the glyph it wears, its bar and its note, in its order. Their
// parts and drawers are still made here, from the sound server through
// Quickshell, until the daemon serves them too by 13 November, so nothing
// is spawned for them, but for what a sound card says only to the kernel:
// what it offers and runs at, its clock, USB speed and firmware, which
// desk-facts writes to a file (modules/desk-facts), and which the right
// arrow lists last, Return copying one. A Level is read only while it's on
// screen, by Quickshell's own peak monitor, in the shell's process.
import qs
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Plate.js" as Plate
import "Sounds.js" as Sounds
import "Targets.js" as Targets

Provider {
    id: provider

    what: Words.of("sound").what

    // Named, and found, as the vocabulary says (Words.qml).
    collection: Words.collection("sound")

    function glyph(mark) {
        return Theme.glyph[mark];
    }

    // What it says, as the vocabulary words it, with what's in braces
    // filled in: "set {device} to {level}".
    function said(id, fill) {
        let words = (Words.of("sound").said ?? {})[id] ?? id;
        for (const key of Object.keys(fill ?? {}))
            words = words.replace(`{${key}}`, fill[key]);
        return words;
    }

    // The daemon's rows of Sound, watched whether or not the launcher's
    // open, so it opens on rows already held.
    ReadyRows {
        id: ready
        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/sound.sock`
        place: "sound"
        // A device's row is made with its parts; a part the daemon serves
        // inside one, as its Volume, as every place's rows are.
        make: (row, key) => row.kind === "device" ? provider.servedRow(row, key)
            : ready.thing(row, key, provider.partFields(row))
        maker: provider
        drawn: provider.launcher?.open ?? false
    }
    // What the daemon serves, for a reader told of each change it hears.
    readonly property alias served: ready
    readonly property bool down: ready.state === "not answering"

    Connections {
        target: ready

        function onHeard() {
            provider.changed();
        }
    }

    // The devices the daemon lists, by their node's name, in its order:
    // outputs, then inputs, each A to Z by the name it gives them.
    readonly property var listing: {
        void ready.placings;
        return ready.keys.map(key => ready.served(key)?.held?.node ?? "");
    }

    // Whether a device is listed, as the daemon decides from what its card
    // says of the jack and display it plays through (modules/sound's
    // rows.rs): a jack with nothing plugged in isn't, unless it's the one
    // chosen, nor a display's output with no display.
    function listed(node) {
        return listing.includes(node.name);
    }

    function display(node) {
        return /hdmi|displayport/i.test(`${node.name} ${node.description}`);
    }

    // Every node the sound server has, which a check with no sound server
    // gives as stand-ins.
    property var serverNodes: Pipewire.nodes.values

    // Hardware, and the filters that put themselves in front of it, as the
    // speakers' tuning and the MV7's voice do: not a program's stream.
    readonly property var nodes: serverNodes
        .filter(node => !!node.audio && !node.isStream
            && (node.type & PwNodeType.Audio) && (node.isSink || (node.type & PwNodeType.Source)))
    readonly property var filters: nodes.filter(node => node.properties["filter.smart"] === "true")
    readonly property var devices: nodes
        .filter(node => node.properties["filter.smart"] !== "true" && listed(node))
    readonly property var outputs: devices.filter(node => node.isSink)
    readonly property var inputs: devices.filter(node => !node.isSink)

    // :set mute and :set volume=40 act on the output sound plays through,
    // as its rows do, a volume set unmuting it as their levels do.
    setOptions: ({
        mute: {
            kind: "switch",
            now: () => Pipewire.defaultAudioSink?.audio?.muted ?? false,
            set: to => provider.toOut(out => out.audio.muted = to)
        },
        volume: {
            kind: "number",
            least: 0,
            most: 100,
            now: () => Math.round((Pipewire.defaultAudioSink?.audio?.volume ?? 0) * 100),
            set: to => provider.toOut(out => {
                out.audio.volume = to / 100;
                out.audio.muted = false;
            })
        },
        // The output sound plays through and the input listened to, by the
        // names their rows give them, as Play Sound Through It and Listen
        // Through It choose them (VKCF3Z, 2026-11-05: choosing a default
        // stays :set's, the machine's).
        output: provider.chooser(() => provider.outputs, () => Pipewire.defaultAudioSink),
        input: provider.chooser(() => provider.inputs, () => Pipewire.defaultAudioSource)
    })

    // An option choosing one of the devices listed by its name, where it
    // stands the one in use. They're read as the line asks, not bound, so
    // the options aren't made again each time a device or its jack does.
    function chooser(devices, using) {
        const shown = () => devices().filter(node => provider.listed(node));
        return {
            kind: "choice",
            get values() {
                return shown().map(node => provider.named(node));
            },
            now: () => using() ? provider.named(using()) : "",
            set: to => {
                const node = shown().find(each => provider.named(each) === to);
                if (node)
                    provider.use(node);
                return "";
            }
        };
    }

    function toOut(change) {
        const out = Pipewire.defaultAudioSink;
        if (out?.audio)
            change(out);
        return "";
    }

    // Each stream open to play through, which carries the server's sink
    // flag, and each open to record from, which doesn't.
    readonly property var streams: serverNodes
        .filter(node => node.isStream && node.isSink && (node.type & PwNodeType.Audio))
    readonly property var recorders: serverNodes
        .filter(node => node.isStream && !node.isSink && (node.type & PwNodeType.Audio))

    // A node's volume and properties are heard only once something binds
    // it; a stream's say which program it is.
    PwObjectTracker {
        objects: provider.nodes.concat(provider.streams, provider.recorders)
    }

    // A program's own stream. A filter's own streams, such as the tuning's
    // way out to the speakers, carry the link group its two halves share.
    // The peaks this shell reads to draw a Level, here and in dictation,
    // open a recording stream of their own (PwNodePeakMonitor), so counting
    // them would say the launcher records every microphone it shows, and
    // another program's level meter is a monitor too: none records anything.
    function own(node) {
        return !!node && !!node.audio && node.name !== "quickshell-peak-monitor"
            && !node.properties["node.link-group"] && node.properties["stream.monitor"] !== "true";
    }

    function chosen(node) {
        return node === (node.isSink ? Pipewire.defaultAudioSink : Pipewire.defaultAudioSource);
    }

    // Its own short name, rather than the sound card's long one before it.
    function nicknamed(node) {
        // A node going away can have lost all three.
        return node.nickname || node.description || node.name || "";
    }

    // What it's called, saying the thing itself: Joel, on 2026-10-25,
    // "speaker is singular and microphone is plural, they give me mixed
    // ideas", and a headset's two rows said the same name. So the machine's
    // own are Built-in speakers, Built-in microphones and the Headphone
    // jack, and a headset's microphone is named as its (Sounds.js called).
    // A noise-cancelled microphone is the one it hears, named for it, as the
    // sound daemon names its row (modules/sound's rows.rs).
    function named(node) {
        const own = nicknamed(node);
        const shared = inputs.includes(node) && outputs.some(output => nicknamed(output) === own);
        const heard = String(node.name).replace(/^noise-cancelled\./, "");
        const called = Sounds.called(heard, shared);
        const name = called === "" ? own
            : Words.name(`sound:called:${called}`, "", { device: own });
        return heard === node.name ? name
            : Words.name("sound:called:noise-cancelled", "", { device: name });
    }

    // What it is, from what the sound server knows of it, and for a
    // Bluetooth device what BlueZ says it is, since the server keeps that
    // on the device rather than its sound: over Bluetooth, through a
    // display, worn, or the machine's own.
    function kind(node) {
        const address = Sounds.address(node.name, node.properties);
        const accessory = address
            ? Bluetooth.devices.values.find(device => device.address === address) : null;
        const known = [node.name, nicknamed(node), node.properties["device.form-factor"] ?? "",
            accessory ? accessory.icon : ""].join(" ").toLowerCase();
        // Headphones are headphones and a speaker a speaker to Joel, however
        // they're joined, so a Bluetooth one wears the same mark as a wired.
        if (!node.isSink)
            return known.includes("headset") ? "headset" : "microphone";
        if (display(node))
            return "display";
        if (known.includes("head"))
            return "headphones";
        return "speaker";
    }

    // The glyph it wears, struck through while it's muted.
    function worn(node) {
        return node.audio.muted ? struck(node) : glyph(kind(node));
    }

    function struck(node) {
        return node.isSink ? Theme.glyph.silent : Theme.glyph.muted;
    }

    function use(node) {
        if (node.isSink)
            Pipewire.preferredDefaultAudioSink = node;
        else
            Pipewire.preferredDefaultAudioSource = node;
    }

    // What a device says as its parts stand, each part as text and its
    // level as a share, so its parts are made again only once one moves.
    function says(node) {
        return [named(node), worn(node), chosen(node), audible(node), switches(node)];
    }

    // The sound server's node a served row stands for, as the server has
    // it now, or undefined while it has none by that name.
    function nodeOf(row) {
        const name = row.held?.node;
        return nodes.find(node => node.name === name);
    }

    // A device's row as the daemon serves it. Return steps in, as every
    // device's does, with its likeliest act first inside: using it, or for
    // the one chosen, whether it's Muted (docs/design.md, a switch is one
    // row). Its parts and drawer are made here from its node as they're
    // asked for, until the daemon serves them too.
    function servedRow(row, key) {
        const inside = () => {
            const node = provider.nodeOf(row);
            return node?.audio ? provider.verbs(node) : [];
        };
        const made = ready.thing(row, key, {
            thing: true,
            verbs: inside,
            targets: inside,
            drawer: () => {
                const node = provider.nodeOf(row);
                return node ? provider.drawerOf(node) : null;
            }
        });
        // Read as :setl asks, so a port read since it was drawn is one.
        Object.defineProperty(made, "options", {
            enumerable: true,
            get: () => {
                const node = provider.nodeOf(row);
                return node?.audio ? provider.deviceOptions(node) : ({});
            }
        });
        return made;
    }

    // With the daemon not answering, one row says so, since none listed
    // would read as the machine having no sound. Its socket is systemd's,
    // so the next ask starts it again.
    function downRow() {
        return thing({
            key: "sound:down",
            name: Words.name("sound:down"),
            note: said("down"),
            glyph: Theme.glyph.caution,
            hue: "warning"
        });
    }

    // A device's own options, and a program's playing through it, which
    // :setl sets as Vim's :setlocal sets a buffer's (VKCF3Z, 2026-11-05):
    // whether it's muted, how loud, and a device's port where it has a
    // choice of them, on it alone, never the default output :set's are
    // (setOptions). A level set or raised is meant to be heard, so it
    // unmutes, as its rows' do.
    function nodeOptions(nodes) {
        const live = () => nodes.filter(node => !!node.audio);
        return {
            mute: {
                kind: "switch",
                now: () => provider.hushed(nodes),
                set: to => {
                    for (const node of live())
                        node.audio.muted = to;
                    return "";
                }
            },
            volume: {
                kind: "number",
                least: 0,
                most: 100,
                now: () => Math.round(provider.loudestOf(nodes) * 100),
                set: (to, change) => {
                    for (const node of live()) {
                        node.audio.volume = to / 100;
                        if (change?.how !== "subtract")
                            node.audio.muted = false;
                    }
                    return "";
                }
            }
        };
    }

    function deviceOptions(node) {
        const options = nodeOptions([node]);
        const choices = portsServed(node).slice(1);
        if (choices.length > 0)
            options.port = {
                kind: "choice",
                values: choices.map(choice => choice.held.port),
                now: () => choices.find(choice => choice.held.chosen)?.held.port ?? "",
                set: to => {
                    const choice = choices.find(one => one.held.port === to);
                    if (choice)
                        ready.act(choice.key, choice.acts[0]);
                    return "";
                }
            };
        return options;
    }

    // What desk-facts knows of each card, by its ALSA id and direction; a
    // card it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/sound.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // What the kernel says of a device's card, by its key in the file, or
    // "". A card with one device each way is keyed by its id and direction
    // ("MV7:in"); one with several, as the machine's own has its speakers,
    // headphones and displays, by its device's number too
    // ("sofsoundwire,2:out"), and whichever the file holds is the one.
    function factKey(node) {
        // Read once: Quickshell makes a node's properties afresh each time.
        const properties = node.properties;
        const card = properties["alsa.id"];
        if (!card)
            return "";
        const way = node.isSink ? "out" : "in";
        const device = properties["alsa.device"] ?? properties["api.alsa.pcm.device"];
        const keys = (device !== undefined ? [`${card},${device}:${way}`] : [])
            .concat([`${card}:${way}`]);
        return keys.find(each => facts[each] !== undefined) ?? "";
    }

    function factsOf(node) {
        const key = factKey(node);
        return key === "" ? [] : facts[key] ?? [];
    }

    // Each of them named first as a part is, Return copying its value.
    function factRows(node) {
        const key = factKey(node);
        return key === "" ? []
            : factParts("sound", key, facts[key], ["lighting", "playing", "recording"]);
    }

    // The facts a device's or a stream's drawer knows, each by its word and
    // only where it has a value, marked as identifiers where they are:
    // pairs of the word's id under sound: and its value.
    function known(ids, identifies) {
        return ids.filter(([, value]) => value !== "").map(([id, value]) => ({
            label: Words.name(`sound:${id}`), value: value, identifies: identifies
        }));
    }

    // A device's drawer (Plate.js): where it is and which way it goes over
    // its name, then its parts as facts, what moves most first (VKCF3Z,
    // 2026-11-07): its State as pipewire says it, its Volume in decibels,
    // its share being its row's, the programs Playing through it and how
    // loud, its Channels balanced or each at its own level, its Tuning,
    // Port and Profile; then what describes it, its Hardware's figures
    // each a fact and what the kernel says of its card, and the Format the
    // graph moves its audio in where the card's doesn't say it; then the
    // priority that picks the one in use; and last what it's known by,
    // marked as identifiers (`identifies`): the ALSA device it opens, as
    // aplay -D takes it, a USB card's ids, and its Graph ID, as wpctl
    // takes it, ending the drawer (VKCF3Z, 2026-11-08). Whether it's Muted is its row's
    // to say, so it isn't said again. Its Level is left out, since reading
    // it opens a stream on a microphone and wakes a sleeping card, which
    // only the step in does; so is a Bluetooth device's codec, which is
    // Bluetooth's to say (Accessories.qml). Each figure is read as it's
    // drawn, so the drawer follows them live, as the rows do.
    function drawerOf(node) {
        if (!node.audio)
            return null;
        const hardware = hardwarePart(node)?.verbs() ?? [];
        const usb = `sound:${node.name}:hardware:usb-id`;
        const channels = Array.from(node.audio.channels ?? []);
        const volumes = Array.from(node.audio.volumes ?? []);
        const parts = [volumePart(node), programsPart(node), channelsPart(node),
            filterPart(node), portPart(node), profilePart(node)]
            .concat(hardware.filter(row => row.key !== usb), factRows(node));
        const facts = Plate.facts(parts, row => {
            switch (row.key) {
            case `sound:${node.name}:volume`:
            case `sound:${node.name}:gain`:
                return { value: row.note };
            case `sound:${node.name}:programs`:
            case `sound:${node.name}:recorders`:
                return { value: provider.programsSaid(node) || row.side };
            case `sound:${node.name}:channels`:
                return { value: Plate.runs([row.side].concat(Sounds.channelLevels(
                    channels.map(channel => Sounds.channel(PwAudioChannel.toString(channel))),
                    channels.map((channel, at) => provider.percent(volumes[at] ?? 0)),
                    said("balanced")))) };
            case `sound:${node.name}:tuning`:
            case `sound:${node.name}:voice`:
                return { value: Plate.runs([row.side].concat(provider.filtersOf(node)
                    .filter(filter => !!filter.audio)
                    .map(filter => `${provider.named(filter)} ${provider.level(filter)}`))) };
            default:
                return null;
            }
        });
        const properties = node.properties;
        // What pipewire says the node is doing moves most, so it leads, in
        // pipewire's own words, running, idle or suspended (VKCF3Z,
        // 2026-11-08); the row's side never says it.
        const state = String(node.state ?? "");
        const doing = state === "" ? [] : [{ label: Words.name("sound:state"), value: state }];
        // The format the graph moves its audio in is said once: as Format
        // where the card's running stream doesn't say one, not at all where
        // it says the same, and as Graph Format beside a card's that differs.
        const graph = Sounds.graphFormat(node.audio.format);
        const card = factsOf(node)
            .find(fact => ["playing", "recording"].includes(fact.name.toLowerCase()))?.value;
        const format = graph === "" || (card && Sounds.sameFormat(graph, card)) ? []
            : [{ label: Words.name(card ? "sound:graph-format" : "sound:format"), value: graph }];
        // How it keeps time moves each second, so it goes ahead of Format
        // (VKCF3Z, 2026-11-08).
        const keeping = keepingOf(node);
        const identified = fact => Object.assign({}, fact, { identifies: true });
        return {
            // Where it is and what it is, in our own words, so in Label's
            // capitals (VKCF3Z, 2026-11-09): one the machine makes, joined
            // by nothing, is on this machine.
            eyebrow: Plate.runs([joinedBy(node) || said("joined-here"),
                said(node.isSink ? "plate-out" : "plate-in")]),
            title: named(node),
            facts: doing.concat(facts, keeping, format,
                known([["priority", String(properties["priority.session"] ?? "")]], false),
                known([["alsa-device", String(properties["api.alsa.path"] ?? "")]], true),
                Plate.facts(hardware.filter(row => row.key === usb)).map(identified),
                known([["graph-id", String(node.id ?? "")]], true))
        };
    }

    // How a node keeps time, as pw-top last said it (VKCF3Z, 2026-11-08):
    // its Dropouts, warned past none, as a glitch it recovers from rather
    // than a failure; its Load, the share of each cycle it was busy; and
    // its Cycle, the frames it moves each time and how long that takes.
    // Only a running node has them. One that ran past its cycle has taken
    // more than all of it, so its Load says so in its own form, its stroke
    // full, both warned. A measure, a load or a cycle's milliseconds,
    // keeps three significant digits with their zeros, as every measure
    // does; a count, dropouts or frames, is whole.
    function keepingOf(node) {
        const timed = timings[node.id] ?? null;
        if (timed === null)
            return [];
        const measure = value => Numbers.significant(value, settings.group, settings.point);
        const count = value => Numbers.format(value, 0, settings.group, settings.point);
        const load = timed.unfinished ? [{
            label: Words.name("sound:load"),
            value: said("each-cycle", {
                share: said("over", { share: `${measure(Sounds.load(1))}%` })
            }),
            warns: true,
            figure: { shape: "stroke", values: [1], warn: true }
        }] : timed.busy === null ? [] : [{
            label: Words.name("sound:load"),
            value: said("each-cycle", { share: `${measure(Sounds.load(timed.busy))}%` }),
            figure: { shape: "stroke", values: [Math.min(timed.busy, 1)], warn: false }
        }];
        return [{
            label: Words.name("sound:dropouts"),
            value: timed.errors === 0 ? said("none") : count(timed.errors),
            warns: timed.errors > 0
        }].concat(load, [{
            label: Words.name("sound:cycle"),
            value: said("cycle", { frames: count(timed.quantum),
                ms: measure(Sounds.cycleMs(timed)) })
        }]);
    }

    // How a device is joined to the machine, in the vocabulary's words:
    // through a display, over Bluetooth, by USB or built in, or "" where
    // the sound server doesn't say. Bluetooth is a name, so it's said as
    // the vocabulary names it.
    function joinedBy(node) {
        const properties = node.properties;
        const bus = String(properties["device.bus"] ?? "");
        const how = display(node) ? "display"
            : Sounds.address(node.name, properties) !== "" || bus === "bluetooth" ? "bluetooth"
            : bus === "usb" ? "usb" : bus === "pci" ? "built-in" : "";
        return how === "" ? "" : how === "bluetooth" ? Words.name("bluetooth")
            : said(`joined-${how}`);
    }

    // Each program playing through a device, or recording from it, and how
    // loud, a run apart: "Spotify 100% \u00b7 Chrome paused at 45%".
    function programsSaid(node) {
        return Plate.runs(programsOf(node).map(program => {
            const paused = program.streams
                .every(stream => stream.properties["pulse.corked"] === "true");
            const level = streamLevel(program.streams);
            return `${program.name} ${paused ? said("paused-at", { level: level }) : level}`;
        }));
    }

    // A device's right arrow, kept while the launcher is shut and nothing
    // its parts say has moved, as its spots ask for it again a minute after
    // each closing (Launcher.qml's spotsFreshened): it made Sound's 111
    // parts again each time with nothing here changed (Y2YBTS,
    // 2026-11-10). Shown, they're made as they're asked for, as they read.
    // A sound daemon serving Sound's rows ready takes this over by
    // 13 November.
    readonly property var partsKeeping: keeper()
    function verbs(node) {
        if (provider.wanted)
            return partsMade(node);
        return keptAlike(partsKeeping, `sound:${node.name}`, [node, partsMadeOf(node)],
            () => partsMade(node));
    }

    // What a device's parts are made of, every figure they say included:
    // its row's words, its properties, channels and levels, its programs'
    // and filters' levels, its ports and its facts.
    function partsMadeOf(node) {
        const programs = programsOf(node).map(program => [program.name,
            program.streams.map(stream => [stream.audio?.volume, stream.audio?.muted,
                stream.properties["pulse.corked"]])]);
        const filters = filtersOf(node).map(filter => [filter.name, filter.audio?.volume,
            filter.audio?.muted, flowing(filter, node)]);
        return JSON.stringify([says(node), switches(node), carried(node), node.properties,
            Array.from(node.audio?.channels ?? []), Array.from(node.audio?.volumes ?? []),
            node === metered ? heard : [], programs, filters, servedOf(node),
            facts[factKey(node)] ?? []]);
    }

    // Every record the daemon serves inside a device's row, at any depth.
    function servedOf(node) {
        const inside = `row:sound:${node.name}:`;
        return ready.keys.filter(key => key.startsWith(inside)).map(key => ready.served(key));
    }

    // A part the daemon serves inside a device's row, by the last of its
    // key, as it's made, or null while it serves none.
    function servedPart(node, part) {
        const key = `row:sound:${node.name}:${part}`;
        return ready.served(key) ? ready.rowAt(key) : null;
    }

    // What a served part carries for the launcher beyond its row: a
    // setting offers its choices, and using a device names it on its
    // line, as :set output= or input= does.
    function partFields(row) {
        const fields = row.kind === "setting" ? { offers: true } : {};
        if (row.held?.fill) {
            fields.fill = row.held.fill;
            fields.wordState = row.held.state ?? "";
        }
        return fields;
    }

    // Using it, when it isn't the one chosen, then its parts, and what the
    // kernel says of its card. Using it, Muted, its Volume and its Port
    // are served; the rest are made here until the daemon serves them too.
    function partsMade(node) {
        const parts = [servedPart(node, "use"), servedPart(node, "mute"), levelPart(node),
            volumePart(node), programsPart(node), filterPart(node), portPart(node),
            channelsPart(node), profilePart(node), hardwarePart(node)];
        return parts.filter(part => part !== null).concat(factRows(node));
    }

    // Figures, as the theme draws them.

    function percent(share) {
        return `${Math.round(share * 100)}%`;
    }

    function bar(share) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, share))], warn: false };
    }

    // A figure: it says something, wears no glyph, and Return copies
    // nothing; a leaf worth having, such as a chip, copies what it says.
    function figureRow(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    function leafRow(fields) {
        return copyRow(Object.assign({ copy: fields.side, hue: "machine", glyph: "", note: "" },
            fields));
    }

    // The Level: what's going out of a device or coming into it, the
    // loudest of each moment, as a spark.

    // The device whose parts are on screen, the step held last, and only
    // while the launcher is open: its Level is read then, and never
    // otherwise, since reading it wakes a sleeping device and opens a
    // stream from a microphone. A collection's row found from the top has
    // a device's key too, but holds devices, not parts, and says where it's
    // from.
    readonly property var watched: {
        const steps = launcher && launcher.open ? launcher.held ?? [] : [];
        const last = steps.length > 0 ? steps[steps.length - 1].row : null;
        if (!last || last.from)
            return null;
        return devices.find(node => `sound:${node.name}` === last.key) ?? null;
    }
    onWatchedChanged: {
        heard = [];
        loudest = 0;
    }

    // The loudest peak since the last moment, and each moment's, newest
    // last, as many as a spark draws.
    property real loudest: 0
    property var heard: []
    readonly property int moments: 20

    // The device whose Level is read: the one watched, unless reading it
    // would switch a headset playing music to calls (`switches`).
    readonly property var metered: watched !== null && !switches(watched) ? watched : null

    PwNodePeakMonitor {
        node: provider.metered
        enabled: provider.metered !== null
        onPeakChanged: provider.loudest = Math.max(provider.loudest, peak)
    }

    // Four moments a second: fast enough to follow a voice, slow enough
    // that each one makes the device's parts again only a few times.
    Timer {
        interval: 250
        repeat: true
        running: provider.metered !== null
        onTriggered: {
            provider.heard = provider.heard.concat([provider.loudest]).slice(-provider.moments);
            provider.loudest = 0;
            provider.changed();
        }
    }

    function levelPart(node) {
        // A headset's microphone, while it plays music, is heard only by
        // switching it to calls, so it isn't, and says why.
        if (switches(node))
            return figureRow({
                key: `sound:${node.name}:level`,
                name: Words.name("sound:level"),
                side: said("unheard"),
                note: said("level-switches")
            });
        const peaks = node === metered ? heard : [];
        const now = peaks.length > 0 ? peaks[peaks.length - 1] : 0;
        return figureRow({
            key: `sound:${node.name}:level`,
            name: Words.name("sound:level"),
            side: now > 0 ? percent(now) : said("silent"),
            // Drawn by the square root, as dictation's wave is, since a
            // voice peaks low and would barely lift a linear spark.
            graph: {
                shape: "spark",
                values: peaks.map(peak => Math.sqrt(Math.max(0, Math.min(1, peak)))),
                warn: false
            },
            note: said(node.isSink ? "level-out" : "level-in")
        });
    }

    // How far a program's Louder and Quieter move it.
    readonly property real step: 0.05

    // The Volume, or a microphone's Gain, as the daemon serves it: a
    // setting saying how it's set, Louder, Quieter and its levels a step
    // in, each asking the daemon to set it.
    function volumePart(node) {
        return servedPart(node, node.isSink ? "volume" : "gain");
    }

    // An act on a device, as the vocabulary says it, in the state it's in.
    // Each changes how something sounds, so it keeps the launcher, its row
    // saying what it changed to.
    function act(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state, fields.fill),
            hue: "action",
            stays: true
        }, fields));
    }

    // The one Muted switch a program has, as a device's is served (VKCF3Z,
    // 2026-10-29: one idea has one word): it keeps its name as it flips,
    // its side saying yes or no (docs/design.md, "A switch is one row that
    // keeps its name as it flips"). How it stands is read as it's
    // drawn and as Return comes, so the row says what Return just did
    // without being made again.
    function mutedSwitch(key, muted, mark, flip) {
        const row = thing({
            key: key,
            name: Words.name("sound:mute"),
            hue: "action",
            stays: true,
            state: () => provider.said(muted() ? "yes" : "no"),
            run: flip
        });
        Object.defineProperties(row, {
            verb: {
                enumerable: true,
                get: () => Words.verb("sound:mute", muted() ? "muted" : "")
            },
            glyph: { enumerable: true, get: mark },
            side: { enumerable: true, get: () => provider.said(muted() ? "yes" : "no") }
        });
        return row;
    }

    // The programs Playing through an output, or Recording from an input:
    // each a part saying how loud it is, muted or turned a step in.

    // Each stream linked to a device, or to a filter in front of it, which
    // is where programs are linked while the filter's there.
    function linkedTo(node) {
        const ends = [node].concat(filtersOf(node));
        const found = [];
        for (const group of Pipewire.linkGroups.values) {
            const stream = node.isSink ? group.source : group.target;
            const end = node.isSink ? group.target : group.source;
            if (stream && stream.isStream && ends.includes(end) && own(stream)
                    && !found.includes(stream))
                found.push(stream);
        }
        return found;
    }

    // A program's name and glyph, as its desktop entry says them, looked up
    // once each, or the name it gives with a capital.
    readonly property var entries: ({})

    function programOf(stream) {
        const given = String(stream.properties["application.name"] ?? stream.name ?? "");
        if (!(given in entries)) {
            const entry = given === "" ? null : DesktopEntries.heuristicLookup(given);
            entries[given] = {
                // Named as it names itself, "pw-cat" as "pw-cat": nothing
                // capitalises a name (VKCF3Z, 2026-11-09).
                name: entry ? entry.name : given,
                id: entry ? entry.id : given
            };
        }
        return entries[given];
    }

    function programsOf(node) {
        return Sounds.grouped(linkedTo(node), stream => provider.programOf(stream).name);
    }

    function loudestOf(streams) {
        return Math.max(0, ...streams.filter(stream => !!stream.audio)
            .map(stream => stream.audio.volume));
    }

    function hushed(streams) {
        return streams.every(stream => !stream.audio || stream.audio.muted);
    }

    function streamLevel(streams) {
        return hushed(streams) ? said("muted") : percent(loudestOf(streams));
    }

    // A program's streams turned as one, from the loudest.
    function turn(streams, by) {
        const to = Sounds.stepped(loudestOf(streams), by, step);
        for (const stream of streams.filter(each => !!each.audio)) {
            stream.audio.volume = to;
            if (by > 0)
                stream.audio.muted = false;
        }
    }

    function programsPart(node) {
        const programs = programsOf(node);
        const id = node.isSink ? "programs" : "recorders";
        const key = `sound:${node.name}:${id}`;
        // No program listening to a microphone is worth saying; an output
        // nothing plays through has nothing to hold.
        if (programs.length === 0)
            return node.isSink ? null : figureRow({
                key: key,
                name: Words.name(`sound:${id}`),
                side: said("nothing"),
                note: said("recording-none")
            });
        return holderRow({
            key: key,
            name: Words.name(`sound:${id}`),
            side: programs.map(program => program.name).join(", "),
            glyph: glyph(kind(node))
        }, () => provider.programsOf(node).map(program => provider.programPart(node, program)));
    }

    function programPart(node, program) {
        const streams = program.streams;
        const first = streams[0];
        const paused = streams.every(stream => stream.properties["pulse.corked"] === "true");
        const key = `sound:${node.name}:program:${program.name}`;
        return thingRow({
            thing: true,
            key: key,
            options: Object.assign(nodeOptions(streams), sentOption(node, program)),
            name: program.name,
            side: paused ? `${streamLevel(streams)} \u00b7 ${said("paused")}` : streamLevel(streams),
            graph: bar(hushed(streams) ? 0 : loudestOf(streams)),
            glyph: programGlyph(programOf(first).id),
            drawer: () => provider.programDrawn(node, program)
        }, () => {
            const stepping = by => ({
                glyph: Theme.glyph.volume,
                stays: true,
                state: () => provider.said("now", { level: provider.streamLevel(streams) }),
                run: () => provider.turn(streams, by)
            });
            const hush = () => provider.hushed(streams);
            const rows = [mutedSwitch(`${key}:mute`, hush,
                () => Words.glyph("verb:program:mute", hush() ? "muted" : ""), () => {
                    const muting = !hush();
                    for (const stream of streams.filter(each => !!each.audio))
                        stream.audio.muted = muting;
                }), act("verb:program:louder", "", stepping(step)),
            act("verb:program:quieter", "", stepping(-step))];
            const sent = provider.sentPart(node, program, key);
            if (sent !== null)
                rows.push(sent);
            return rows.concat(provider.streamFigures(key, first));
        });
    }

    // A stream's Sample Rate, as it asks for one, and its Delay: how long
    // it waits first, a measure by the one rule, and how that's asked for,
    // in the footer.
    function streamFigures(key, stream) {
        const rows = [];
        const rate = Sounds.rate(stream.properties["node.rate"]);
        if (rate !== "")
            rows.push(figureRow({
                key: `${key}:rate`,
                name: Words.name("sound:rate"),
                side: rate
            }));
        const asked = Sounds.fraction(stream.properties["node.latency"]);
        if (asked)
            rows.push(figureRow({
                key: `${key}:delay`,
                name: Words.name("sound:delay"),
                side: `${Numbers.significant(asked.ms, settings.group, settings.point)} ms`,
                note: said("delay-note", {
                    frames: asked.frames,
                    rate: Sounds.kilohertz(asked.hertz)
                })
            }));
        return rows;
    }

    // A program's drawer, playing through a device or recording from one
    // (VKCF3Z, 2026-11-08): each of its streams, loudest then newest
    // first (by its graph ID, which pipewire gives out rising but reuses,
    // so newest only roughly), a group headed by what it plays where it has more than one,
    // or that said as its Stream where it has one. Under it, how it keeps
    // time, which moves most; its Format; its Sample Rate and Delay, its
    // parts, the rate only where its Format doesn't open with it; its
    // Role, in pipewire's word as given; the filter it plays through or
    // records from, where one stands in front of the device; and what
    // identifies it, ending its group: its Process ID, Graph ID and Client
    // ID. Its name is the row's, and so is whether it's paused or muted,
    // so neither is said again; its eyebrow says where it plays and that
    // it's a stream, in Note since it names the device, as Media's "on
    // Spotify \u00b7 track" does (VKCF3Z, 2026-11-09). It's read as it's
    // drawn, its streams as the device has them now; a program gone has
    // none, its old streams' nodes perhaps gone with it, so its drawer
    // shuts.
    function programDrawn(node, program) {
        return programPlate(node, programsOf(node).find(each => each.name === program.name)
            ?? { name: program.name, streams: [] });
    }

    function programPlate(node, live) {
        const key = `sound:${node.name}:program:${live.name}`;
        const loudness = stream => stream.audio ? stream.audio.volume : 0;
        const streams = Order.sorted(live.streams.slice(),
            (a, b) => loudness(b) - loudness(a) || b.id - a.id);
        const headed = streams.length > 1;
        return {
            eyebrow: Plate.runs([said(node.isSink ? "stream-on" : "stream-from",
                { device: named(node) }), said("plate-stream")]),
            eyebrowNames: true,
            title: live.name,
            facts: [].concat(...streams.map(stream => provider.streamFacts(node, key, stream,
                headed, live.name)))
        };
    }

    // What a stream plays says something only where it isn't empty, "-"
    // (pw-cat's, playing what it's given), or the program's own name
    // again (VKCF3Z, 2026-11-09); a group goes by the stream's own name
    // then. Its end is said only where it's a filter in front of the
    // device, since the eyebrow names the device.
    function streamFacts(node, key, stream, headed, program) {
        const properties = stream.properties;
        const given = String(properties["media.name"] ?? "").trim();
        const programNames = [program, String(properties["application.name"] ?? "")]
            .map(name => name.toLowerCase());
        const plays = given === "-" || programNames.includes(given.toLowerCase()) ? ""
            : given;
        // One of several is headed by what it plays, else its own name, and
        // goes unheaded where it has neither; one alone says it as Stream.
        const called = plays || String(stream.name ?? "");
        let head = [];
        if (headed && called !== "")
            head = [{ heading: called }];
        else if (!headed && plays !== "")
            head = [{ label: Words.name("sound:stream"), value: plays }];
        const format = Sounds.graphFormat(stream.audio?.format);
        const rate = Sounds.rate(properties["node.rate"]);
        const opensWithRate = format !== "" && Sounds.sameFormat(rate, format);
        return head.concat(keepingOf(stream),
            format === "" ? [] : [{ label: Words.name("sound:format"), value: format }],
            Plate.facts(streamFigures(key, stream)
                .filter(row => !(opensWithRate && row.key === `${key}:rate`))),
            known([["role", String(properties["media.role"] ?? "")],
                [node.isSink ? "plays-on" : "records-from", endOf(stream, node)]], false),
            known([["process", String(properties["application.process.id"] ?? "")],
                ["graph-id", String(stream.id ?? "")],
                ["client", String(properties["client.id"] ?? "")]], true));
    }

    // The filter in front of the device a stream is linked to, by the
    // filter's name; "" while it's linked to the device itself, which the
    // eyebrow names, or to neither.
    function endOf(stream, node) {
        const ends = filtersOf(node);
        for (const group of Pipewire.linkGroups.values) {
            const from = node.isSink ? group.source : group.target;
            const end = node.isSink ? group.target : group.source;
            if (from === stream && ends.includes(end))
                return named(end);
        }
        return "";
    }

    // The Tuning in front of an output, or the Voice processing in front of
    // an input: a filter that puts itself before a device, aimed at it by
    // what it asks a device to be (filter.smart.target), so programs reach
    // the device through it.

    function filtersOf(node) {
        return filters.filter(filter => filter.isSink === node.isSink
            && Sounds.aims(Sounds.target(filter.properties["filter.smart.target"] ?? ""),
                node.properties));
    }

    // Whether sound is passing between a filter and its device now: its
    // other half, a stream sharing its link group, linked and running.
    function flowing(filter, node) {
        const group = filter.properties["node.link-group"];
        return !!group && Pipewire.linkGroups.values.some(link => {
            const half = node.isSink ? link.source : link.target;
            const end = node.isSink ? link.target : link.source;
            return end === node && !!half && half.properties["node.link-group"] === group
                && link.state === PwLinkState.Active;
        });
    }

    function filterPart(node) {
        const found = filtersOf(node);
        if (found.length === 0)
            return null;
        const id = node.isSink ? "tuning" : "voice";
        const key = `sound:${node.name}:${id}`;
        return thingRow({
            key: key,
            name: Words.name(`sound:${id}`),
            side: found.some(filter => provider.flowing(filter, node)) ? said("running")
                : said("on"),
            glyph: glyph(kind(node)),
            note: said(id)
        }, () => found.filter(filter => !!filter.audio).reduce((rows, filter) => rows.concat([
            provider.leafRow({
                key: `${key}:${filter.name}`,
                name: Words.name("sound:filter"),
                side: provider.named(filter),
                note: provider.said(provider.flowing(filter, node) ? "running" : "waiting")
            }),
            provider.figureRow({
                key: `${key}:${filter.name}:level`,
                name: Words.name(node.isSink ? "sound:volume" : "sound:gain"),
                side: provider.level(filter),
                graph: provider.loudness(filter),
                note: filter.audio ? Sounds.decibels(filter.audio.volume) : ""
            })
        ]), []));
    }

    // Its Channels, each with its own volume.
    function channelsPart(node) {
        const channels = Array.from(node.audio.channels ?? []);
        if (channels.length === 0)
            return null;
        const volumes = Array.from(node.audio.volumes ?? []);
        const count = channels.length;
        const key = `sound:${node.name}:channels`;
        return holderRow({
            key: key,
            name: Words.name("sound:channels"),
            side: count === 1 ? said("mono") : count === 2 ? said("stereo")
                : said("channels", { count: count }),
            glyph: glyph(kind(node))
        }, () => channels.map((channel, i) => provider.figureRow({
            key: `${key}:${i}`,
            name: Sounds.channel(PwAudioChannel.toString(channel)),
            side: provider.percent(volumes[i] ?? 0),
            graph: provider.bar(node.audio.muted ? 0 : volumes[i] ?? 0),
            note: Sounds.decibels(volumes[i] ?? 0)
        })));
    }

    // Its Port, when its card offers it several, as one card's analog
    // output offers its speakers and a headphone jack: the sound daemon
    // serves it inside the device's row, a setting saying the one in use,
    // and inside that each it can be moved to, which the daemon does when
    // asked, from the card it watches, as it hears each jack.
    function portKey(node) {
        return `row:sound:${node.name}:port`;
    }

    // The Port's row as served, then each of its choices', or none for a
    // device with no choice of port.
    function portsServed(node) {
        const part = portKey(node);
        const held = ready.served(part);
        if (!held)
            return [];
        const inside = ready.keys.filter(key => ready.served(key)?.parent === part);
        return [held].concat(inside.map(key => ready.served(key)));
    }

    function portPart(node) {
        return servedPart(node, "port");
    }

    // How a process says it ended of itself (QProcess's NormalExit), rather
    // than being stopped, whose output is cut short.
    readonly property int normalExit: 0

    // The device whose drawer is out, or a program's playing through it or
    // recording from it, by the device's node's name, or "". pw-top runs
    // only while one shows, since it asks the graph for its timing every
    // cycle while it runs.
    readonly property string timedDevice: timedBy(launcher !== null && launcher.open
        && launcher.drawerOut ? String(launcher.selected?.key ?? "") : "", devices)

    // Which of the devices the row by this key times, by its node's name,
    // or "": the device's own, or a program's through it, never its
    // Playing list's.
    function timedBy(key, among) {
        const device = among.find(node => key === `sound:${node.name}`
            || key.startsWith(`sound:${node.name}:program:`));
        return device === undefined ? "" : device.name;
    }

    // How each running node keeps time, by its id, as pw-top's last whole
    // frame said it (Sounds.timing), the frame it's reading now, and the
    // driver last read in it, whose cycle its followers run in. The drawer
    // reads `timings`, so it moves with each frame without the launcher
    // refreshing.
    property var timings: ({})
    property var timing: ({})
    property var driving: null

    // pw-top prints every node each second until it's stopped; a frame
    // opens with its header, so the one before stands whole then. It
    // never flushes, so into a pipe its frames would come in clumps of a
    // few kilobytes; stdbuf has it give each line as it's printed.
    Process {
        running: provider.timedDevice !== ""
        command: [provider.settings.stdbuf ?? "stdbuf", "-oL",
            provider.settings.pwTop ?? "pw-top", "--batch-mode"]
        onRunningChanged: {
            if (!running) {
                provider.timings = ({});
                provider.timing = ({});
                provider.driving = null;
            }
        }
        stdout: SplitParser {
            onRead: line => provider.heardTiming(line)
        }
    }

    function heardTiming(line) {
        if (Sounds.heading(line)) {
            timings = timing;
            timing = ({});
            driving = null;
            return;
        }
        const timed = Sounds.timing(line, driving);
        // A line that isn't a follower's ends its driver's run, so one
        // not running leaves its followers with no cycle, not the last.
        if (!Sounds.follower(line))
            driving = timed;
        if (timed !== null)
            timing[timed.id] = timed;
    }

    // Which device each stream is sent to, by its id, as WirePlumber keeps
    // it in the default metadata: heard while Sound is shown, which costs
    // the graph nothing, since pw-metadata only listens, and set from what
    // a send of the launcher's own did once it's done.
    property var targets: ({})

    // Whether the launcher is in Sound, by its own row or one of its
    // devices' as a match from the top.
    readonly property bool shownHere: launcher !== null && launcher.open
        && (launcher.held ?? []).some(step => {
            const key = String(step.row?.key ?? "");
            return key === "collection:sound" || key.startsWith("sound:");
        })

    Process {
        running: provider.shownHere
        command: [provider.settings.pwMetadata ?? "pw-metadata", "-n", "default", "-m"]
        onRunningChanged: {
            if (!running)
                provider.targets = ({});
        }
        stdout: SplitParser {
            onRead: line => {
                // Only a target's change is news: the rest of the default
                // metadata, as the default's own, refreshes nothing.
                const heard = Targets.heard(provider.targets, line);
                if (heard !== provider.targets)
                    provider.targets = heard;
            }
        }
    }

    // A program's Plays On, or Records From for one recording: the device
    // it's on, stepping in to the default and each device it could go to
    // instead (VKCF3Z, 2026-11-09), where there's another to go to.
    function sentPart(node, program, key) {
        const among = sendable(node);
        if (among.length < 2)
            return null;
        const id = node.isSink ? "plays-on" : "records-from";
        const usual = usualFor(node);
        const choices = () => Targets.choices(among,
            Targets.sentTo(program.streams, provider.targets));
        // What's chosen: the device it's sent to, or the default and the
        // device that is, so a stream sent somewhere is told from one
        // following the default (VKCF3Z, 2026-11-09).
        const pinned = choices().find(choice => choice.chosen && choice.device !== null);
        return holderRow({
            key: `${key}:${id}`,
            name: Words.name(`sound:${id}`),
            side: pinned ? named(pinned.device)
                : Plate.runs([said("the-default"), usual ? named(usual) : ""]),
            glyph: glyph(kind(node)),
            offers: true
        }, () => choices().map(choice => {
            // Each said by its value alone, wearing its own row's mark,
            // the default its device's, as a choice is (design.md).
            const device = choice.device ?? usual;
            // One choice's words for both ways, a device's name being the
            // same word whether sound goes to it or comes from it, and
            // :setlocal output= or input= its command, as the part's own.
            const name = choice.device === null ? provider.said("the-default")
                : provider.named(choice.device);
            const value = choice.device === null ? Targets.byDefault()
                : provider.named(choice.device);
            return provider.act("verb:sound-sent", node.isSink ? "" : "input", {
                key: `verb:sound-${id}:${choice.device?.name ?? ""}`,
                fill: { device: name, value: value },
                glyph: device ? provider.glyph(provider.kind(device)) : provider.glyph(""),
                side: choice.chosen ? provider.said("chosen") : "",
                note: choice.device === null && usual ? provider.named(usual) : "",
                run: () => provider.send(node, program, choice.device)
            });
        }));
    }

    // The devices a program's streams could go to, in Sound's order: the
    // outputs for one playing, the inputs for one recording.
    function sendable(node) {
        return ordered(node.isSink ? outputs : inputs);
    }

    function usualFor(node) {
        return node.isSink ? Pipewire.defaultAudioSink : Pipewire.defaultAudioSource;
    }

    // :setlocal output= on a program playing, or input= on one recording:
    // "default" or a device by its name, as its Plays On's choices send it,
    // where there's another device to go to.
    function sentOption(node, program) {
        if (sendable(node).length < 2)
            return {};
        const option = {
            kind: "choice",
            get values() {
                return [Targets.byDefault()].concat(provider.sendable(node)
                    .map(device => provider.named(device)));
            },
            now: () => {
                const sent = Targets.sentTo(program.streams, provider.targets);
                const device = provider.sendable(node).find(each => Targets.names(each, sent));
                return device ? provider.named(device) : Targets.byDefault();
            },
            set: to => {
                const device = provider.sendable(node).find(each => provider.named(each) === to)
                    ?? null;
                if (device !== null || to === Targets.byDefault())
                    provider.send(node, program, device);
                return "";
            }
        };
        return node.isSink ? { output: option } : { input: option };
    }

    // Sends each of a program's streams to a device, or back to the
    // default, given none, which it then follows wherever it moves, one
    // pw-metadata at a time. Each done is held as sent; one refused says
    // so and sends no more. Then, once the program's heard there, the
    // launcher goes to it on its new device's path.
    Process {
        id: sender
        property var left: []
        property var sending: null
        onExited: (code, status) => provider.sent(code === 0 && status === provider.normalExit)
    }

    function send(node, program, device) {
        const to = device ?? usualFor(node);
        const sends = Targets.sending(settings.pwMetadata ?? "pw-metadata", program.streams,
            device ? device.name : "");
        if (sends.length === 0 || !to)
            return;
        const moving = { program: program.name, to: to, sink: node.isSink,
            ids: sends.map(each => each.id) };
        sender.left = sender.left.concat(sends.map(each => Object.assign(each, { moving })));
        if (to !== node) {
            following = moving;
            followFor.restart();
        }
        sendNext();
    }

    function sendNext() {
        if (sender.running || sender.left.length === 0)
            return;
        sender.sending = sender.left[0];
        sender.command = sender.sending.command;
        sender.left = sender.left.slice(1);
        sender.running = true;
    }

    // What pw-metadata did with a send: done, it's held as sent, so the
    // part the launcher lands on says so before the listener tells it
    // back, without refreshing under the walk there; refused, the move
    // stops and says so.
    function sent(done) {
        const was = sender.sending;
        sender.sending = null;
        if (was === null)
            return;
        if (done) {
            targets = Targets.sentAs(targets, [{ id: was.id }], was.target);
            // Followed, it's drawn where it lands, once it's there; sent
            // where it already plays, its part says so now.
            if (following === was.moving)
                arrived();
            else if (!stillSending(was.moving))
                changed();
            Qt.callLater(sendNext);
            return;
        }
        sender.left = sender.left.filter(each => each.moving !== was.moving);
        unmoved(was.moving);
        Qt.callLater(sendNext);
    }

    function unmoved(moving) {
        if (following === moving) {
            following = null;
            followFor.stop();
        }
        launcher?.operatorSays(said("not-sent", { program: moving.program,
            device: named(moving.to) }));
    }

    // The program just sent, until a stream it sent is heard on the device
    // it went to; WirePlumber links it there a moment later, or never, as
    // when the device went first, which is said.
    property var following: null

    Timer {
        id: followFor
        interval: 3000
        onTriggered: {
            if (provider.following !== null)
                provider.unmoved(provider.following);
        }
    }

    Connections {
        target: Pipewire.linkGroups
        enabled: provider.following !== null

        function onValuesChanged() {
            provider.arrived();
        }
    }

    // Whether a stream it sent is linked there now, by its id, so one of the
    // program's already there doesn't stand for it.
    // Whether any of a move's sends is still to be done, so its part isn't
    // drawn naming where it was.
    function stillSending(moving) {
        return sender.sending?.moving === moving
            || sender.left.some(each => each.moving === moving);
    }

    function arrived() {
        const went = following;
        if (went === null || stillSending(went)
            || !linkedTo(went.to).some(stream => went.ids.includes(String(stream.id))))
            return;
        following = null;
        followFor.stop();
        const device = `sound:${went.to.name}`;
        const program = `${device}:program:${went.program}`;
        const id = went.sink ? "plays-on" : "records-from";
        const list = went.sink ? "programs" : "recorders";
        launcher?.goTo({
            // From the top, through Machine, which holds Sound.
            trail: ["category:computer", "collection:sound", device, `${device}:${list}`, program],
            spot: `${program}:${id}`
        });
        launcher?.operatorTells(said(went.sink ? "sent-to" : "sent-from",
            { program: went.program, device: named(went.to) }));
    }

    // A Bluetooth device's Profile: what its sound is for now, music or
    // calls, said in those words. Its Codec and link are said once, in
    // Bluetooth, which the footer points to. A headset microphone's
    // stand-in has no profile of its own: it says its headset's, and that
    // listening through it, while that's music, switches it to calls.
    function headsetOf(node) {
        const address = Sounds.address(node.name, node.properties);
        if (address === "")
            return null;
        if (node.properties["api.bluez5.profile"])
            return node;
        return nodes.find(each => each.properties["api.bluez5.address"] === address
            && !!each.properties["api.bluez5.profile"]) ?? null;
    }

    function carried(node) {
        const headset = headsetOf(node);
        return headset ? Sounds.carries(headset.properties["api.bluez5.profile"]) : "";
    }

    // Whether listening through it would switch a headset from music to
    // calls, as anything reading its microphone's stand-in does.
    function switches(node) {
        return !!node && !node.isSink && Sounds.standIn(node.properties)
            && carried(node) === "music";
    }

    function profilePart(node) {
        const carries = carried(node);
        if (carries === "")
            return null;
        return figureRow({
            key: `sound:${node.name}:profile`,
            name: Words.name("sound:profile"),
            side: said(carries),
            note: said(switches(node) ? "profile-switches" : "profile-elsewhere")
        });
    }

    // Its Hardware, from what the sound server says of its card: the chip
    // it goes through, as the card's components name them, picked by what
    // the device is, unless desk-facts names it, by its maker too, among
    // the facts after it; then a USB card's ids and its name, and the bits
    // its samples carry. A card built into the machine is named by its
    // driver ("sof-soundwire"), which says nothing, so it isn't.
    readonly property var chipFacts: ["codec", "amplifiers"]

    function hardwarePart(node) {
        const properties = node.properties;
        const told = factsOf(node).some(fact => provider.chipFacts
            .includes(String(fact.name).toLowerCase()));
        const chips = Sounds.components(properties["alsa.components"] ?? "");
        const what = `${properties["device.profile.description"] ?? ""} ${nicknamed(node)}`
            .toLowerCase();
        const key = `sound:${node.name}:hardware`;
        const rows = [];
        const add = (id, side) => {
            if (side !== "")
                rows.push(provider.leafRow({ key: `${key}:${id}`, name: Words.name(`sound:${id}`),
                    side: side }));
        };
        if (!told) {
            if (what.includes("head"))
                add("codec", chips.headset);
            else if (node.isSink && chips.speaker !== "")
                add("amplifiers", chips.amplifiers > 0
                    ? said("amplifiers", { count: chips.amplifiers, chip: chips.speaker })
                    : chips.speaker);
            else if (!node.isSink && chips.microphone !== "")
                add("microphones", chips.digital ? said("digital", { chip: chips.microphone })
                    : chips.microphone);
        }
        add("usb-id", chips.usb);
        if (properties["device.bus"] !== "pci")
            add("card", String(properties["alsa.card_name"] ?? ""));
        const bits = properties["alsa.resolution_bits"];
        if (bits)
            add("resolution", said("bits", { count: bits }));
        if (rows.length === 0)
            return null;
        return holderRow({
            key: key,
            name: Words.name("sound:hardware"),
            side: rows[0].side,
            glyph: glyph(kind(node))
        }, () => rows);
    }

    // In the daemon's order: by name, outputs before inputs, the one chosen
    // where it stands, so choosing one never moves it
    // (RSCH-20261030-GB56HX: choices keep their natural order).
    function ordered(among) {
        return listing.map(name => among.find(node => node.name === name))
            .filter(node => node !== undefined);
    }

    // The daemon's rows, as it serves them, made only while they're wanted
    // (Provider.qml's whileWanted), each made again only as its record
    // changes. Home's count reads the devices listed.
    readonly property var all: whileWanted("all", () => down ? [downRow()] : ready.rows(""))
    onAllChanged: changed()

    // What its spots are made of (Launcher.qml's spotsFrom): each device
    // listed, whether it's the one chosen or would switch a headset to
    // calls, and which programs, filters, ports, channels, profile and facts
    // it has parts for; never a volume, a level or how loud a program
    // plays, which move while nothing else does. Without it, every closing
    // made Sound's 77 to 80 spots again with nothing here changed (8N2CSC,
    // 2026-11-10). The sound daemon takes this over as it serves their
    // parts, by 13 November.
    property string spotsStamp: JSON.stringify(ordered(outputs).concat(ordered(inputs))
        .filter(node => !!node.audio).map(node => [node.name, named(node), chosen(node),
            switches(node), programsOf(node).map(program => program.name),
            filtersOf(node).map(filter => filter.name),
            portsServed(node).slice(1).map(choice => choice.held.port),
            Array.from(node.audio.channels ?? []).length, carried(node),
            factsOf(node).map(fact => fact.name)]))

    // A device's row as it's served now, for Now's Volume and Microphone,
    // which borrow its drawer while Now is held and these rows aren't
    // wanted. Where they're wanted, or for a key no device has, the
    // launcher looks in `all`, which is as it reads now while they are.
    function rowNow(key) {
        if (provider.wanted || down || !key.startsWith("sound:"))
            return undefined;
        const record = `row:sound:${key.slice("sound:".length)}`;
        return ready.served(record) ? ready.rowAt(record) : undefined;
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ score: m.score, grade: m.grade, row: row });
        }
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = Order.sorted(found.filter(f => f.grade === finest),
            (a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }

    // A run, its parts a middle dot apart (VKCF3Z, 2026-11-09).
    function counted() {
        return `${outputs.length} out \u00b7 ${inputs.length} in`;
    }

    examples: ["volume", "volume 40"]

    // How loud a device is, in words.
    function level(node) {
        return !node || !node.audio ? said("none")
            : node.audio.muted ? said("muted") : percent(node.audio.volume);
    }

    // The words it answers whole, and by their start (Provider.qml), as
    // the vocabulary lists them.
    questions: Words.started("sound")

    function answer(typed) {
        const text = typed.trim().toLowerCase().replace(/\?$/, "");
        const asked = /^(?:volume|vol|sound)\s+(?:to\s+)?(\d{1,3})\s*%?$/.exec(text);
        if (asked)
            return set(Math.min(100, Number(asked[1])));
        if (asks(text.replace(/^whats /, "what's ")) === "")
            return null;
        return told();
    }

    // How loud a device is as a bar, empty while it's muted. A loud one
    // wants nothing, so the bar never warns.
    function loudness(node) {
        return bar(audible(node));
    }

    // How loud it's heard, as a share, none while it's muted.
    function audible(node) {
        return !node || !node.audio || node.audio.muted ? 0 : node.audio.volume;
    }

    // A player's drawer, as the programs' Media row has it (Applications'
    // mediaDrawer), or none where no source draws one.
    function playerDrawn(player) {
        const drawing = (launcher?.sources ?? [])
            .find(other => typeof other.mediaDrawer === "function");
        return drawing && player ? drawing.mediaDrawer(player) : null;
    }

    function told() {
        const out = Pipewire.defaultAudioSink;
        const into = Pipewire.defaultAudioSource;
        const rows = [];
        // Each device by name, its level beside it, so a long name is what
        // the row is rather than what gets cut.
        if (out)
            rows.push(copyRow({
                name: named(out),
                side: said("out", { level: level(out) }),
                graph: loudness(out)
            }));
        if (into)
            rows.push(copyRow({
                name: named(into),
                side: said("in", { level: level(into) }),
                graph: loudness(into)
            }));
        // The track playing, as the music mark has it (Music.qml), then
        // each program with a stream open to play through, by the name it
        // gives, and each recording; a stream may be open and silent, so it
        // says only that it's open. The track's drawer is its player's, the
        // one its program's Media row opens, since one thing gets one
        // drawer (VKCF3Z, 2026-11-08).
        if (Music.playing)
            rows.push(copyRow({
                name: Music.artist === "" ? Music.title
                    : `${Music.title} \u00b7 ${Music.artist}`,
                side: said("playing"),
                thing: true,
                drawer: () => provider.playerDrawn(Music.player)
            }));
        const names = list => list.filter(node => provider.own(node))
            .map(node => provider.programOf(node).name)
            .filter((name, i, every) => name && every.indexOf(name) === i);
        const playing = names(streams);
        for (const name of playing)
            rows.push(copyRow({ name: name, side: said("open") }));
        for (const name of names(recorders))
            rows.push(copyRow({ name: name, side: said("recording") }));
        return {
            headline: out ? said("through", { level: level(out), device: named(out) })
                : said("nothing-out"),
            graph: out ? loudness(out) : undefined,
            // The answer wears the output's glyph; its rows, figures, none.
            glyph: out && out.audio ? worn(out) : Theme.glyph.silent,
            rows: rows,
            note: !Music.playing && playing.length === 0 ? said("nothing-playing") : ""
        };
    }

    function set(share) {
        const out = Pipewire.defaultAudioSink;
        if (!out || !out.audio)
            return null;
        return {
            headline: said("set", { device: named(out), level: `${share}%` }),
            verb: Words.command("command:volume").verb,
            glyph: worn(out),
            stays: true,
            run: () => {
                out.audio.volume = share / 100;
                out.audio.muted = false;
            },
            rows: told().rows,
            note: said("now", { level: level(out) })
        };
    }
}
