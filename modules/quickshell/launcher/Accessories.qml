// Bluetooth, a collection: the devices paired, those connected first, then
// the rest by name, each glyph saying what kind of device it is and the
// footer its battery where it says; then Nearby, where a device never
// paired is found; and last the machine's own Adapter. Return connects
// one, or disconnects the one connected, and the launcher stays, saying
// it's connecting, until it's done or says why not, in BlueZ's own terms
// made plain (Nearby.js). A device that wants a passkey typed is paired
// in bluetoothctl instead, since this pairs only as headphones and mice
// do, with nothing to type. Turning Bluetooth off and on is an action in
// the main search, and the Adapter's Radio too. Everything of a device's
// state comes from BlueZ over D-Bus through Quickshell, so no process is
// asked.
//
// Nearby is a place: only while it's open does the radio look for
// devices, and it lists every named one it hears, paired or not,
// strongest first, each with its kind's glyph, its signal as a bar and in
// dBm, and whether it's paired or connected. Return on one never paired
// trusts it, pairs it and connects it; on one paired, connects it. A
// device's signal and why a pairing failed come from the shell's patch to
// Quickshell (modules/quickshell/bluetooth-rssi-and-failures.patch).
//
// A device's right arrow lists what can be done to it, then its parts:
// its Battery as a bar; its Signal and how well its link carries (Link
// quality) as sparks of their last twenty reads; the Codec its sound goes
// in, as PipeWire names it, holding what it's for, its sample rate and
// its channels; how many Channels of the radio's are clear, the rest
// skipped as noisy, which is how crowded the air is; its Connection,
// which radio carries it, which end leads it and whether it's encrypted;
// how an input device's kernel side says it's carried, holding its
// buttons, wheel or keys and ids (from desk-facts); whether it Reconnects
// by itself and Wakes the machine, settings whose two choices are a step
// in; and its Hardware Address, which Return copies. The Adapter holds its Radio,
// whether it's Visible to others and Pairable, as settings, then its
// Traffic as a spark, its Name, Hardware Address, Version and Chip.
//
// BlueZ says nothing of a link's signal, quality, channels or how it's
// held, nor of an adapter's traffic, address or version, so they're from
// the file the machine's link reader writes (modules/bluetooth-links),
// with each link's last reads, so a spark is full the moment it's shown.
// It's read as a list is made, and while a device's parts, the Adapter's
// or an answer are shown it's watched and read again only when the reader
// writes it, never for a key; a machine without the reader shows none of
// them. "bluetooth performance", "bluetooth signal" or "interference"
// answers with each connected device's Signal and Channels.
import qs
import Quickshell
import Quickshell.Bluetooth
import Quickshell.Io
import Quickshell.Services.Pipewire
import QtQuick
import "../Devices.js" as Devices
import "../Fuzzy.js" as Fuzzy
import "Links.js" as Links
import "Nearby.js" as Nearby
import "Order.js" as Order

Provider {
    id: provider

    what: "Bluetooth"

    // BlueZ's own, which a check gives a stand-in for.
    property var adapter: Bluetooth.defaultAdapter
    readonly property bool on: adapter !== null && adapter.enabled

    // :set bluetooth, nobluetooth and bluetooth! turn it as its row does.
    setOptions: adapter === null ? ({}) : ({
        bluetooth: {
            kind: "switch",
            // Off drops every device connected.
            ends: true,
            now: () => provider.on,
            set: to => {
                provider.adapter.enabled = to;
                return "";
            }
        }
    })

    // On, it always holds Nearby and the Adapter, so only off is it empty.
    collection: adapter === null ? null : Words.collection("bluetooth")

    // Looking for devices costs the radio's power, and has BlueZ keep every
    // stranger it hears, so it looks only while Nearby is open to show
    // them, or while a terminal's `launcher show` walked into it
    // (Launcher.qml's terminalSees), so Nearby asked for cold lists what's
    // heard.
    readonly property bool looksNearby: {
        const launcher = provider.launcher;
        return on && launcher !== null && (launcher.open
            && (launcher.held ?? []).some(step => step.row.key === "bluetooth:nearby")
            || terminalLooks);
    }
    readonly property bool terminalLooks: (launcher?.terminalSees.walked ?? [])
        .some(row => row?.key === "bluetooth:nearby")

    // A terminal walked into Nearby waits while nothing's heard yet, as
    // before the radio's first look, for the first seconds it listens:
    // BlueZ says what it hears as it hears it, never that a look is done.
    // A radio that can't listen both ways at once listens for LE devices
    // for 5.12 s before it asks for classic ones, and a device in btvirt
    // was heard 4.3 s in, so a show waits 8 s while nothing's heard and no
    // longer (Provider.stillReading), within the program's own longer wait
    // (in-a-terminal's SETTLING, 10 s, for a Wi-Fi look).
    stillReading: terminalLooks && on && !listened
        && nearby.length === 0 && nameless.length === 0

    // Whether this look has listened its few seconds, false from the moment
    // a terminal walks in, before the Timer has started.
    property bool listened: false
    Timer {
        id: listening

        interval: 8000
        onTriggered: provider.listened = true
    }
    onTerminalLooksChanged: {
        listened = false;
        if (terminalLooks)
            listening.restart();
        else
            listening.stop();
    }

    Binding {
        target: provider.adapter
        property: "discovering"
        value: provider.looksNearby
        when: provider.adapter !== null
    }

    readonly property var radio: thing({
        key: "bluetooth:radio",
        // An act, not a device, so it says no kind (docs/design.md).
        what: "",
        verb: Words.verb("bluetooth:radio", on ? "" : "off"),
        name: Words.name("bluetooth:radio", on ? "" : "off"),
        glyph: Words.glyph("bluetooth:radio", on ? "" : "off"),
        // Its words' state, so the : line names the command for it.
        wordState: on ? "" : "off",
        hue: "machine",
        stays: true,
        run: () => provider.adapter.enabled = !provider.on
    })

    things: adapter === null ? [] : [radio]

    // What it is, from the icon BlueZ names for its class, as the theme's
    // mark for that kind of thing.
    function kind(device) {
        return Theme.glyph[Devices.kind(device.icon)];
    }

    // The devices paired, or connected while pairing, connected first, then
    // by name; a device that never said its name is left out, as nothing
    // could find it. One never paired is Nearby's.
    readonly property var devices: {
        if (!on)
            return [];
        const rank = device => device.connected ? 0 : 1;
        return Order.sorted(adapter.devices.values.filter(device => device.deviceName !== ""
            && (device.paired || device.connected)),
            Order.live({ stands: rank, name: device => device.name }));
    }

    // Those Nearby hears, strongest first, as their rows say them: a list
    // shown keeps its order as they move (Steady.js), so only the first
    // look orders them. Heard again, Nearby's rows say so. Those that never
    // said their names are only counted, after the rest.
    readonly property var nearby: on && looksNearby ? Nearby.heard(adapter.devices.values) : []
    readonly property var nameless: on && looksNearby
        ? Nearby.nameless(adapter.devices.values) : []
    readonly property string nearbySaid: nearby.concat(nameless)
        .map(device => [device.address, device.rssi, device.paired, device.connected].join(" "))
        .join(",")
    onNearbySaidChanged: changed()

    // The device being connected, and what it said if it wouldn't be.
    property var joining: null
    property string trouble: ""

    // What Bluetooth says, in words.json's words, of a device where it's
    // given, as the foot says how connecting it goes.
    function word(id) {
        return Words.of("bluetooth").said?.[id] ?? id;
    }

    function saidOf(id, device) {
        return word(id).replace("{device}", device.name);
    }

    // What BlueZ answered a pairing or connecting that failed, in Joel's
    // words (Nearby.js's failure), as a live act says it didn't happen:
    // "Buds not paired \u00b7 it wants a passkey"; a reason Joel has no
    // words for in BlueZ's own. Nothing for one done already. Quickshell
    // says it before the device stops pairing, so which it was is known.
    function failed(device, name, message) {
        const failure = Nearby.failure(name, message);
        if (failure === "")
            return;
        const why = failure === "other" ? Nearby.plain(name, message) : word(failure);
        trouble = word("because").replace("{failed}",
            saidOf(device.pairing ? "notPaired" : "notConnected", device)).replace("{why}", why);
    }

    // A new device is trusted before it pairs. Headphones start their audio
    // the moment they're paired, and with no agent to ask, BlueZ refuses an
    // untrusted device's audio, so it hangs up; trust given after pairing
    // comes too late. A pairing that fails takes the trust back.
    function join(device) {
        trouble = "";
        joining = device;
        if (device.paired) {
            device.connect();
            return;
        }
        device.trusted = true;
        device.pair();
    }

    function said(device, otherwise) {
        if (joining !== device)
            return otherwise;
        if (trouble !== "")
            return trouble;
        return saidOf(device.pairing ? "pairing" : "connecting", device);
    }

    // Disconnected, the device's row says so at once, and inside it
    // Disconnect It goes, until bluez says so too (launcher/Becoming.js).
    // bluez answers nothing to a disconnect, so only its word on the device
    // tells it took.
    function leave(device) {
        becomes(`bluetooth:${device.address}`, made(device, false), device.name, "disconnect");
        becomes("verb:bluetooth-disconnect", null, device.name, "disconnect");
        device.disconnect();
    }

    // The device's row, as connected as `joinedAs` says where it's given,
    // or as bluez says.
    function made(device, joinedAs, nearby) {
        const joined = joinedAs ?? device.connected;
        if (nearby)
            return nearbyRow(device, joined);
        // The side says connected or paired, so the note says only what it
        // doesn't: the battery, or that the device is new.
        const told = joined || device.paired ? "" : "new";
        const charge = Math.round(device.battery * 100);
        const stood = joined ? "connected" : device.paired ? "paired" : "";
        // One running low says so at its side, where it warns, and its note
        // says what the side would. Running low is the devices piece's own
        // word on it, so this row and Now, which lists it as this row,
        // never disagree while its charge wavers about the mark.
        const warns = provider.runningLow.includes(`bluetooth:${device.address}`);
        const note = warns ? stood : device.batteryAvailable ? `${charge}% battery` : told;
        return member({
            thing: true,
            key: `bluetooth:${device.address}`,
            verb: joined ? "disconnect" : device.paired ? "connect" : "pair",
            name: device.name,
            glyph: kind(device),
            hue: "machine",
            side: warns ? `${charge}% \u00b7 running low` : stood,
            warns: warns,
            note: note,
            device: device,
            state: () => provider.said(device, note),
            // Connecting or disconnecting keeps the launcher, which says how
            // it went.
            stays: true,
            run: joined ? () => provider.leave(device) : () => provider.join(device),
            verbs: () => provider.verbs(device),
            // Joined, its Return disconnects, which Return typed in insert
            // never does.
            ends: joined ? Words.ending("verb:bluetooth-disconnect").ends : ""
        });
    }

    // The machine's own adapter, last, so Return on the first row still
    // connects the likeliest device: a thing holding how it's set and what
    // it is.
    readonly property var adapterRow: adapter === null ? null : thingRow({
        thing: true,
        key: "bluetooth:adapter",
        name: Words.name("bluetooth:adapter"),
        side: on ? "on" : "off",
        glyph: Theme.glyph.bluetooth,
        adapter: adapter
    }, () => provider.adapterParts(provider.adapter))

    // A device as Nearby hears it: what kind it is, how strong it's heard,
    // as a bar and in dBm, and whether it's paired or connected; the same
    // thing as its Bluetooth row, holding the same acts and parts. Its
    // figure is said at its right as an access point's is, and how strong
    // that is follows it muted after a dot, as every figure's qualifier
    // does (docs/design.md; VKCF3Z, 2026-11-07); paired or connected says
    // most, and the band gives way first.
    function nearbyRow(device, joined) {
        const strength = Nearby.strength(device);
        const items = (strength === null ? [] : [
            { said: word("heard").replace("{dbm}", Nearby.dbm(device)), keep: 2 },
            { said: word(strength.word), keep: 1, hue: "muted" }
        ]).concat([{ said: joined ? "connected" : device.paired ? "paired" : "", keep: 3 }])
            .filter(item => item.said !== "");
        return member({
            thing: true,
            key: `bluetooth:${device.address}`,
            verb: joined ? "disconnect" : device.paired ? "connect" : "pair",
            name: device.name,
            glyph: kind(device),
            hue: "machine",
            side: items.map(item => item.said).join(" \u00b7 "),
            sideItems: items,
            graph: strength === null ? undefined : bar(strength.value, false),
            device: device,
            state: () => provider.said(device, ""),
            stays: true,
            run: joined ? () => provider.leave(device) : () => provider.join(device),
            verbs: () => provider.verbs(device),
            ends: joined ? Words.ending("verb:bluetooth-disconnect").ends : ""
        });
    }

    // Nearby, the place a device is found in, after those paired: it says
    // how many it heard once it's looked, and what it does until then.
    readonly property var nearbyPlace: !on ? null : thingRow({
        thing: true,
        key: "bluetooth:nearby",
        name: Words.name("bluetooth:nearby"),
        keywords: (Words.of("bluetooth:nearby").words ?? []).join(" "),
        side: nearby.length > 0 ? word("heardCount").replace("{count}", nearby.length) : "",
        note: word("looks"),
        glyph: Words.glyph("bluetooth:nearby"),
        empty: Words.of("bluetooth:nearby").empty
    }, () => provider.nearby.map(device => provider.made(device, undefined, true))
        .concat(provider.namelessRow === null ? [] : [provider.namelessRow]))

    // Those heard without a name, counted as Nearby's last row, so nothing
    // heard is hidden and nothing unreadable is listed among the rest.
    // Stepped into, it lists them by the addresses BlueZ names them by, so
    // one whose name comes only once it's paired can still be.
    readonly property var namelessRow: nameless.length === 0 ? null : thingRow({
        key: "bluetooth:nearby:nameless",
        name: word("nameless").replace("{count}", nameless.length),
        glyph: "",
        hue: "machine"
    }, () => provider.nameless.map(device => provider.made(device, undefined, true)))

    readonly property var all: devices.map(device => made(device))
        .concat(nearbyPlace === null ? [] : [nearbyPlace])
        .concat(adapterRow === null ? [] : [adapterRow])
    onAllChanged: changed()

    // What a device's spots are made of, so a closing makes again only the
    // parts of a device that changed (Launcher.qml's heldSpots): how it
    // stands, which of its link's readings there are, its codec and what
    // desk-facts says of it, never a level, since a battery or a signal
    // moving shows only in its figures. Joel's closing at 14:36 on
    // 2026-11-11 made Bluetooth's 20 rows again. Null for any other thing,
    // Nearby's and the adapter's, which are made again. A daemon serving
    // Bluetooth's rows ready, as wifid serves Wi-Fi's, would end it.
    function spotsStampOf(thing) {
        const device = thing?.device ?? null;
        if (device === null || thing.key !== `bluetooth:${device.address}`)
            return null;
        const link = device.connected ? Links.of(links(false), device.address) : null;
        const node = device.connected ? soundNode(device) : null;
        const readings = [Links.signal(link), Links.quality(link), Links.channels(link),
            Links.held(link)].map(one => one !== null);
        const carried = node === null ? null
            : Links.carried(node.properties, node.audio?.channels?.length ?? 0);
        return JSON.stringify([device.address, device.name, device.icon ?? "",
            device.connected, device.paired, device.trusted, device.wakeAllowed,
            device.batteryAvailable, readings,
            node === null ? "" : node.properties["api.bluez5.codec"] ?? "", carried,
            (facts[device.address] ?? []).map(fact => `${fact.name}:${fact.value}`)]);
    }

    // What desk-facts knows of each connected input device, by its address;
    // a device it says nothing of, or no file at all, has no facts.
    property var facts: ({})

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/desk/bluetooth.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: provider.facts = provider.parsed(text())
        onLoadFailed: provider.facts = ({})
    }

    // How each link is doing, as the reader last wrote it: its text, when
    // it was read, and when watching it began. While something of it is
    // shown the file is watched, and read again only when the reader
    // writes it; otherwise it's read as it's asked, once a second at most,
    // so an answer asked again by each letter of its question reads it
    // once. A check points it at a stand-in.
    readonly property var memo: ({ text: "", at: 0, since: 0 })
    readonly property int second: 1000

    FileView {
        id: linksFile
        path: provider.settings.links ?? "/run/bluetooth-links.json"
        blockLoading: true
        printErrors: false
        watchChanges: provider.watching
        onFileChanged: provider.heard()
    }

    readonly property bool watching: looking !== null || answering
    onWatchingChanged: memo.since = Date.now()

    // What the reader's file says, read again unless it's been read since
    // the reader last wrote it, as far as can be told.
    function linksText(fresh) {
        const now = Date.now();
        const current = watching ? memo.at >= memo.since : now - memo.at < second;
        if (fresh || !current) {
            memo.text = read(linksFile);
            memo.at = now;
        }
        return memo.text;
    }

    // Every link the reader says, or null with no reader.
    function links(fresh) {
        return Links.links(linksText(fresh));
    }

    // The reader wrote its file while something of it is shown: an answer
    // is asked again, and a list says it changed only when the file did.
    function heard() {
        const before = memo.text;
        linksText(true);
        if (answering)
            answered();
        else if (memo.text !== before)
            changed();
    }

    // PipeWire's own Bluetooth nodes, whose properties name the codec each
    // sends in and what it carries, heard once something binds them.
    readonly property var bluez: Pipewire.nodes.values
        .filter(node => (node.name ?? "").startsWith("bluez_"))

    PwObjectTracker {
        objects: provider.bluez
    }

    // The node a device's sound goes through, its output's before its
    // input's, or null for a device with no sound.
    function soundNode(device) {
        const nodes = bluez.filter(node => node.properties["api.bluez5.address"] === device.address
            && !!node.properties["api.bluez5.codec"]);
        return nodes.find(one => one.isSink) ?? nodes[0] ?? null;
    }

    // How rows are drawn. A figure says something and Return closes; a leaf
    // worth having elsewhere, as an address, copies what it says. Neither
    // wears a glyph.
    function saying(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    function leaf(fields) {
        return copyRow(Object.assign({ copy: fields.side, hue: "machine", glyph: "", note: "" },
            fields));
    }

    // A graph a row draws (Graph.qml): a bar has one value, a spark one a
    // read, the newest last, each 0 to 1.
    readonly property int graphed: 20

    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    function spark(values) {
        return { shape: "spark", values: values.slice(-graphed), warn: false };
    }

    // A setting as a part: its name, how it's set, and its choices a step
    // in, each an act wearing its thing's glyph.
    function setting(owner, id, side, mark, choices, fields) {
        return holderRow(Object.assign({
            key: `bluetooth:${owner}:${id}`,
            name: Words.name(`bluetooth:${id}`),
            side: side,
            glyph: mark,
            offers: true
        }, fields ?? {}), choices);
    }

    function choice(id, chosen, mark, run) {
        return thing({
            key: `verb:${id}`,
            name: Words.name(`verb:${id}`),
            // Its own word for the foot where it has one, since a choice's
            // name can start with a word that says nothing ("Only").
            verb: Words.verb(`verb:${id}`) ?? "",
            glyph: mark,
            hue: "action",
            side: chosen ? "chosen" : "",
            stays: true,
            run: run
        });
    }

    // The marks of the devices running low (pieces/Devices.qml).
    readonly property var runningLow: Facts.all
        .filter(fact => fact.piece?.place === "bluetooth")
        .map(fact => fact.entry.mark)

    // A battery at this share or below warns.
    readonly property real low: 0.2

    function batteryRow(device) {
        if (!device.batteryAvailable)
            return null;
        const warns = device.battery <= low;
        return saying({
            key: `bluetooth:${device.address}:battery`,
            name: Words.name("bluetooth:battery"),
            side: `${Math.round(device.battery * 100)}%`,
            graph: bar(device.battery, warns),
            note: warns ? "running low" : ""
        });
    }

    function signalRow(device, link) {
        const signal = Links.signal(link);
        return signal === null ? null : saying({
            key: `link:bluetooth:${device.address}:signal`,
            name: Words.name("bluetooth:signal"),
            side: signal.side,
            graph: spark(Links.signals(link)),
            note: signal.note
        });
    }

    function qualityRow(device, link) {
        const quality = Links.quality(link);
        return quality === null ? null : saying({
            key: `link:bluetooth:${device.address}:quality`,
            name: Words.name("bluetooth:quality"),
            side: quality.side,
            graph: spark(Links.qualities(link)),
            note: quality.note
        });
    }

    // The codec a connected device's sound goes in, holding its sample rate
    // and its channels, as far as PipeWire says them. What it's for, music
    // or calls, is Sound's Profile, so it isn't said twice; the codec's
    // note says when it's a call's.
    function codecRow(device) {
        const node = device.connected ? soundNode(device) : null;
        const sent = node ? Links.codec(node.properties["api.bluez5.codec"]) : null;
        if (sent === null)
            return null;
        const carried = Links.carried(node.properties, node.audio?.channels?.length ?? 0);
        const parts = [["rate", carried.rate], ["sound-channels", carried.channels]].filter(each => each[1] !== "")
            .map(each => saying({
                key: `bluetooth:${device.address}:${each[0]}`,
                name: Words.name(`bluetooth:${each[0]}`),
                side: each[1]
            }));
        const fields = {
            key: `link:bluetooth:${device.address}:codec`,
            name: Words.name("bluetooth:codec"),
            side: sent.name,
            note: sent.note,
            device: device
        };
        return parts.length === 0 ? saying(fields)
            : holderRow(Object.assign(fields, { glyph: kind(device) }), () => parts);
    }

    function channelsRow(device, link) {
        const channels = Links.channels(link);
        return channels === null ? null : saying({
            key: `link:bluetooth:${device.address}:channels`,
            name: Words.name("bluetooth:channels"),
            side: channels.side,
            graph: { shape: "strip", values: channels.values, warn: false },
            note: channels.note
        });
    }

    function connectionRow(device, link) {
        const held = Links.held(link);
        return held === null ? null : saying({
            key: `link:bluetooth:${device.address}:held`,
            name: Words.name("bluetooth:held"),
            side: held.side,
            note: held.note
        });
    }

    // What desk-facts says of a connected input device: how it's carried,
    // holding the rest, each named first as a part is (Buttons "5").
    function inputRow(device) {
        const told = facts[device.address] ?? [];
        if (told.length === 0)
            return null;
        const carried = told.find(fact => fact.name === "connection") ?? null;
        const rest = told.filter(fact => fact !== carried);
        const fields = {
            key: `bluetooth:${device.address}:input`,
            name: Words.name("bluetooth:input"),
            side: carried ? carried.value : "",
            device: device
        };
        if (rest.length === 0)
            return leaf(fields);
        return holderRow(Object.assign(fields, { glyph: kind(device) }),
            () => provider.factParts("bluetooth", device.address, rest));
    }

    // Whether a paired device comes back by itself, which BlueZ calls
    // trusting it, and whether an input device may wake the machine.
    function trustedRow(device) {
        if (!device.paired)
            return null;
        const trusted = device.trusted;
        const mark = kind(device);
        return setting(device.address, "trusted", trusted ? "yes" : "no", mark, () => [
            choice("bluetooth-trust", trusted, mark, () => device.trusted = true),
            choice("bluetooth-untrust", !trusted, mark, () => device.trusted = false)
        ], { device: device });
    }

    function wakeRow(device) {
        if (!device.paired || !(device.icon ?? "").startsWith("input-"))
            return null;
        const wakes = device.wakeAllowed;
        const mark = kind(device);
        return setting(device.address, "wake", wakes ? "yes" : "no", mark, () => [
            choice("bluetooth-wake", wakes, mark, () => device.wakeAllowed = true),
            choice("bluetooth-no-wake", !wakes, mark, () => device.wakeAllowed = false)
        ], { device: device });
    }

    function addressRow(device) {
        return leaf({
            key: `bluetooth:${device.address}:address`,
            name: Words.name("bluetooth:address"),
            side: device.address
        });
    }

    // A device's parts, after what can be done to it; the link's only
    // while it's connected.
    function deviceParts(device) {
        const link = device.connected ? Links.of(links(false), device.address) : null;
        return [batteryRow(device), signalRow(device, link), qualityRow(device, link),
            codecRow(device), channelsRow(device, link), connectionRow(device, link),
            inputRow(device), trustedRow(device), wakeRow(device), addressRow(device)]
            .filter(row => row !== null);
    }

    // What the kernel's sysfs says of an adapter's chip, its maker's and
    // model's ids, read once, since they never change: a PCI chip's as
    // vendor and device, a USB one's from the device its interface is of.
    readonly property var chips: ({})

    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }

    function sysfs(file) {
        probe.path = file;
        return read(probe).trim().replace(/^0x/, "");
    }

    function chip(radio, told) {
        const id = radio.adapterId;
        if (chips[id] === undefined) {
            const at = `/sys/class/bluetooth/${id}/device`;
            const pci = [sysfs(`${at}/vendor`), sysfs(`${at}/device`)];
            const ids = pci[0] !== "" ? pci : [sysfs(`${at}/../idVendor`),
                sysfs(`${at}/../idProduct`)];
            chips[id] = ids.every(part => part !== "") ? ids.join(":") : "";
        }
        return [told ? Links.maker(told.maker) : "", chips[id]].filter(part => part !== "")
            .join(", ");
    }

    // How long the adapter stays visible once it's made so, as its note.
    function hides(radio) {
        const seconds = radio.discoverableTimeout;
        const minutes = Math.round(seconds / 60);
        if (seconds <= 0)
            return "";
        return seconds < 60 ? `for ${seconds} s once made visible`
            : `for ${minutes} min once made visible`;
    }

    // The adapter's parts: its Radio, and while it's on whether others can
    // find it and pair with it, and what it carries; then what it is.
    function adapterParts(radio) {
        const told = Links.adapter(Links.adapters(linksText(false)), radio.adapterId);
        const mark = Theme.glyph.bluetooth;
        const enabled = radio.enabled;
        const rows = [setting("adapter", "powered", enabled ? "on" : "off", mark, () => [
            choice("bluetooth-radio-on", enabled, Words.glyph("bluetooth:radio", "off"),
                () => radio.enabled = true),
            choice("bluetooth-radio-off", !enabled, Words.glyph("bluetooth:radio"),
                () => radio.enabled = false)
        ])];
        if (enabled) {
            const visible = radio.discoverable;
            rows.push(setting("adapter", "discoverable", visible ? "yes" : "no", mark, () => [
                choice("bluetooth-visible", visible, mark, () => radio.discoverable = true),
                choice("bluetooth-hidden", !visible, mark, () => radio.discoverable = false)
            ], { note: hides(radio) }));
            const pairable = radio.pairable;
            rows.push(setting("adapter", "pairable", pairable ? "yes" : "no", mark, () => [
                choice("bluetooth-pairable", pairable, mark, () => radio.pairable = true),
                choice("bluetooth-unpairable", !pairable, mark, () => radio.pairable = false)
            ]));
            const carried = Links.traffic(told);
            if (carried !== null)
                rows.push(saying({
                    key: "bluetooth:adapter:traffic",
                    name: Words.name("bluetooth:traffic"),
                    side: carried.side,
                    graph: spark(carried.values),
                    note: carried.note
                }));
        }
        // What it is, each worth copying, as a model is.
        return rows.concat([["name", radio.name], ["address", told ? told.address : ""],
            ["version", told ? Links.version(told.version) : ""], ["chip", chip(radio, told)]]
            .filter(each => each[1] !== "").map(each => leaf({
                key: `bluetooth:adapter:${each[0]}`,
                name: Words.name(`bluetooth:${each[0]}`),
                side: each[1]
            })));
    }

    // How the links are doing, as an answer: one device's Signal and
    // Channels, or a row each of several, with theirs a step in.
    questions: Words.started("bluetooth:links")

    function answer(typed) {
        if (asks(typed) === "" || !on)
            return null;
        return linksAnswer(devices.filter(device => device.connected));
    }

    function linksAnswer(connected) {
        const all = links(false);
        if (all === null)
            return null;
        const told = connected.map(device => {
            const link = Links.of(all, device.address);
            return {
                device: device,
                link: link,
                signal: Links.signal(link),
                channels: Links.channels(link),
                rows: [signalRow(device, link), channelsRow(device, link)]
                    .filter(row => row !== null)
            };
        }).filter(each => each.rows.length > 0);
        if (told.length === 0)
            return null;
        const said = each => [each.signal ? each.signal.side : "",
            each.channels ? each.channels.side : ""].filter(part => part !== "").join(" \u00b7 ");
        const graph = each => each.signal ? spark(Links.signals(each.link)) : undefined;
        const glyph = Words.glyph("bluetooth");
        if (told.length === 1)
            return {
                headline: said(told[0]),
                copy: "",
                graph: graph(told[0]),
                glyph: glyph,
                note: `the link to ${told[0].device.name}`,
                rows: told[0].rows,
                live: provider
            };
        return {
            headline: `${told.length} connected`,
            glyph: glyph,
            note: "",
            rows: told.map(each => thingRow({
                thing: true,
                key: `link:bluetooth:${each.device.address}`,
                name: each.device.name,
                side: said(each),
                graph: graph(each),
                glyph: kind(each.device)
            }, () => each.rows)),
            live: provider
        };
    }

    // The connected device, or the adapter, whose parts are shown, and
    // whether one of its answers is: the file is watched while they're
    // there, and never otherwise.
    readonly property var looking: {
        const launcher = provider.launcher;
        if (!launcher || !launcher.open || launcher.held.length === 0)
            return null;
        const row = launcher.held[launcher.held.length - 1].row;
        if (row.from !== undefined && row.from !== provider)
            return null;
        if (row.adapter)
            return row.adapter;
        const shown = row.device ?? null;
        return shown && shown.connected ? shown : null;
    }
    readonly property bool answering: launcher !== null && launcher.open
        && launcher.held.length === 0 && launcher.answer?.live === provider

    function verbs(device) {
        const rows = [device.connected ? thing(Object.assign({
            key: "verb:bluetooth-disconnect",
            name: Words.name("verb:bluetooth-disconnect"),
            glyph: Theme.glyph.disconnect,
            hue: "action",
            stays: true,
            run: () => provider.leave(device),
            // A device connected was paired, so joining it again is how u
            // puts it back.
            undo: () => provider.join(device)
        }, Words.ending("verb:bluetooth-disconnect"))) : thing({
            key: "verb:bluetooth-connect",
            verb: device.paired ? "connect" : "pair",
            name: Words.name("verb:bluetooth-connect", device.paired ? "" : "unpaired"),
            glyph: Theme.glyph.connect,
            hue: "action",
            device: device,
            stays: true,
            state: () => provider.said(device, ""),
            run: () => provider.join(device)
        })];
        if (!device.paired)
            rows.push(thing({
                key: "verb:bluetooth-passkey",
                name: Words.name("verb:bluetooth-passkey"),
                glyph: Theme.glyph.password,
                hue: "action",
                later: true,
                run: () => Niri.spawn(["kitty", "bluetoothctl"])
            }));
        if (device.paired)
            rows.push(thing(Object.assign({
                key: "verb:bluetooth-forget",
                name: Words.name("verb:bluetooth-forget"),
                glyph: Theme.glyph.forget,
                hue: "action",
                stays: true,
                run: () => device.forget()
            }, Words.ending("verb:bluetooth-forget"))));
        return rows.concat(deviceParts(device));
    }

    // Done with the device being connected: a passkey asked for is done
    // with, so the launcher steps back to the device, which says it's
    // connected, and stays, as a setting does.
    function finish(device) {
        joining = null;
        const launcher = provider.launcher;
        if (!launcher)
            return;
        const at = launcher.selected ?? launcher.asking;
        if (launcher.open && at && at.device === device && launcher.asking === at)
            launcher.back();
    }

    // How the device being connected went. BlueZ says a new device is
    // connected as soon as its link is up, before pairing is done, so only a
    // paired one is finished: paired, it's connected if it isn't yet, and
    // trusted, so it comes back by itself. A pairing that ends unpaired
    // says so and is no longer trusted.
    Connections {
        target: provider.joining

        function onPairedChanged() {
            const device = provider.joining;
            if (!device || !device.paired)
                return;
            device.trusted = true;
            if (device.connected)
                provider.finish(device);
            else
                device.connect();
        }

        // BlueZ's answer comes before pairing is said to have ended, so a
        // pairing that ends unpaired with no answer says only that.
        function onFailed(name, message) {
            const device = provider.joining;
            if (device)
                provider.failed(device, name, message);
        }

        function onPairingChanged() {
            const device = provider.joining;
            if (device && !device.pairing && !device.paired) {
                device.trusted = false;
                if (provider.trouble === "")
                    provider.trouble = provider.saidOf("notPaired", device);
            }
        }

        function onConnectedChanged() {
            const device = provider.joining;
            if (device && device.connected && device.paired)
                provider.finish(device);
        }

        function onStateChanged() {
            const device = provider.joining;
            if (device && device.paired && device.state === BluetoothDeviceState.Disconnected
                && !device.connected && provider.trouble === "")
                provider.trouble = provider.saidOf("notConnected", device);
        }
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

    // With Bluetooth off, turning it on comes first, then the adapter.
    function everything() {
        return on ? all.slice() : [radio].concat(all);
    }

    function counted() {
        return !on ? "off" : `${all.filter(row => row.device && row.side !== "").length} paired`;
    }
}
