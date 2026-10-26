// Sound, a collection: the devices sound goes out of and comes in through,
// outputs then inputs, the one in use first in each, every glyph saying
// what kind of device it is, its volume drawn as a bar and said in the
// footer. A jack with nothing plugged into it, as the headphones' while
// none are, isn't listed unless it's the one in use, which says so, or
// what's said of its jack can no longer be trusted to be current. Return
// uses one, or mutes or unmutes the one in use, and the right arrow holds
// its parts, each named first and saying how it stands:
//
// - an output: its Level going out, live while it shows; its Volume, in
//   percent and in decibels, with Louder, Quieter and levels to set a step
//   in; whether it's Muted, Mute and Unmute a step in; the programs
//   Playing through it, each muted or turned a step in; its Tuning, when a
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
// Everything comes from the sound server through Quickshell, so nothing is
// asked and nothing is spawned, but for what a sound card says only to the
// kernel: what it offers and runs at, its clock, USB speed and firmware,
// and what its jacks have plugged in, which desk-facts writes to a file
// (modules/desk-facts), and which the right arrow lists last, Return
// copying one; and a card's ports, which Quickshell doesn't carry, so
// pw-dump is asked for them once a device with several is stepped into. A
// Level is read only while it's on screen, by Quickshell's own peak
// monitor, in the shell's process.
import qs
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "../Fuzzy.js" as Fuzzy
import "Sounds.js" as Sounds

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

    // A display plugged in besides the machine's own panel, which niri gives
    // workspaces of its own. The sound server lists a sound card's display
    // outputs whether or not anything is plugged in, so they wait for one.
    readonly property bool displayed: Object.values(Niri.workspaces)
        .some(workspace => workspace.output && !workspace.output.startsWith("eDP"))

    function display(node) {
        return /hdmi|displayport/i.test(`${node.name} ${node.description}`);
    }

    // Hardware, and the filters that put themselves in front of it, as the
    // speakers' tuning and the MV7's voice do: not a program's stream.
    readonly property var nodes: Pipewire.nodes.values
        .filter(node => !!node.audio && !node.isStream
            && (node.type & PwNodeType.Audio) && (node.isSink || (node.type & PwNodeType.Source)))
    readonly property var filters: nodes.filter(node => node.properties["filter.smart"] === "true")
    readonly property var devices: nodes
        .filter(node => node.properties["filter.smart"] !== "true" && listed(node))
    readonly property var outputs: devices.filter(node => node.isSink)
    readonly property var inputs: devices.filter(node => !node.isSink)

    // Each stream open to play through, which carries the server's sink
    // flag, and each open to record from, which doesn't.
    readonly property var streams: Pipewire.nodes.values
        .filter(node => node.isStream && node.isSink && (node.type & PwNodeType.Audio))
    readonly property var recorders: Pipewire.nodes.values
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

    function inUse(node) {
        return node === (node.isSink ? Pipewire.defaultAudioSink : Pipewire.defaultAudioSource);
    }

    // A device is listed while there's something to hear it through: a jack
    // desk-facts says has nothing plugged in isn't, as the headphones
    // aren't while none are, unless it's the one in use, which says so, or
    // what desk-facts said is no longer current (jacksCurrent), when it's
    // listed rather than hidden. A display's output desk-facts says nothing
    // of waits for a display.
    function listed(node) {
        const told = factsOf(node);
        if (Sounds.plugged(told) !== "")
            return !Sounds.hidden(told, jacksCurrent) || inUse(node);
        return displayed || !display(node);
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
    function named(node) {
        const own = nicknamed(node);
        const called = Sounds.called(node.name);
        return called === "" ? own : Words.name(`sound:called:${called}`, "", { device: own });
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

    function mute(node) {
        node.audio.muted = !node.audio.muted;
    }

    function made(node) {
        const used = inUse(node);
        const muted = node.audio.muted;
        const note = [said("volume", { level: percent(node.audio.volume) })]
            .concat(muted ? [said("muted")] : [],
                Sounds.hidden(factsOf(node), jacksCurrent) ? [said("unplugged")] : [],
                switches(node) ? [said("to-calls")] : [])
            .join(" \u00b7 ");
        return member({
            key: `sound:${node.name}`,
            // Return does what its first act would: use it, or mute or
            // unmute the one in use.
            verb: !used ? Words.verb("verb:sound-use")
                : Words.verb("verb:sound-mute", muted ? "muted" : ""),
            name: named(node),
            glyph: worn(node),
            hue: "machine",
            side: used ? said("in-use") : "",
            graph: loudness(node),
            note: note,
            run: used ? () => provider.mute(node) : () => provider.use(node),
            verbs: () => provider.verbs(node)
        });
    }

    // What desk-facts knows of each card, by its ALSA id and direction; a
    // card it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/sound.json`
        watchChanges: true
        printErrors: false
        onFileChanged: {
            reload();
            writer.reload();
        }
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // Whether what desk-facts said of the jacks is current. It writes the
    // file only when what it says changes, and holds no time, so an old
    // file is no sign of an old fact; the sign is desk-facts itself having
    // stopped without removing it, as it does when stopped, by crashing
    // until systemd gives up on it. So its service's control group is
    // watched, which the kernel says is populated while it runs and which
    // goes when it's stopped, and the jacks are current until it's been
    // gone longer than systemd takes to start it again.
    property bool jacksCurrent: true
    readonly property bool writing: Sounds.populated(writerSaid)
    property string writerSaid: ""
    readonly property string account: String(Quickshell.env("XDG_RUNTIME_DIR") ?? "")
        .split("/").pop()

    onWritingChanged: {
        if (writing)
            jacksCurrent = true;
    }

    FileView {
        id: writer
        path: `/sys/fs/cgroup/user.slice/user-${provider.account}.slice/user@${provider.account}`
            + ".service/app.slice/desk-facts.service/cgroup.events"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.writerSaid = text()
        onLoadFailed: provider.writerSaid = ""
    }

    // desk-facts' service starts it again five seconds after it fails
    // (RestartSec in modules/desk-facts/nixos.nix), so gone for twice that
    // it isn't coming back of itself.
    Timer {
        id: writerGone
        interval: 10000
        running: !provider.writing
        onTriggered: provider.jacksCurrent = false
    }

    // The group, once gone, can come again where nothing watches for it,
    // so it's looked at again each time the launcher opens.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                writer.reload();
        }
    }

    // What the kernel says of a device's card, by its key in the file, or
    // "". A card with one device each way is keyed by its id and direction
    // ("MV7:in"); one with several, as the machine's own has its speakers,
    // headphones and displays, by its device's number too
    // ("sofsoundwire,2:out"), and whichever the file holds is the one.
    function factKey(node) {
        const card = node.properties["alsa.id"];
        if (!card)
            return "";
        const way = node.isSink ? "out" : "in";
        const device = node.properties["alsa.device"] ?? node.properties["api.alsa.pcm.device"];
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

    // A device's right arrow: using it, when it isn't the one in use, then
    // its parts, and what the kernel says of its card.
    function verbs(node) {
        const using = node.isSink ? "" : "input";
        const rows = inUse(node) ? [] : [thing({
            key: "verb:sound-use",
            verb: Words.verb("verb:sound-use", using),
            name: Words.name("verb:sound-use", using),
            glyph: glyph(kind(node)),
            hue: "action",
            note: switches(node) ? said("profile-switches") : "",
            run: () => provider.use(node)
        })];
        const parts = [levelPart(node), volumePart(node), mutePart(node), programsPart(node),
            filterPart(node), portPart(node), channelsPart(node), profilePart(node),
            hardwarePart(node)];
        return rows.concat(parts.filter(part => part !== null), factRows(node));
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

    // The Volume, or a microphone's Gain: a setting, saying how it's set,
    // its choices a step in.

    readonly property real step: 0.05
    readonly property var levels: [0.25, 0.5, 0.75, 1]

    function volumePart(node) {
        const id = node.isSink ? "volume" : "gain";
        return holderRow({
            key: `sound:${node.name}:${id}`,
            name: Words.name(`sound:${id}`),
            side: level(node),
            graph: loudness(node),
            glyph: worn(node),
            note: Sounds.decibels(node.audio.volume),
            offers: true
        }, () => provider.volumeChoices(node));
    }

    // An act on a device, as the vocabulary says it, in the state it's in.
    function act(key, state, fields) {
        return thing(Object.assign({
            key: key,
            verb: Words.verb(key, state),
            name: Words.name(key, state, fields.fill),
            hue: "action"
        }, fields));
    }

    // Louder and Quieter keep the launcher, to be pressed again, and say
    // where it is now; louder is meant to be heard, so it unmutes, as a
    // level set is. Each level says its decibels in the footer.
    function volumeChoices(node) {
        const stepping = by => ({
            glyph: Theme.glyph.volume,
            stays: true,
            state: () => provider.said("now", { level: provider.level(node) }),
            run: () => {
                node.audio.volume = Sounds.stepped(node.audio.volume, by, provider.step);
                if (by > 0)
                    node.audio.muted = false;
            }
        });
        const rows = [act("verb:sound-louder", "", stepping(step)),
            act("verb:sound-quieter", "", stepping(-step))];
        const state = node.isSink ? "" : "input";
        return rows.concat(levels.map(each => act("verb:sound-to", state, {
            key: `verb:sound-to-${percent(each)}`,
            fill: { level: percent(each) },
            glyph: Theme.glyph.volume,
            side: !node.audio.muted && Math.abs(node.audio.volume - each) < 0.005
                ? said("in-use") : "",
            note: Sounds.decibels(each),
            run: () => {
                node.audio.volume = each;
                node.audio.muted = false;
            }
        })));
    }

    // Whether it's Muted: a setting, saying yes or no as Bluetooth's are,
    // Mute and Unmute a step in, the one it is in use. Each keeps the words
    // that found it before ("mute it").
    function mutePart(node) {
        const muted = node.audio.muted;
        return holderRow({
            key: `sound:${node.name}:mute`,
            name: Words.name("sound:mute"),
            side: said(muted ? "yes" : "no"),
            glyph: worn(node),
            offers: true
        }, () => [act("verb:sound-mute", "", {
            glyph: provider.struck(node),
            side: node.audio.muted ? provider.said("in-use") : "",
            run: () => {
                node.audio.muted = true;
            }
        }), act("verb:sound-mute", "muted", {
            key: "verb:sound-unmute",
            glyph: provider.glyph(provider.kind(node)),
            side: node.audio.muted ? "" : provider.said("in-use"),
            run: () => {
                node.audio.muted = false;
            }
        })]);
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
                name: entry ? entry.name : given.charAt(0).toUpperCase() + given.slice(1),
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
        return holderRow({
            key: key,
            name: program.name,
            side: paused ? `${streamLevel(streams)} \u00b7 ${said("paused")}` : streamLevel(streams),
            graph: bar(hushed(streams) ? 0 : loudestOf(streams)),
            glyph: programGlyph(programOf(first).id)
        }, () => {
            const stepping = by => ({
                glyph: Theme.glyph.volume,
                stays: true,
                state: () => provider.said("now", { level: provider.streamLevel(streams) }),
                run: () => provider.turn(streams, by)
            });
            const rows = [act("verb:program:mute", hushed(streams) ? "muted" : "", {
                glyph: Words.glyph("verb:program:mute", hushed(streams) ? "muted" : ""),
                run: () => {
                    const muting = !provider.hushed(streams);
                    for (const stream of streams.filter(each => !!each.audio))
                        stream.audio.muted = muting;
                }
            }), act("verb:program:louder", "", stepping(step)),
            act("verb:program:quieter", "", stepping(-step))];
            const rate = Sounds.rate(first.properties["node.rate"]);
            if (rate !== "")
                rows.push(figureRow({
                    key: `${key}:rate`,
                    name: Words.name("sound:rate"),
                    side: rate
                }));
            // How long it waits, first; how that's asked for, in the footer.
            const asked = Sounds.fraction(first.properties["node.latency"]);
            if (asked)
                rows.push(figureRow({
                    key: `${key}:delay`,
                    name: Words.name("sound:delay"),
                    side: `${asked.ms.toFixed(1)} ms`,
                    note: said("delay-note", {
                        frames: asked.frames,
                        rate: Sounds.kilohertz(asked.hertz)
                    })
                }));
            return rows;
        });
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
        return holderRow({
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
    // output offers its speakers and a headphone jack: a setting, saying
    // the one in use, the others a step in, each saying when nothing's
    // plugged into it. Quickshell carries no card's ports, so pw-dump is
    // asked for them, once a device the sound server says has several is
    // stepped into, and again when a jack or the port changes.
    property var portsOf: ({})

    function routed(node) {
        return !!node && Number(node.properties["device.routes"] ?? 0) > 1
            && node.properties["device.id"] !== undefined;
    }

    readonly property var porting: routed(watched) ? watched : null
    onPortingChanged: readPorts(porting)
    onFactsChanged: readPorts(porting)

    // One read at a time (Sounds.portReads): a read asked while another
    // runs, as a jack changing or a port moved asks one, waits for it, and
    // each run keeps the device it's for, so its output is only that one's.
    property var portReading: null

    function readPorts(node) {
        if (routed(node))
            portStep({ asked: node });
    }

    function portStep(event) {
        const after = Sounds.portReads(portReading, event);
        portReading = after.reads;
        const done = after.take;
        if (done !== null && done.node.audio) {
            const read = Object.assign({}, portsOf);
            read[done.node.name] = Sounds.ports(done.text, done.node.properties,
                done.node.isSink);
            portsOf = read;
            changed();
        }
        if (after.start === null)
            return;
        // Only a card's object id is given to pw-dump; a device whose isn't
        // one isn't read, and the next asked is, if any.
        const card = Sounds.dumpable(after.start.properties["device.id"]);
        if (card === "") {
            portStep({ gaveUp: true });
            return;
        }
        portReader.command = [provider.settings.pwDump ?? "pw-dump", card];
        portReader.running = true;
        portsGiven.restart();
    }

    // A sound server that never answers mustn't leave the reader running,
    // which would leave every later read waiting on it for good: three
    // seconds is many times what pw-dump takes. One that never started
    // ends nothing, so its run is given up here.
    Timer {
        id: portsGiven
        interval: 3000
        onTriggered: {
            if (portReader.running)
                portReader.running = false;
            else
                provider.portStep({ gaveUp: true });
        }
    }

    // How a process says it ended of itself (QProcess's NormalExit), rather
    // than being stopped, whose output is cut short.
    readonly property int normalExit: 0

    Process {
        id: portReader
        onExited: (code, status) => {
            portsGiven.stop();
            provider.portStep({ ended: code === 0 && status === provider.normalExit });
        }
        stdout: StdioCollector {
            id: dump
            onStreamFinished: provider.portStep({ printed: dump.text })
        }
    }

    // Moves a device to a port, by its card's index for it, then reads its
    // ports again to say so.
    Process {
        id: portMover
        property var moving: null
        onExited: provider.readPorts(portMover.moving)
    }

    function movePort(node, port) {
        portMover.moving = node;
        portMover.command = [provider.settings.wpctl ?? "wpctl", "set-route", String(node.id),
            String(port.index)];
        portMover.running = true;
    }

    function portPart(node) {
        const offered = portsOf[node.name] ?? [];
        const now = offered.find(port => port.active) ?? null;
        // A port nothing's plugged into is no choice, unless it's in use.
        const choosable = offered.filter(port => port.active || port.available !== "no");
        if (!routed(node) || now === null || choosable.length < 2)
            return null;
        const mark = glyph(kind(node));
        return holderRow({
            key: `sound:${node.name}:port`,
            name: Words.name("sound:port"),
            side: now.available === "no" ? `${now.name}, ${said("unplugged")}` : now.name,
            glyph: mark,
            offers: true
        }, () => choosable.map(port => provider.act("verb:sound-port", "", {
            key: `verb:sound-port-${port.index}`,
            fill: { port: port.name },
            glyph: mark,
            side: port.active ? provider.said("in-use") : "",
            note: port.available === "no" ? provider.said("unplugged") : "",
            run: () => provider.movePort(node, port)
        })));
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

    // In use first, then by name, outputs before inputs.
    function ordered(nodes) {
        return nodes.slice().sort((a, b) => Number(inUse(b)) - Number(inUse(a))
            || named(a).localeCompare(named(b)));
    }

    // A device going away can be read once more before the lists drop it.
    readonly property var all: ordered(outputs).concat(ordered(inputs))
        .filter(node => !!node.audio).map(node => made(node))
    onAllChanged: changed()

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
        const kept = found.filter(f => f.grade === finest).sort((a, b) => b.score - a.score);
        return { rows: kept.map(f => f.row), grade: finest };
    }

    function everything() {
        return all.slice();
    }

    function counted() {
        return `${outputs.length} out, ${inputs.length} in`;
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
        return bar(!node || !node.audio || node.audio.muted ? 0 : node.audio.volume);
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
        // says only that it's open.
        if (Music.playing)
            rows.push(copyRow({
                name: Music.artist === "" ? Music.title : `${Music.title}, ${Music.artist}`,
                side: said("playing")
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
            run: () => {
                out.audio.volume = share / 100;
                out.audio.muted = false;
            },
            rows: told().rows,
            note: said("now", { level: level(out) })
        };
    }
}
