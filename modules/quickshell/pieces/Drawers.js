// What Now's own pieces say in their drawers, where the row can't show all
// their source holds (VKCF3Z, 2026-11-08): each where it is and what it is
// above, then what moves most, then what describes it, its identifiers
// last, and a fact it can't say left out, never said as unknown. A piece
// whose place already holds its row shows that place's drawer instead, so
// only the pieces with no place of their own are here: Disk, Notices, Lag,
// Sharing, Recording and Do Not Disturb.
.pragma library
.import "../launcher/Program.js" as Program

const FIELD = /\{(\w+)\}/g;
const SIGNIFICANT = 3;
// lag-watch says swap in mebibytes.
const MEBIBYTE = 1_048_576;
// The holders a disk's drawer names growing, the most first.
const GROWING = 3;
// The slices lag-watch watches, in the order a drawer names them: the
// desktop's own, the programs', then what runs on its own and builds.
const SLICES = ["session", "app", "jobs", "build"];
// Each kind of stall, as lag-watch counts it, by the word a drawer says it in.
const STALLS = [["cpu", "processor"], ["memory", "memory"], ["io", "disk"]];

// A template's {fields} filled in.
function filled(template, fields) {
    return String(template ?? "").replace(FIELD, (_whole, name) => String(fields[name] ?? ""));
}

// A fact named by its id, left out by drawn() when it says nothing.
function fact(named, id, value) {
    return { label: named(id), value: String(value ?? "") };
}

function drawn(facts) {
    return facts.filter(one => one.value.trim() !== "");
}

// A share in percent at three figures: 12.0%, 3.00%, 0%.
function percent(share) {
    const value = Number(share) || 0;
    return value === 0 ? "0%" : `${value.toPrecision(SIGNIFICANT)}%`;
}

// The disk short of room most, or the one Now names: how soon it's full and
// how fast it fills, what's growing most, its room and size, and what the
// guard last cleared. `told` holds the clock (`now`, in seconds), the
// vocabulary (`named`, `said`), and the guard's own ways of saying sizes,
// durations, paths and times (`shown`, `duration`, `within`, `time`), so
// every figure reads as the Disks place says it.
function disk(state, told) {
    const { named, said, shown } = told;
    const { worst } = state;
    if (!worst) {
        return null;
    }
    const left = typeof worst.full_at === "number" ? worst.full_at - told.now : 0;
    const filling = Number(worst.fill_per_hour) || 0;
    const growing = (state.holders ?? []).filter(holder => (holder.grew?.bytes ?? 0) > 0)
        .sort((one, other) => other.grew.bytes - one.grew.bytes).slice(0, GROWING)
        .map(holder => filled(said.grew, { path: told.within(holder.path),
            amount: shown(holder.grew.bytes) }));
    const [cleared] = state.cleared ?? [];
    return {
        eyebrow: filled(said.eyebrow, { path: worst.path }),
        // A path is said as it's written, not in the eyebrow's capitals.
        eyebrowNames: true,
        facts: drawn([
            fact(named, "machine:disk:full-in", left > 0 ? told.duration(left) : ""),
            fact(named, "on:disk:filling", filling > 0
                ? filled(said.hourly, { amount: shown(filling) }) : ""),
            fact(named, "on:disk:growing", growing.join(" \u00b7 ")),
            fact(named, "machine:disk:free", shown(worst.free)),
            fact(named, "machine:disk:size", shown(worst.size ?? worst.total)),
            fact(named, "on:disk:cleared", cleared ? filled(said.cleared, {
                amount: shown(cleared.bytes),
                path: told.within(cleared.path),
                time: told.time(cleared.at)
            }) : "")
        ]),
        body: String(worst.says ?? "")
    };
}

// The notices waiting, newest first as Notices holds them, a fact for each
// program that sent some, the one that sent last first, saying how many.
function notices(missed, said) {
    const counts = new Map();
    for (const notice of missed) {
        counts.set(notice.app, (counts.get(notice.app) ?? 0) + 1);
    }
    return {
        eyebrow: said.eyebrow,
        facts: [...counts].map(([app, count]) => ({ label: app, value: String(count) }))
    };
}

// What lag-watch said of its last window: what has kept them waiting, how
// long each slice stalled on each kind, the swap held, and its own
// sentences of what counted as waiting.
function lag(window, what, told) {
    const { named, said } = told;
    const stalled = SLICES.filter(slice => window.stalls?.[slice])
        .map(slice => fact(named, `on:lag:${slice}`, STALLS.map(([kind, word]) =>
            `${word} ${percent(window.stalls[slice][`${kind}_some`])}`).join(" \u00b7 ")));
    const swapped = Object.values(window.memory ?? {})
        .reduce((sum, group) => sum + (Number(group.swap_mib) || 0), 0);
    return {
        eyebrow: said.eyebrow,
        facts: drawn([fact(named, "on:lag:waiting", what)].concat(stalled,
            [fact(named, "on:lag:swap", swapped > 0 ? Program.memory(swapped * MEBIBYTE) : "")])),
        body: (window.trouble ?? []).join(" ")
    };
}

// What another program sees of the screen: what it is, as the title,
// whether notices are held meanwhile, whether it's a window or the whole
// screen, and the stream niri sends it, last. `seen` is each cast's
// title, as Sharing says it.
function sharing(casts, seen, told) {
    const { named, said } = told;
    const one = casts.length === 1 ? casts[0] : null;
    return {
        eyebrow: said.eyebrow,
        title: one === null ? filled(said.several, { count: casts.length }) : seen[0],
        facts: drawn(one === null
            ? seen.map(title => fact(named, "on:sharing:what", title))
            : [
                fact(named, "on:sharing:notices", told.held ? said.held : said.shown),
                fact(named, "on:sharing:kind", one.target?.Window ? said.window : said.screen),
                Object.assign(fact(named, "on:sharing:stream", one.stream_id ?? ""),
                    { identifier: true })
            ])
    };
}

// A recording: where it lands and what it is, how long it has run, and
// what it shows.
function recording(length, what, told) {
    const { named, said } = told;
    return {
        eyebrow: filled(said.eyebrow, { where: told.where }),
        facts: drawn([
            fact(named, "capture:length", length),
            fact(named, "on:recording:showing", what)
        ])
    };
}

// Do not disturb: since when, how many notices it has held, and why it's
// on, as its words. Until when is the row's own side, so it isn't said
// again.
function quiet(state, told) {
    const { named, said } = told;
    const held = state.held === 1 ? said.one : filled(said.many, { count: state.held });
    return {
        eyebrow: said.eyebrow,
        facts: drawn([
            fact(named, "on:quiet:since", state.since),
            fact(named, "on:quiet:held", held)
        ]),
        body: state.kept ? said.kept : said.shared
    };
}
