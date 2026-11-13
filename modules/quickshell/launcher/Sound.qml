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
//   from it for those Playing, its Voice processing for a Tuning; then its
//   Noise Cancelling, the cleaner the microphones run now, and Choose by
//   Ear, the blind trial picking it: Record, then clips A to D, each
//   played by Return with Pick a step in, then every clip's cleaner and
//   its cost revealed (ByEar.js works each step out, and NoiseCancelling
//   holds the trial). Leaving it, by Escape or by closing, discards it.
//   Beneath Record, Name This Spot takes Joel's name for where he is,
//   whose picks are kept under it. Record records the input it's under.
//
// After a call on a microphone at a spot Joel's ear hasn't chosen for it
// there, For You holds one thing to do, Choose by Ear for it, while it's
// plugged in (forYou); taking it or saying No Thanks is the last of it.
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
// called, the glyph it wears, its bar and its note, in its order; its
// Use, Muted, Volume and Port parts; and where each program's stream is
// sent, a send being an ask of it. Their other parts and drawers are
// still made here, from the sound server through
// Quickshell, until the daemon serves them too by 13 November, so nothing
// is spawned for them, but for what a sound card says only to the kernel:
// what it offers and runs at, its clock, USB speed and firmware, which
// desk-facts writes to a file (modules/desk-facts), and which the right
// arrow lists last, Return copying one. A Level is read only while it's on
// screen, by Quickshell's own peak monitor, in the shell's process.
import qs
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Ago.js" as Ago
import "ByEar.js" as ByEar
import "ForYou.js" as ForYou
import "Order.js" as Order
import "ReadyRows.js" as Ready
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
        // Where each program's stream is sent, as one record, and each
        // program's drawer, which its row names.
        beside: ["drawer", "targets"]
        // A device's row is made with its parts, a program's with what's
        // still made here of it, and a part the daemon serves inside one, as
        // its Volume, as every place's rows are (made).
        make: (row, key) => provider.made(row, key)
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
            if (provider.following !== null)
                provider.arrived();
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
    readonly property var devices: nodes
        .filter(node => node.properties["filter.smart"] !== "true" && listed(node))
    readonly property var outputs: devices.filter(node => node.isSink)
    readonly property var inputs: devices.filter(node => !node.isSink)

    // :set mute and :set volume=40 act on the output sound plays through,
    // as its rows do, a volume set unmuting it as their levels do, each an
    // ask of the daemon.
    setOptions: ({
        mute: {
            kind: "switch",
            now: () => Pipewire.defaultAudioSink?.audio?.muted ?? false,
            set: to => provider.toOut(out => provider.deviceAsked(out, "mute", { muted: to }))
        },
        volume: {
            kind: "number",
            least: 0,
            most: 100,
            now: () => Math.round((Pipewire.defaultAudioSink?.audio?.volume ?? 0) * 100),
            set: to => provider.toOut(out => provider.deviceAsked(out, "volume",
                { volume: to / 100, muted: false }))
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
    // A noise-cancelled microphone is the one it hears, named as that one's
    // row and said Noise Cancelled once, as the sound daemon names its row
    // (modules/sound's rows.rs); its own description says so already, for
    // call programs, so it's named by its own only once that one's gone.
    function named(node) {
        const heard = String(node.name).replace(/^noise-cancelled\./, "");
        if (heard !== node.name) {
            const microphone = inputs.find(input => input.name === heard);
            return microphone ? Words.name("sound:called:noise-cancelled", "",
                { device: named(microphone) }) : nicknamed(node);
        }
        const own = nicknamed(node);
        const shared = inputs.includes(node) && outputs.some(output => nicknamed(output) === own);
        const called = Sounds.called(heard, shared);
        return called === "" ? own
            : Words.name(`sound:called:${called}`, "", { device: own });
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

    // Using a device, and setting its Muted or Volume, are asked of the
    // sound daemon, as its parts' acts are, so the shell sets nothing on
    // the sound server itself (modules/sound's acts.rs).
    function deviceAsked(node, act, more) {
        ready.link?.ask(Object.assign({ ask: "act", act: act, key: `device:${node.name}` },
            more ?? {}), () => {});
        return "";
    }

    function use(node) {
        deviceAsked(node, "use");
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
    // row). Its parts are made here from its node as they're asked for,
    // until the daemon serves them too; its drawer is the one served beside
    // it (device_drawer.rs), which its row names.
    function servedRow(row, key) {
        const inside = () => {
            const node = provider.nodeOf(row);
            return node?.audio ? provider.verbs(node) : [];
        };
        const made = ready.thing(row, key, {
            thing: true,
            verbs: inside,
            targets: inside
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
    // so the next ask starts it again, as its drawer says, beside what it
    // serves and its service, the name its log is read by; and Return
    // has the machine daemon start it, for when its socket has gone too,
    // as Spend's down row starts its watch.
    function downRow() {
        const fact = (id, value) => ({ label: Words.name(id), value: value });
        return thing({
            key: "sound:down",
            name: Words.name("sound:down"),
            note: said("down"),
            glyph: Theme.glyph.caution,
            hue: "warning",
            verb: Words.verb("sound:down"),
            stays: true,
            run: () => Watchers.act({ act: "restart-unit", unit: "sound.service" },
                answer => provider.startRefused(Words.name("sound:down"), answer)),
            drawer: {
                eyebrow: said("joined-here"),
                title: Words.name("sound:down"),
                facts: [fact("sound:state", said("down")),
                    fact("sound:down:serves", said("down-serves")),
                    Object.assign(fact("sound:down:service", "sound.service"),
                        { identifies: true }),
                    fact("sound:down:starts", said("down-starts"))],
                body: ""
            }
        });
    }

    // A device's own options, and a program's playing through it, which
    // :setl sets as Vim's :setlocal sets a buffer's (VKCF3Z, 2026-11-05):
    // whether it's muted, how loud, and a device's port where it has a
    // choice of them, on it alone, never the default output :set's are
    // (setOptions), each an ask of the daemon. A level set or raised is
    // meant to be heard, so it unmutes, as its rows' do.
    function nodeOptions(nodes) {
        const live = () => nodes.filter(node => !!node.audio);
        return {
            mute: {
                kind: "switch",
                now: () => provider.hushed(nodes),
                set: to => {
                    for (const node of live())
                        provider.deviceAsked(node, "mute", { muted: to });
                    return "";
                }
            },
            volume: {
                kind: "number",
                least: 0,
                most: 100,
                now: () => Math.round(provider.loudestOf(nodes) * 100),
                set: (to, change) => {
                    const more = change?.how === "subtract" ? { volume: to / 100 }
                        : { volume: to / 100, muted: false };
                    for (const node of live())
                        provider.deviceAsked(node, "volume", more);
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

    // What desk-facts says of a device's card, as the sound daemon serves
    // it last inside the device's row (desk.rs), so the shell reads no
    // file for it: the keys of those rows, in the daemon's order.
    function factKeys(node) {
        const parent = `row:sound:${node.name}`;
        return ready.keys.filter(key => {
            const row = ready.served(key);
            return row?.parent === parent && row.kind === "fact";
        });
    }

    // Each fact by its own name and what it says, as the Hardware and the
    // drawer ask them.
    function factsOf(node) {
        return factKeys(node).map(key => ready.served(key)).map(row => ({
            name: row.held?.fact ?? "", value: row.side?.[0]?.said ?? ""
        }));
    }

    function factRows(node) {
        return factKeys(node).map(key => ready.rowAt(key));
    }

    // A device's right arrow, kept while the launcher is shut, as its
    // spots ask for it again a minute after each closing (Launcher.qml's
    // spotsFreshened): it made Sound's 111 parts again each time with
    // nothing here changed (Y2YBTS, 2026-11-10). Each part made here is
    // kept while what it says is as it was, so what moves makes only its
    // own part again. Kept whole, a device's parts were all made again as
    // it began or stopped running, which gives its node a period and a
    // latency no part says and its card a fact: a call starting made 107
    // of them (2026-11-11, 15:03). Those the daemon serves are made once
    // as each of their records changes (ReadyRows.qml's rowAt). Shown,
    // they're made as they're asked for, as they read.
    readonly property var partsKeeping: keeper()
    function verbs(node) {
        if (provider.wanted)
            return partsMade(node);
        const parts = partsMade(node, (part, from, make) => keptAlike(partsKeeping,
            `sound:${node.name}:${part}`, [node, JSON.stringify(from)], make));
        return keptAlike(partsKeeping, `sound:${node.name}`, parts, () => parts);
    }

    // A part the daemon serves inside a device's row, by the last of its
    // key, as it's made, or null while it serves none.
    function servedPart(node, part) {
        const key = `row:sound:${node.name}:${part}`;
        return ready.served(key) ? ready.rowAt(key) : null;
    }

    // A served row as Sound draws it: a device's with its parts; a
    // program's with its drawer and :setl options; a Plays On choice sent
    // as Sound's own send sends it, so the launcher follows the program to
    // where it lands; any other as the daemon serves it.
    function made(row, key) {
        switch (row.kind) {
        case "device":
            return servedRow(row, key);
        case "program":
            return programRow(row, key);
        case "send":
            return ready.thing(row, key, Object.assign(partFields(row), {
                run: () => provider.sendServed(ready.served(key) ?? row)
            }));
        default:
            return ready.thing(row, key, partFields(row));
        }
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
    // kernel says of its card. Every part but its Level is served; the
    // Level is made here, from the peaks the shell reads, by `kept` (verbs)
    // where it's given, from what it says.
    function partsMade(node, kept) {
        const made = kept ?? ((part, from, make) => make());
        const programs = node.isSink ? "programs" : "recorders";
        const parts = [servedPart(node, "use"), servedPart(node, "mute"),
            made("level", [switches(node), node === metered ? heard : []],
                () => levelPart(node)),
            volumePart(node), servedPart(node, programs),
            filterPart(node),
            made("noise", noiseSaid(node), () => noisePart(node)),
            made("by-ear", noiseSaid(node), () => byEarPart(node)), portPart(node),
            servedPart(node, "channels"),
            servedPart(node, "profile"),
            servedPart(node, "hardware")];
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

    // The programs Playing through an output, or Recording from an input,
    // are served: each program's row, its Muted, Louder and Quieter, its
    // Plays On, its figures and its drawer, timings and all. Its :setl
    // options are made here until the daemon takes them as asks.

    // A program's row as the daemon serves it, its drawer the one the
    // daemon serves beside it, with its options, read from the row as it's
    // served when they're asked for, so they're never made again as it
    // moves.
    function programRow(row, key) {
        const held = () => (ready.served(key) ?? row).held;
        return gotten(ready.thing(row, key),
            { options: () => provider.programOptions(held()) });
    }

    // A program as a send takes one: its name, and its streams by their
    // ids, as its served row holds them.
    function programHeld(held) {
        return { name: held?.program ?? "", streams: (held?.streams ?? []).map(id => ({ id })) };
    }

    function deviceNamed(name) {
        return nodes.find(node => node.name === name) ?? null;
    }

    // A program's own options, which :setl sets as Vim's :setlocal sets a
    // buffer's (VKCF3Z, 2026-11-05): whether it's muted, how loud, and where
    // it plays, each an ask of the daemon on every one of its streams. A
    // level set or raised is meant to be heard, so it unmutes, as its
    // Louder does.
    function programOptions(held) {
        const node = deviceNamed(held?.node);
        const link = ready.link;
        if (node === null || !link)
            return {};
        const asked = (act, more) => {
            link.ask(Object.assign({ ask: "act", act: act,
                key: `program:${held.node}:${held.program}`, streams: held.streams }, more),
                () => {});
            return "";
        };
        return Object.assign({
            mute: {
                kind: "switch",
                now: () => held.muted === true,
                set: to => asked("mute", { muted: to })
            },
            volume: {
                kind: "number",
                least: 0,
                most: 100,
                now: () => Math.round((held.volume ?? 0) * 100),
                set: (to, change) => asked("volume", change?.how === "subtract"
                    ? { volume: to / 100 } : { volume: to / 100, muted: false })
            }
        }, sentOption(node, programHeld(held)));
    }

    // A Plays On choice's Return: the program's streams sent there, or to
    // the default, as its option sends them, so the launcher follows it.
    function sendServed(row) {
        const held = row.held;
        const node = deviceNamed(held?.node);
        const to = held?.to ? deviceNamed(held.to) : null;
        if (node !== null && (to !== null || !held.to))
            send(node, programHeld(held), to);
    }

    // A program's name as its desktop entry says it, looked up once each,
    // or the name it gives, for Now's list of what's open; the daemon names
    // Sound's own programs.
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

    function loudestOf(streams) {
        return Math.max(0, ...streams.filter(stream => !!stream.audio)
            .map(stream => stream.audio.volume));
    }

    function hushed(streams) {
        return streams.every(stream => !stream.audio || stream.audio.muted);
    }

    // The Tuning in front of an output, or the Voice Processing in front of
    // an input: the filters aimed at it, whose part, and whether sound
    // passes through them now, the sound daemon serves (filters.rs).
    function filterPart(node) {
        return servedPart(node, node.isSink ? "tuning" : "voice");
    }

    // What the served filters part holds: each filter's name, for the
    // spots, and each said with its level, for the drawer.
    function filtersHeld(node) {
        const part = `row:sound:${node.name}:${node.isSink ? "tuning" : "voice"}`;
        return ready.served(part)?.held ?? { filters: [], levels: [] };
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

    // The device whose drawer is out, or a program's playing through it or
    // recording from it, by the device's node's name, or "". How nodes keep
    // time is watched only while one shows, since the sound server measures
    // every cycle while it's watched.
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

    // How each running node keeps time, watched while one's drawer is out,
    // as PipeWire's profiler times it only while something watches (modules/
    // sound's timing.rs); the daemon says it in the drawers it serves.
    Daemon {
        id: timing

        socket: ready.socket
        kind: "timing"
        live: provider.timedDevice !== ""
        // The daemon forgets each node's time once nothing watches it, so
        // a watch's answer is all there is, and none stands in a gap.
        answersWhole: true
        keptThroughGap: false
    }

    // Which device each stream is sent to, by its id, as WirePlumber keeps
    // it in the default metadata and the sound daemon serves it, told of
    // each change, a send of the launcher's own included.
    readonly property var targets: {
        void ready.placings;
        return ready.held["targets:default"]?.targets ?? ({});
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
    // default, given none, which it then follows wherever it moves, each
    // an ask of the sound daemon, which says it in the default metadata.
    // One refused says so, and the rest are let be. Then, once the
    // program's heard there, the launcher goes to it on its new device's
    // path.
    function send(node, program, device) {
        const to = device ?? usualFor(node);
        const ids = Targets.sendable(program.streams);
        const link = ready.link;
        if (ids.length === 0 || !to || !link)
            return;
        const moving = { program: program.name, to: to, sink: node.isSink, ids: ids,
            left: ids.length, refused: false };
        if (to !== node) {
            following = moving;
            followFor.restart();
        }
        for (const id of ids) {
            link.ask({ ask: "act", act: "send", key: `stream:${id}`,
                target: device ? device.name : "" }, answer => provider.sent(moving,
                answer?.done === true));
        }
    }

    // What the daemon did with one stream's send: once each is done, the
    // program is drawn where it lands, followed, or its part says so where
    // it already plays; refused, the move stops and says so, once.
    function sent(moving, done) {
        if (moving.refused)
            return;
        if (!done) {
            moving.refused = true;
            unmoved(moving);
            return;
        }
        moving.left -= 1;
        if (moving.left > 0)
            return;
        if (following === moving)
            arrived();
        else
            changed();
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

    // Whether any of a move's sends is still to be done, so its part isn't
    // drawn naming where it was.
    function stillSending(moving) {
        return moving.left > 0 && !moving.refused;
    }

    // Once the daemon serves the program on the device it went to, holding
    // a stream it sent, by its id, so one of the program's already there
    // doesn't stand for it, the launcher goes to it there.
    function arrived() {
        const went = following;
        const list = went?.sink ? "programs" : "recorders";
        const there = went === null ? null
            : ready.served(`row:sound:${went.to.name}:${list}:program:${went.program}`);
        if (went === null || stillSending(went)
            || !(there?.held?.streams ?? []).some(id => went.ids.includes(String(id))))
            return;
        following = null;
        followFor.stop();
        const device = `sound:${went.to.name}`;
        const program = `${device}:program:${went.program}`;
        const id = went.sink ? "plays-on" : "records-from";
        launcher?.goTo({
            // From the top, through Machine, which holds Sound.
            trail: ["category:computer", "collection:sound", device, `${device}:${list}`, program],
            spot: `${program}:${id}`
        });
        launcher?.operatorTells(said(went.sink ? "sent-to" : "sent-from",
            { program: went.program, device: named(went.to) }));
    }

    // A Bluetooth device's headset, whose profile says what its sound is
    // for now, music or calls. A headset microphone's stand-in has no
    // profile of its own: it says its headset's. The daemon serves the
    // Profile part; Level and spots still read it here.
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

    // Noise Cancelling and Choose by Ear, on each input but a noise-cancelled
    // one, which is a cleaner's already: what the trial daemon says of them
    // (NoiseCancelling.qml), each step worked out by ByEar.js and drawn here.
    readonly property var soundSaid: Words.of("sound").said ?? {}

    function cancels(node) {
        return !node.isSink && !String(node.name).startsWith("noise-cancelled.");
    }

    // What the two parts say, by which they're made again.
    function noiseSaid(node) {
        const record = NoiseCancelling.record;
        return cancels(node) ? [record?.running ?? null, record?.stage ?? null,
            record?.picked?.clip ?? null, NoiseCancelling.state] : [];
    }

    function noisePart(node) {
        if (!cancels(node))
            return null;
        const record = NoiseCancelling.record;
        const key = `sound:${node.name}:noise`;
        const side = ByEar.running(soundSaid, record, NoiseCancelling.state);
        // A daemon not answering has no cleaner to copy, so Return starts it.
        if (record === null)
            return provider.startsRow(key, Words.name("sound:noise"), side, said("noise"));
        return figureRow({ key: key, name: Words.name("sound:noise"), side: side,
            note: said("noise") });
    }

    // A row saying noise-cancelled isn't answering, whose Return starts it,
    // staying open to say so once it answers. Its socket is mostly what's
    // gone, stopped or failed, so the machine daemon restarts its trial's
    // service, which listens itself when handed no socket; then the shell
    // asks it again rather than waiting out its next try.
    function startsRow(key, name, side, note) {
        return thing({
            key: key,
            name: name,
            verb: Words.verb("sound:by-ear:down"),
            hue: "action",
            stays: true,
            side: side,
            note: note,
            run: () => Watchers.act(
                { act: "restart-unit", unit: "noise-cancelled-trial.service" }, answer => {
                    if (!provider.startRefused(name, answer))
                        NoiseCancelling.start();
                })
        });
    }

    // Says a start the machine daemon refused in its words, or that it had
    // no answer, as ReadyRows' shut does, and whether it did. A row that
    // starts a daemon holds nothing drawn, so `refused` would have no hold
    // to take back, and said nothing at all (VKCF3Z, 2026-11-13).
    function startRefused(name, answer) {
        const why = Ready.refusal(answer, Words.field.becoming ?? {});
        if (why !== "")
            launcher?.toldNot(name, "start", why);
        return why !== "";
    }

    // Choose by Ear: Return steps into the trial, saying how it stands.
    function byEarPart(node) {
        if (!cancels(node))
            return null;
        const row = entering(thing({
            key: `sound:${node.name}:by-ear`,
            name: Words.name("sound:by-ear"),
            glyph: Words.glyph("sound:by-ear"),
            hue: "action",
            side: ByEar.standing(soundSaid, NoiseCancelling.record),
            note: said("by-ear")
        }), () => provider.byEarRows(node));
        row.verb = Words.verb("sound:by-ear");
        return row;
    }

    function byEarRows(node) {
        return ByEar.rows(soundSaid, NoiseCancelling.record, NoiseCancelling.state, Date.now())
            .map(row => provider.byEarRow(ByEar.aimed(row, node.name)));
    }

    // For You's one thing to do while the daemon offers Choose by Ear, after
    // a call on a microphone at a spot Joel's ear hasn't chosen for it there
    // (noise-cancelled's offer.rs), as Now gathers each source's (ForYou.js's
    // given), and only while that microphone is plugged in. Return steps
    // into its Choose by Ear, and a step in holds No Thanks; either is the
    // last of it, since the daemon offers a microphone at a spot once. Made
    // again only as the offer or the inputs change, never with a trial's
    // steps, which change the record many times a second.
    readonly property string offerSaid: {
        const offered = ByEar.offer(NoiseCancelling.record);
        const here = offered !== null && inputs.some(node => node.name === offered.node);
        return JSON.stringify(here ? offered : null);
    }
    readonly property var forYou: {
        const offered = JSON.parse(offerSaid);
        return offered === null ? [] : [{
            row: provider.offerRow(offered),
            thing: { kind: "todo", ready: true, takes: 1, since: offered.since ?? null }
        }];
    }

    // The offer's row: the microphone it's for, where at its side, Return
    // going to that microphone's Choose by Ear, ready to record.
    function offerRow(offered) {
        const words = "sound:offer";
        const device = `sound:${offered.node}`;
        const noThanks = thing({
            key: `${words}:no-thanks`,
            name: Words.name(`${words}:no-thanks`),
            verb: Words.verb(`${words}:no-thanks`),
            glyph: Words.glyph(`${words}:no-thanks`),
            hue: "action",
            run: () => NoiseCancelling.act(ByEar.taking(), null)
        });
        return thing({
            key: words,
            what: "on",
            // What it is and where it's from, which For You filters by.
            forYou: { kind: "do", source: "sound" },
            name: Words.name(words, "", { microphone: offered.microphone }),
            verb: Words.verb(words),
            glyph: Words.glyph(words),
            hue: "strong",
            side: offered.spot,
            stays: true,
            drawer: () => provider.offerDrawer(offered),
            verbs: () => [noThanks],
            targets: () => [noThanks],
            run: () => {
                provider.launcher?.goTo({
                    // From the top, through Machine, which holds Sound.
                    trail: ["category:computer", "collection:sound", device, `${device}:by-ear`],
                    spot: "sound:by-ear:now"
                });
                NoiseCancelling.act(ByEar.taking(), null);
            }
        });
    }

    // The offer's drawer, in the facts every For You drawer gives (Now.qml's
    // drawerOf): where it's from and that it's a thing to do, what it
    // weighs, how long it takes, since when it has waited and what clears
    // it; its words say why it's there, then what to do.
    function offerDrawer(offered) {
        const now = Date.now();
        const say = Object.assign({}, Words.of("on:waiting").said ?? {}, {
            clock: at => provider.clocked(at),
            ago: at => Ago.ago(at, now)
        });
        const weighed = { kind: "todo", ready: true, takes: 1, since: offered.since ?? null };
        const fact = (id, value) => ({ label: Words.name(id), value: String(value) });
        const offerWords = Words.of("sound:offer");
        const tells = offerWords.tells ?? {};
        const facts = [
            fact("on:waiting:weight", ForYou.weightSaid(weighed, now, say)),
            fact("on:waiting:takes", ForYou.filled(say.takes, { minutes: weighed.takes })),
            fact("on:waiting:since", weighed.since === null ? ""
                : provider.clocked(weighed.since)),
            fact("on:waiting:clears", offerWords.said?.clears ?? "")
        ].filter(one => one.value !== "");
        return {
            eyebrow: ForYou.filled(say.from, { from: Words.name("sound"), kind: say.todo }),
            eyebrowNames: true,
            title: Words.name("sound:offer", "", { microphone: offered.microphone }),
            facts: facts,
            body: [ByEar.offerWhy(tells, offered), tells.steps ?? ""]
                .filter(one => one !== "").join("\n\n")
        };
    }

    // A time as the clock says it: today's by its hour, and another day's
    // with its day, as For You's own are.
    function clocked(at) {
        const when = new Date(at);
        const today = new Date().toDateString() === when.toDateString();
        return Qt.formatDateTime(when, today ? "HH:mm" : "ddd d MMM, HH:mm");
    }

    // A step's row as ByEar.js lays it: an act asking the daemon on
    // Return, a clip holding Pick a step in, or a figure, whose Return
    // only closes, since what it says is no use pasted.
    function byEarRow(row) {
        const state = row.state ?? "";
        const name = Words.name(row.id, state, row.fill);
        if (row.names)
            return provider.spotRow(row, name);
        if (row.starts)
            return provider.startsRow(row.key, name, row.side, row.note);
        if (!row.act)
            return figureRow({
                key: row.key,
                name: name,
                side: row.side,
                note: row.note,
                copiesNothing: true
            });
        const made = thing({
            key: row.key,
            name: name,
            verb: Words.verb(row.id, state),
            glyph: Words.glyph(row.id, state),
            hue: "action",
            stays: true,
            side: row.side ?? "",
            note: row.note ?? "",
            run: () => provider.byEarAsk(row)
        });
        if (row.pick)
            made.verbs = () => [provider.byEarRow(row.pick)];
        return made;
    }

    // How naming the spot last went, for the foot under what's typed: the
    // daemon's plain reason when it refused, that it isn't answering when
    // it's gone, else nothing.
    property string spotTold: ""

    // Where Joel is: Return asks him what he calls it, in the field, and
    // hands it to the daemon. A name taken steps back out to the trial,
    // which then says it; one refused stays asked, the foot saying why.
    function spotRow(row, name) {
        const state = row.state ?? "";
        return thing({
            key: row.key,
            name: name,
            verb: Words.verb(row.id, state),
            glyph: Words.glyph(row.id, state),
            hue: "action",
            side: row.side ?? "",
            note: row.note ?? "",
            ask: Words.of(row.id, state).ask ?? "",
            chip: row.side ?? "",
            state: () => provider.spotTold,
            given: words => {
                provider.spotTold = "";
                NoiseCancelling.act(ByEar.naming(words), why => {
                    if (why === null)
                        provider.spotTold = soundSaid.down ?? "";
                    else if (why)
                        provider.spotTold = why;
                    else
                        launcher?.back?.();
                });
            },
            stays: true
        });
    }

    // Asks the daemon what Return on a row does; a refusal is said in its
    // plain words, and what follows is the record's to say. A Pick steps
    // back out to the trial, whose clips it drops.
    function byEarAsk(row) {
        const asking = ByEar.asked(row);
        if (asking !== null)
            NoiseCancelling.act(asking, why => {
                if (why)
                    provider.refused(row.key, why);
            });
        if (row.back)
            launcher?.back?.();
    }

    // Whether Choose by Ear's step is held, so leaving it, by Escape or by
    // closing the launcher, discards the trial (ByEar.leaving).
    readonly property bool choosing: {
        const steps = launcher && launcher.open ? launcher.held ?? [] : [];
        return steps.some(step => String(step?.row?.key ?? "").endsWith(":by-ear"));
    }
    onChoosingChanged: {
        const asking = choosing ? null : ByEar.leaving(NoiseCancelling.record);
        if (asking !== null)
            NoiseCancelling.act(asking, null);
    }

    Connections {
        target: NoiseCancelling

        function onHeard() {
            provider.changed();
        }
    }

    // A recording's count, a quarter second at a time, so the second it
    // shows turns over within a frame or so of the clock's; only while the
    // trial's step is held.
    Timer {
        interval: 250
        repeat: true
        running: provider.choosing && NoiseCancelling.record?.stage === "recording"
        onTriggered: provider.changed()
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
            switches(node), ready.served(`row:sound:${node.name}:programs`)?.side ?? [],
            filtersHeld(node).filters,
            portsServed(node).slice(1).map(choice => choice.held.port),
            Array.from(node.audio.channels ?? []).length, carried(node),
            factsOf(node).map(fact => fact.name)]))

    // And what one device's spots are made of, so a change to one device
    // makes only its spots again as the launcher closes (Launcher.qml's
    // heldSpots): its row and each part served inside it by key, name,
    // mark and hue, and what spotsStamp says of its node; never a volume
    // or a level. Joel's closing at 14:36 on 2026-11-11 made all 111 of
    // Sound's rows again for one device's change. Null for a row not made
    // of a device now, so it's made again. The daemon serving every part
    // ready, by 13 November, ends it.
    function spotsStampOf(thing) {
        if (ready.stampOf(thing) === null)
            return null;
        const key = ready.madeOf[thing.key];
        const row = ready.served(key);
        const node = nodeOf(row);
        if (!node?.audio)
            return null;
        const steady = one => [one.name ?? "", one.glyph ?? "", one.hue ?? ""];
        const parts = ready.keys.filter(one => ready.served(one)?.parent === key)
            .map(one => [one].concat(steady(ready.served(one))));
        return JSON.stringify([key, steady(row), parts, named(node), kind(node), chosen(node),
            switches(node), ready.served(`${key}:programs`)?.side ?? [],
            filtersHeld(node).filters,
            Array.from(node.audio.channels ?? []).length, carried(node),
            factsOf(node).map(fact => fact.name)]);
    }

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

    // Choose by Ear for each microphone noise cancelling cleans, found by
    // its name from the top though it sits two steps inside the
    // microphone, and worded as For You's offer words it (VKCF3Z's (d),
    // 2026-11-12). Searched, never listed, since Sound lists devices. Made
    // again only as which microphones there are, or their names, change,
    // never as a volume or a level moves (tests/sound-served.qml).
    readonly property string byEarFoundSaid: JSON.stringify(byEarFoundFrom(ordered(inputs)))
    readonly property var byEarFound: whileWanted("byEarFound",
        () => JSON.parse(byEarFoundSaid).map(([node, called]) => byEarFoundRow(node, called)))

    // Each microphone it cleans among `nodes`, by its node's name and its own.
    function byEarFoundFrom(nodes) {
        return nodes.filter(node => cancels(node)).map(node => [node.name, named(node)]);
    }

    function byEarFoundOf(nodes) {
        return byEarFoundFrom(nodes).map(([node, called]) => byEarFoundRow(node, called));
    }

    function byEarFoundRow(node, called) {
        const device = `sound:${node}`;
        return thing({
            key: `sound:by-ear:for:${node}`,
            name: Words.name("sound:by-ear:for", "", { microphone: called }),
            verb: Words.verb("sound:by-ear"),
            glyph: Words.glyph("sound:by-ear"),
            hue: "action",
            stays: true,
            note: said("by-ear"),
            // As For You's offer goes, from the top through Machine, though
            // nothing here takes an offer.
            run: () => provider.launcher?.goTo({
                trail: ["category:computer", "collection:sound", device, `${device}:by-ear`],
                spot: "sound:by-ear:now"
            })
        });
    }

    // A search looks through Choose by Ear's rows too.
    function looksThrough() {
        const was = lookingThrough.now;
        lookingThrough.now = true;
        try {
            return everything().concat(standing("byEarFound", byEarFound));
        } finally {
            lookingThrough.now = was;
        }
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all.concat(byEarFound)) {
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
        return standing("all", all).slice();
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
            run: () => deviceAsked(out, "volume", { volume: share / 100, muted: false }),
            rows: told().rows,
            note: said("now", { level: level(out) })
        };
    }
}
