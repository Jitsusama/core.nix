// The cases of keep-awake for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Power.js" as Power
.import "../Awake.js" as Awake
.import "../Interfaces.js" as Interfaces

function run(failures, fromRoot) {
    let asked = 0;
    const machine = Common.checker(failures, () => asked++);
    const vocabulary = JSON.parse(Common.read(fromRoot, "words.json")).things;
    // How sleep stands, from the daemon's sleep:main record, in Sleep's
    // words: a machine that failed twice freezing and once resuming,
    // last freezing, held by a device.
    const sleepWords = { failed: "failed while {step}", at: "{count} while {step}",
        time: "1 time", times: "{count} times", freeze: "freezing", resume: "resuming" };
    const slept = Power.sleepOf({ key: "sleep:main", mode: "s2idle", modes: ["s2idle", "deep"],
        states: ["freeze", "mem", "disk"], successes: 12, failures: 3,
        failed_at: { freeze: 2, prepare: 0, resume: 1, suspend: null },
        last_failed_device: "0000:00:14.0", last_failed_step: "freeze",
        last_failed_errno: -16, last_hardware_sleep_us: 3_600_000_000,
        total_hardware_sleep_us: 0, hardware_sleep_counts_to_us: 523_986_009_990 });
    machine("sleep's failures by step, in the order taken", slept.failedAt,
        [{ step: "freeze", count: 2 }, { step: "resume", count: 1 }]);
    machine("sleep's other modes", slept.others, ["deep"]);
    machine("sleep's hardware time in seconds", [slept.lastHardware, slept.countsTo],
        [3600, 523986.00999]);
    const sleepErrors = { EBUSY: "a device was busy" };
    const sleepTold = Power.sleepSaid(slept, sleepWords, Interfaces.lasted, sleepErrors);
    machine("sleep's mode and the step it last failed at", [sleepTold.side, sleepTold.note],
        ["s2idle", "failed while freezing \u00b7 2 times"]);
    machine("sleep's drawer, the failures the note doesn't say, the device last",
        sleepTold.facts, [
        { id: "sleep:others", value: "deep" },
        { id: "sleep:ways", value: "freeze \u00b7 mem \u00b7 disk" },
        { id: "sleep:slept", value: "12 times" },
        { id: "sleep:failed", value: "1 while resuming" },
        { id: "sleep:error", value: "a device was busy" },
        { id: "sleep:hardware", value: "1 h" },
        { id: "sleep:counts-to", value: "6 d 1 h" },
        { id: "sleep:device", value: "0000:00:14.0", identifies: true },
        { id: "sleep:code", value: "EBUSY", identifies: true }
    ]);
    const neverFailed = Power.sleepSaid(Power.sleepOf({ mode: "s2idle", modes: ["s2idle"],
        successes: 1, failures: 0, failed_at: {}, last_failed_step: "",
        last_failed_errno: null, last_hardware_sleep_us: null }), sleepWords, Interfaces.lasted,
        sleepErrors);
    machine("sleep that never failed says only its mode",
        [neverFailed.note, neverFailed.facts],
        ["", [{ id: "sleep:slept", value: "1 time" }]]);
    machine("sleep before the daemon answers", Power.sleepOf(null), null);
    machine("an error Linux names", Power.errorOf(-5), "EIO");

    // What keeps the machine awake, from the daemon's awake:main:
    // a call holding idle off, a download holding sleep, Keep Awake's
    // own two locks left out, two delays, one of them twice, and a
    // sleep of six hours. Its words are Keep Awake's own, so a word
    // the drawer needs and the vocabulary lacks fails here.
    const awakeWords = vocabulary.find(thing => thing.id === "session:awake").said;
    const awakeAt = 1_790_000_000_000;
    const clock = at => `${(at - awakeAt) / 60_000} min`;
    const logind = { idle: false, idle_since_ms: awakeAt - 600_000, preparing: false,
        idle_action: "suspend", idle_action_ms: 300_000, blocked: ["idle", "sleep"],
        delayed: ["sleep"], inhibitors: [
            { what: ["idle"], who: "Google Chrome", why: "Playing video", mode: "block" },
            { what: ["sleep", "idle"], who: "mycelium", why: "holding work",
                mode: "block" },
            { what: ["idle"], who: "Keep Awake", why: "Kept awake from the launcher",
                mode: "block" },
            { what: ["handle-lid-switch"], who: "Keep Awake", why: "", mode: "block" },
            { what: ["sleep"], who: "NetworkManager", why: "NetworkManager needs to",
                mode: "delay" },
            { what: ["sleep"], who: "swayidle", why: "Lock before sleep", mode: "delay" },
            { what: ["sleep"], who: "swayidle", why: "Again", mode: "delay" },
            { what: ["sleep"], who: "1password", why: "Locking", mode: "delay" }],
        last_sleep: { began_at_ms: awakeAt - 7_200_000 - 21_780_000,
            ended_at_ms: awakeAt - 7_200_000 } };
    const awakeTold = plugged => ({ plugged, when: clock,
        lasted: Interfaces.lasted, named: who => ({ "1password": "1Password" })[who] ?? who });
    machine("keep awake's drawer, others' locks first, its own left out",
        Awake.facts(logind, awakeWords, awakeTold(false)), [
        { id: "awake:idle-held",
            value: "Google Chrome: Playing video \u00b7 mycelium: holding work" },
        { id: "awake:sleep-held", value: "mycelium: holding work" },
        { id: "awake:idle-sleep", value: "suspends 5 min after the screens go off" },
        { id: "awake:put-off", value: "NetworkManager \u00b7 swayidle \u00b7 1Password" },
        { id: "awake:slept", value: "-483 min \u00b7 6 h 3 min" }
    ]);
    const idleNow = Object.assign({}, logind, { idle: true, idle_since_ms: awakeAt - 60_000,
        inhibitors: [], last_sleep: { began_at_ms: awakeAt, ended_at_ms: null } });
    // Nobody else holding a lock says none, and a sleep still going
    // says nothing: only how long after the screens go off it sleeps.
    machine("with no other locks it says its span alone, a sleep still going nothing",
        Awake.facts(idleNow, awakeWords, awakeTold(false)),
        [{ id: "awake:idle-sleep", value: "suspends 5 min after the screens go off" }]);
    const idleSaid = (action, plugged) => Awake.facts({ idle_action: action,
        idle_action_ms: 300_000 }, awakeWords, awakeTold(plugged))[0]?.value ?? "";
    machine("idle sleep as logind is set", [idleSaid("lock", true), idleSaid("lock", false),
        idleSaid("ignore", true), idleSaid("hibernate", false), idleSaid("halt", false),
        idleSaid("kexec", false), idleSaid("", false)],
        ["never on the charger \u00b7 it only locks", "never \u00b7 it only locks", "never",
            "hibernates 5 min after the screens go off", "halts 5 min after the screens go off",
            "reboots without its firmware 5 min after the screens go off", ""]);
    // logind answering with nothing Keep Awake shows, as a machine with
    // no idle action, no other locks and no sleep yet does, gives it no
    // drawer, so the row is no thing held for one (80ECO2's bisect of
    // vm-desktop-launcher-reads-mail, ba28c5c1).
    const head = { eyebrow: "Machine", title: "Keep Awake" };
    const label = id => ({ "awake:idle-sleep": "Idle Sleep" })[id] ?? id;
    machine("logind saying nothing to show gives no drawer",
        Awake.drawer(Awake.facts({ inhibitors: [] }, awakeWords, awakeTold(false)), head, label),
        null);
    machine("logind saying something gives its drawer, each fact by its name",
        Awake.drawer(Awake.facts(idleNow, awakeWords, awakeTold(false)), head, label), {
            eyebrow: "Machine", eyebrowNames: true, title: "Keep Awake",
            facts: [{ label: "Idle Sleep", value: "suspends 5 min after the screens go off" }],
            body: ""
        });
    machine("an error with no name kept", Power.errorOf(-71), "\u221271");
    return asked;
}
