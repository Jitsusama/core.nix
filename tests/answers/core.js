// The cases of core for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Asking.js" as Asking
.import "../Standing.js" as Standing
.import "../Cameras.js" as Cameras
.import "../Seen.js" as Seen

function run(failures) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    // What a URL said stands while it's asked, then for its fresh
    // time after an answer, or the failure's time after a failure, and
    // is asked again after either.
    const standing = [
        ["nothing asked yet", null, 0, false],
        ["still being asked", { pending: true, at: 0 }, 60000, true],
        ["an answer, fresh", { at: 0 }, 59999, true],
        ["an answer, stale", { at: 0 }, 60000, false],
        ["a failure, fresh", { failed: true, at: 0 }, 9999, true],
        ["a failure, stale", { failed: true, at: 0 }, 10000, false]
    ];
    for (const [what, entry, now, stands] of standing) {
        asked++;
        if (Asking.stands(entry, now, 60000, 10000) !== stands) {
            failures.push(`${what} ${stands ? "was asked again" : "wasn't asked again"}`);
        }
    }

    // One still being asked is given up on once its patience has
    // passed, and nothing else ever is.
    const overdue = [
        ["nothing asked yet", null, 0, null],
        ["just asked", { pending: true, at: 1000 }, 1000, 5000],
        ["nearly given up on", { pending: true, at: 1000 }, 5999, 1],
        ["given up on", { pending: true, at: 1000 }, 6000, 0],
        ["long given up on", { pending: true, at: 1000 }, 60000, 0],
        ["answered", { at: 1000 }, 60000, null],
        ["failed", { failed: true, at: 1000 }, 60000, null]
    ];
    for (const [what, entry, now, left] of overdue) {
        asked++;
        const got = Asking.overdueIn(entry, now, 5000);
        if (got !== left) {
            failures.push(`${what} was given up on in ${got}, not ${left}`);
        }
    }

    // How things stand in the foot, as room runs short: the least of
    // each, then each its glyph by rank, and no glyph ever gives way,
    // each a character wide here, two between.
    const fact = (sign, glance, least, collapse) => ({ sign, glance, least, entry: { collapse } });
    const facts = [fact("", "17:30", "17:30", null), fact("", "Sat 24 Oct", "Sat 24 Oct", 3),
        fact("B", "50% 0:02 left", "50%", 20), fact("N", "", "", 10), fact("V", "40%", "", 0),
        fact("R", "$9 today", "$9", 5), fact("R", "2 working", "2", 6)];
    const footFor = room => Standing.fitted(facts, told => told.reduce((sum, one) => sum
        + one.said.length, 0) + 2 * Math.max(0, told.length - 1) <= room)
        .map(one => one.said).join("  ");
    for (const [room, said] of [
        [100, "17:30  Sat 24 Oct  B 50% 0:02 left  N  V 40%  R $9 today  2 working"],
        [39, "17:30  Sat 24 Oct  B 50%  N  V  R $9  2"],
        // The date has no glyph, so it gives way where it'd shrink.
        [30, "17:30  B 50%  N  V  R $9  2"],
        // The day's spend and the agents as their glyph are one robot.
        [22, "17:30  B 50%  N  V  R"],
        [17, "17:30  B  N  V  R"],
        // Every glyph stays, however short the room, as the time does.
        [12, "17:30  B  N  V  R"],
        [3, "17:30  B  N  V  R"]
    ]) {
        machine(`how things stand in ${room}`, footFor(room), said);
    }
    // Once the foot has had to read shorter, it reads no longer until
    // the launcher closes, though there's room again.
    const roomy = told => told.reduce((sum, one) => sum + one.said.length, 0)
        + 2 * Math.max(0, told.length - 1) <= 100;
    const tight = Standing.fittedFrom(facts, told => told.reduce((sum, one) => sum
        + one.said.length, 0) + 2 * Math.max(0, told.length - 1) <= 22, 0);
    machine("short of room, the foot reads shorter", tight.told.map(one => one.said).join("  "),
        "17:30  B 50%  N  V  R");
    machine("and with room again, no longer", Standing.fittedFrom(facts, roomy, tight.stage)
        .told.map(one => one.said).join("  "), "17:30  B 50%  N  V  R");
    machine("from the first way, as long as there's room", Standing.fittedFrom(facts, roomy, 0)
        .stage, 0);
    machine("a way past the last is the last", Standing.fittedFrom(facts, roomy, 99)
        .told.map(one => one.said).join("  "), "17:30  B  N  V  R");
    // The keys in the room the facts leave them, Return first as told:
    // those furthest from Return give way first, never Return nor
    // Space ? just before it, each key its letters wide here, two
    // between, as visual's foot over a full list at 2.
    const visualKeys = [["\u21b5", "open 2"], [":", "commands"], ["d", "close"], ["y", "copy"],
        ["Space ?", "keys"]];
    const keysWide = keys => keys.reduce((sum, key) => sum + `${key[0]} ${key[1]}`.length, 0)
        + 2 * Math.max(0, keys.length - 1);
    for (const [room, kept] of [
        [60, "\u21b5 : d y Space ?"],
        [44, "\u21b5 d y Space ?"],
        [33, "\u21b5 y Space ?"],
        [26, "\u21b5 Space ?"],
        [5, "\u21b5 Space ?"]
    ]) {
        machine(`the foot's keys in ${room}`, Standing.keysFitted(visualKeys, keysWide, room)
            .map(key => key[0]).join(" "), kept);
    }
    machine("Return alone stays", Standing.keysFitted([["\u21b5", "open"]], keysWide, 0)
        .length, 1);
    // On the find they give way by need instead, the words named first
    // going first, in their order, wherever they're drawn: the
    // completion, whose ghost the line shows, then next time, and
    // Tab in never, though it's drawn furthest from Return so nothing
    // slides as it comes and goes (VKCF3Z, 2026-11-06).
    const findKeys = [["\u21b5", "start"], ["Ctrl+]", "in"], ["Ctrl+Y", "complete"],
        ["Super+\u21b5", "next time"]];
    for (const [room, kept] of [
        [54, "\u21b5 Ctrl+] Ctrl+Y Super+\u21b5"],
        [40, "\u21b5 Ctrl+] Super+\u21b5"],
        [30, "\u21b5 Ctrl+]"],
        [5, "\u21b5 Ctrl+]"]
    ]) {
        machine(`the find's keys in ${room}`, Standing.keysFitted(findKeys, keysWide, room,
            ["complete", "next time"]).map(key => key[0]).join(" "), kept);
    }
    // The foot is told the selected row's d and y again only where
    // they'd say something new.
    const notes = { name: "Notes" };
    const screens = { name: "Screens" };
    const closes = [["d", "close"]];
    for (const [what, last, row, keys, anew] of [
        ["none before and none now tell nothing", { row: notes, keys: [] }, screens, [], false],
        ["the same row's same keys tell nothing", { row: notes, keys: closes }, notes,
            [["d", "close"]], false],
        ["another row's keys, though alike, are told", { row: notes, keys: closes }, screens,
            closes, true],
        ["keys where none were are told", { row: screens, keys: [] }, notes, closes, true],
        ["and none where keys were", { row: notes, keys: closes }, screens, [], true],
        ["and a key's other word", { row: notes, keys: closes }, notes,
            [["d", "quit"]], true]
    ]) {
        machine(what, Standing.toldAnew(last, row, keys), anew);
    }

    // Whether a provider's rows can be seen, so it watches its daemon
    // only then: never once only asked, and never with the launcher
    // closed unless a terminal just showed them.
    const watcher = {};
    const its = { live: watcher };
    const closed = { open: false, answer: its, held: [], matches: [], placed: ["heat"] };
    const opened = fields => Object.assign({ open: true, answer: null, held: [], matches: [],
        placed: [] }, fields);
    machine("unseen with the launcher closed, whatever it last held",
        Seen.seen(closed, watcher, ["heat"]), false);
    machine("unseen with the launcher open on something else",
        Seen.seen(opened({ answer: { live: {} }, placed: ["power"] }), watcher, ["heat"]),
        false);
    machine("seen as its answer", Seen.seen(opened({ answer: its }), watcher, []), true);
    machine("seen stepped into", Seen.seen(opened({ held: [{ row: its }] }), watcher, []),
        true);
    machine("seen among what's found", Seen.seen(opened({ matches: [its] }), watcher, []),
        true);
    // A collection's own row, and a member of it found from outside,
    // are the collection's (Launcher.qml's into and gathered rows).
    machine("seen stepped into as a collection",
        Seen.seen(opened({ held: [{ row: { from: watcher } }] }), watcher, []), true);
    machine("seen as a collection's member found from outside",
        Seen.seen(opened({ matches: [{ from: watcher }] }), watcher, []), true);
    machine("unseen inside another collection",
        Seen.seen(opened({ held: [{ row: { from: {} } }] }), watcher, []), false);
    machine("seen in a place holding its row",
        Seen.seen(opened({ placed: ["heat"] }), watcher, ["heat"]), true);
    machine("seen as the place it says how things stand in",
        Seen.seen(opened({ matches: [{ category: { id: "energy" } }] }), watcher,
            ["energy", "power"]), true);
    machine("seen by a terminal's show with the launcher closed",
        Seen.seen(Object.assign({}, closed, { terminalSees: { walked: [], listed: [its] } }),
            watcher, []), true);
    machine("no launcher sees nothing", Seen.seen(null, watcher, ["heat"]), false);
    // Its answer is made from what the daemon says, so words that ask
    // it watch it first, and a row a condition holds back until the
    // daemon says is still watched where its place holds it.
    const asking = typed => typed === "heat";
    machine("seen as what's typed asks it",
        Seen.seen(opened({ typed: "heat" }), watcher, [], asking), true);
    machine("unseen as what's typed asks another",
        Seen.seen(opened({ typed: "battery" }), watcher, [], asking), false);
    machine("unseen as words ask it with the launcher closed",
        Seen.seen(Object.assign({}, closed, { answer: null, typed: "heat" }), watcher, [],
            asking), false);
    machine("a question is asked whole", Seen.asks(["battery"], "battery"), true);
    machine("a question is asked in the plural", Seen.asks(["fan"], "fans"), true);
    machine("a question is asked by its start", Seen.asks(["battery"], "batt"), true);
    machine("two letters start no question", Seen.asks(["battery"], "ba"), false);
    machine("another word asks nothing", Seen.asks(["battery"], "heat"), false);
    machine("seen in a place holding it before its condition holds it back",
        Seen.seen(opened({ category: { holds: ["energy", "heat"] } }), watcher, ["heat"]),
        true);
    // What each camera says of itself, from sysfs and desk-facts.
    machine("a loopback's format at a fraction of frames",
        Cameras.loopback("NV12:1920x1080@300000/10000\n"), "NV12 1920 \u00d7 1080 at 30");
    machine("a loopback's format at whole frames", Cameras.loopback("GREY:648x368@30"),
        "GREY 648 \u00d7 368 at 30");
    machine("a loopback's format as usually written", Cameras.loopback("MJPG:640x480@15/1"),
        "MJPEG 640 \u00d7 480 at 15");
    machine("no loopback format", Cameras.loopback(""), "");
    machine("a sensor on the bus", Cameras.sensor("i2c-OVTI08F4:00"),
        { id: "OVTI08F4", lights: "OVTI08F4_00::" });
    machine("not a sensor", Cameras.sensor("i2c-17"), null);
    const cameraColour = "OVTI08F4_00::";
    const cameraInfrared = "HIMX1092_00::";
    machine("a sensor's privacy light",
        Cameras.light(`${cameraColour}privacy_led`, cameraColour), "privacy");
    machine("a sensor's IR light",
        Cameras.light(`${cameraInfrared}ir_flood_led`, cameraInfrared), "flood");
    machine("another sensor's light",
        Cameras.light(`${cameraInfrared}ir_flood_led`, cameraColour), "");
    machine("the keyboard's light", Cameras.light("dell::kbd_backlight", "OVTI08F4_00::"), "");
    machine("where the laptop's cameras sit", Cameras.place({ panel: "front", vertical: "center",
        horizontal: "right", lid: "no" }), "front, centre right");
    machine("a camera dead centre, in the lid", Cameras.place({ panel: "front",
        vertical: "center", horizontal: "center", lid: "yes" }), "front, centre, in the lid");
    machine("a camera the firmware doesn't place", Cameras.place({ panel: "unknown",
        vertical: "", horizontal: "", lid: "" }), "");
    machine("powered a moment", Cameras.lasted(999), "0 s");
    machine("powered for minutes", Cameras.lasted(678291), "11 min");
    machine("powered for hours", Cameras.lasted(7500000), "2 h 5 min");
    machine("powered for whole hours", Cameras.lasted(7200000), "2 h");
    const cameraFacts = [{ name: "streaming", value: "MJPEG 1920 \u00d7 1080 at 30" },
        { name: "MJPEG", value: "3840 \u00d7 2160 at 30; 1920 \u00d7 1080 at 60" },
        { name: "YUYV", value: "640 \u00d7 480 at 30" },
        { name: "H.264", value: "1920 \u00d7 1080 at 30" },
        { name: "field of view", value: "90\u00b0" }, { name: "USB", value: "5 Gbit/s" },
        { name: "firmware", value: "release 3.17" }];
    const cameraSplit = Cameras.formats(cameraFacts);
    machine("a camera's formats", cameraSplit.modes.map(shown => shown.name),
        ["MJPEG", "YUYV", "H.264"]);
    machine("a camera's other facts", cameraSplit.rest.map(shown => shown.name),
        ["streaming", "field of view", "USB", "firmware"]);
    machine("a camera with no facts", Cameras.formats(undefined), { modes: [], rest: [] });
    return asked;
}
