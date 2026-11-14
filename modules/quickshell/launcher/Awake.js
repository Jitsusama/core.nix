// What Keep Awake's drawer says of the machine daemon's awake:main
// record, what logind holds, in plain JavaScript so a check can run it
// in Qt's own engine, as the shell does: who else keeps the machine from idling,
// sleeping or sleeping with the lid shut, when an idle machine sleeps,
// who puts sleep off a moment, and when it last slept. Session.qml says
// it.
.pragma library

// The locks logind holds against, by the fact that names who holds each:
// going idle, sleeping and the lid's own sleep.
const HELD = [
    ["idle", "awake:idle-held"],
    ["sleep", "awake:sleep-held"],
    ["handle-lid-switch", "awake:lid-held"]
];

// Who holds Keep Awake's own locks (home.nix's `holding`), left out of
// its drawer, since its side already says them.
const KEEPS_AWAKE = "Keep Awake";

const MILLISECONDS = 1000;
const BETWEEN = " \u00b7 ";

// A template's {fields}, filled.
const FIELD = /\{(\w+)\}/g;

function filled(template, fields) {
    return String(template ?? "").replace(FIELD, (_whole, name) => String(fields[name] ?? ""));
}

// Two names in the order Joel would look for them, whatever their case.
// logind lists its locks in no lasting order, so the drawer says each
// list by name, and an opening reads as the last did (VKCF3Z,
// 2026-11-13).
function byName(one, other) {
    return one.toLowerCase().localeCompare(other.toLowerCase());
}

// Each lock logind lists that's held in `mode` against `what`.
function holding(logind, mode, what) {
    return (logind.inhibitors ?? []).filter((lock) => lock.mode === mode
        && (lock.what ?? []).includes(what));
}

// Keep Awake's drawer's facts, each by its words' id and giving way when
// empty, in the order they move most (VKCF3Z, 2026-11-08): everyone else
// blocking idling, sleep or the lid, each who then why, by name, a run
// of whole items; when an idle machine sleeps, as logind is set to
// whether or not it's kept awake; who puts sleep off a moment, by name, who alone,
// since every delay's why says it's before sleep alike; and when the last
// sleep the daemon saw began and how long it lasted. `said` gives the
// words (words.json's Keep Awake), and `told` whether it's on the charger
// (`plugged`), how a moment (`when(at)`, as Captured.js's taken says one)
// and a time (`lasted(seconds)`) are said, and how a holder is named
// (`named(who)`): as its program names itself where the desktop knows it,
// "1Password" for 1password (VKCF3Z, 2026-11-08). Left out: `preparing`,
// never true while the launcher is drawn; `idle` and when it began, since
// logind hears the session idle only once its screens are off, when no
// drawer is seen; and each lock's uid and pid, since who says what they
// would.
function facts(logind, said, told) {
    const held = HELD.map(([what, id]) => ({
        id,
        value: holding(logind, "block", what).filter((lock) => lock.who !== KEEPS_AWAKE)
            .map((lock) => ({ who: told.named(lock.who), why: lock.why }))
            .sort((one, other) => byName(one.who, other.who))
            .map((lock) => (lock.why ? filled(said.held, lock) : lock.who))
            .join(BETWEEN)
    }));
    const delaying = holding(logind, "delay", "sleep").map((lock) => told.named(lock.who));
    return held.concat([
        { id: "awake:idle-sleep", value: idleSleep(logind, said, told) },
        { id: "awake:put-off", value: [...new Set(delaying)].sort(byName).join(BETWEEN) },
        { id: "awake:slept", value: lastSlept(logind.last_sleep ?? {}, told) }
    ]).filter((fact) => fact.value !== "");
}

// Keep Awake's drawer of `shown`, the facts as facts gives them,
// headed by `head`'s eyebrow and title, each fact named by
// `named(id)`, or null while logind says
// nothing it shows: a switch has no drawer, so one held with nothing in
// it is held for a row that is no thing (Launcher.qml's actless
// undrawn, 80ECO2's ba28c5c1).
function drawer(shown, head, named) {
    if (shown.length === 0) {
        return null;
    }
    return {
        eyebrow: head.eyebrow ?? "",
        eyebrowNames: true,
        title: head.title,
        facts: shown.map((fact) => ({ label: named(fact.id), value: fact.value })),
        body: ""
    };
}

// When an idle machine sleeps, as logind's IdleAction and IdleActionUSec
// say: "suspends 5 min after the screens go off"; "never on the charger
// \u00b7 it only locks" where it only locks, as idle-sleep has it on the
// charger; "never" where it ignores idling. An action `said` has no verb
// for is said as logind names it. The screens go off as the session goes
// idle (idle-sleep's nixos.nix), so logind's time counts from then.
function idleSleep(logind, said, told) {
    const action = logind.idle_action ?? "";
    if (action === "") {
        return "";
    }
    if (action === "lock") {
        return told.plugged ? said.locksCharging : said.locks;
    }
    if (action === "ignore") {
        return said.never;
    }
    const does = said[action] ?? action;
    const after = logind.idle_action_ms ?? 0;
    return filled(said.after, { does, lasted: told.lasted(after / MILLISECONDS) });
}

// When the last sleep began and how long it lasted, "02:14 \u00b7 6 h 3
// min", or nothing before the daemon has seen one end.
function lastSlept(slept, told) {
    if (typeof slept.began_at_ms !== "number" || typeof slept.ended_at_ms !== "number") {
        return "";
    }
    return `${told.when(slept.began_at_ms)}${BETWEEN}${
        told.lasted((slept.ended_at_ms - slept.began_at_ms) / MILLISECONDS)}`;
}
