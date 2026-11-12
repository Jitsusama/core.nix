// What Now's own pieces say in their drawers, where the row can't show all
// their source holds (VKCF3Z, 2026-11-08): each where it is and what it is
// above, then what moves most, then what describes it, its identifiers
// last, and a fact it can't say left out, never said as unknown. A piece
// whose place already holds its row shows that place's drawer instead, so
// only the pieces with no place of their own are here: Disk, Notices, Lag,
// Sharing, Recording and Do Not Disturb.
.pragma library
.import "../FileFacts.js" as FileFacts

const FIELD = /\{(\w+)\}/g;
const SIGNIFICANT = 3;
// A stall shorter than this share of lag-watch's window, in percent, is
// too little to say, and gives way as none at all does (VKCF3Z, 2026-11-08).
const LEAST = 0.1;
// lag-watch says swap in mebibytes.
const MEBIBYTE = 1_048_576;
// The holders a disk's drawer names growing, the most first.
const GROWING = 3;
const MINUTE = 60;
const HOUR = 3600;
const DAY = 86_400;
// The slices lag-watch watches, in the order a drawer names them: the
// desktop's own, the programs', then what runs on its own and builds.
const SLICES = ["session", "app", "jobs", "build"];
// Each kind of stall, as lag-watch counts it, in the order a drawer names
// them; words.json's on:lag says each as stall:<kind>.
const STALLS = ["cpu", "memory", "io"];

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

// A share in percent at three figures: 12.0%, 3.00%, 0.500%.
function percent(share) {
    return `${share.toPrecision(SIGNIFICANT)}%`;
}

// How long until something, in its two largest units to the minute, "5 h
// 57 min", "40 min", and past a day in days, as Cameras' Powered says how
// long (VKCF3Z, 2026-11-08: never a clock-like "5:57"). `said` names a day
// and days.
function span(seconds, said) {
    if (seconds >= DAY) {
        const days = Math.round(seconds / DAY);
        return days === 1 ? said.day : filled(said.days, { count: days });
    }
    // Never "0 min": what's due within the minute is a minute away.
    const minutes = Math.max(1, Math.round(seconds / MINUTE));
    const hours = Math.floor(minutes * MINUTE / HOUR);
    const left = minutes % MINUTE;
    if (hours === 0) {
        return `${left} min`;
    }
    return left === 0 ? `${hours} h` : `${hours} h ${left} min`;
}

// The disk short of room most, or the one Now names: how soon it's full and
// how fast it fills, what's growing most, its size, and what the guard
// last cleared. Its room is the row's own side, "19.3 GB free", so it
// isn't said again (VKCF3Z, 2026-11-08). `told` holds the clock (`now`, in seconds), the
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
    // Each holder an item the drawer wraps between, never inside, its
    // path giving way from its middle where alone it's too wide (VKCF3Z,
    // 2026-11-08).
    const growing = (state.holders ?? []).filter(holder => (holder.grew?.bytes ?? 0) > 0)
        .sort((one, other) => other.grew.bytes - one.grew.bytes).slice(0, GROWING)
        .map(holder => {
            const path = told.within(holder.path);
            return { said: filled(said.grew, { path, amount: shown(holder.grew.bytes) }), path };
        });
    const [cleared] = state.cleared ?? [];
    return {
        // The disk's own name is the title, so the eyebrow says only where
        // and what (VKCF3Z, 2026-11-08).
        title: worst.path,
        eyebrow: said.eyebrow,
        facts: drawn([
            fact(named, "machine:disk:full-in", left > 0 ? told.duration(left) : ""),
            fact(named, "on:disk:filling", filling > 0
                ? filled(said.hourly, { amount: shown(filling) }) : ""),
            Object.assign(fact(named, "on:disk:growing",
                growing.map(item => item.said).join(" \u00b7 ")), { valueItems: growing }),
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

// What lag-watch said of its last window: the most any slice waited on
// what keeps them waiting, of how long a window, which every share is of;
// how long each slice stalled on each kind, as a share of it; the swap
// held; and, last, who waited on what, in our own sentence. The row's
// side says what, so the fact says only how long (VKCF3Z, 2026-11-08).
function lag(window, what, told) {
    const { named, said, tells } = told;
    const most = Math.max(0, ...SLICES
        .map(slice => Number(window.stalls?.[slice]?.[`${window.worst}_some`]) || 0));
    const waited = what === "" || most < LEAST ? "" : filled(said.over, {
        share: percent(most),
        seconds: Math.round(Number(window.seconds) || 0)
    });
    const who = window.who === "desktop" ? tells.desktop : tells.programs;
    const stalled = SLICES.filter(slice => window.stalls?.[slice])
        .map(slice => fact(named, `on:lag:${slice}`, STALLS
            .map(kind => [said[`stall:${kind}`] ?? kind,
                Number(window.stalls[slice][`${kind}_some`]) || 0])
            .filter(([, share]) => share >= LEAST)
            .map(([word, share]) => `${word} ${percent(share)}`).join(" \u00b7 ")));
    const swapped = Object.values(window.memory ?? {})
        .reduce((sum, group) => sum + (Number(group.swap_mib) || 0), 0);
    return {
        eyebrow: said.eyebrow,
        facts: drawn([fact(named, "on:lag:waited", waited)].concat(stalled,
            [fact(named, "on:lag:swap",
                swapped > 0 ? FileFacts.weighed(swapped * MEBIBYTE) : "")])),
        body: what === "" ? "" : filled(who, { what })
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
                    { identifies: true })
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
// on, as its words: its figures in told.said and its sentences in
// told.tells. Until when is the row's own side, so it isn't said again.
// Off, as Desktop's row shows it, it says until when Return would hold them
// (`hour`) and whether a shared screen turns it on by itself, so the row
// says more than its line (Joel, 2026-11-11: every row has a drawer).
function quiet(state, told) {
    const { named, said, tells } = told;
    if (state.off) {
        return {
            eyebrow: said.eyebrow,
            facts: drawn([
                fact(named, "session:quiet:60", filled(said.until, { time: state.hour })),
                fact(named, "on:quiet:sharing", state.whileSharing ? said.sharing : said.unshared)
            ]),
            body: tells.off
        };
    }
    const held = state.held === 1 ? said.one : filled(said.many, { count: state.held });
    return {
        eyebrow: said.eyebrow,
        facts: drawn([
            fact(named, "on:quiet:since", state.since),
            fact(named, "on:quiet:held", held)
        ]),
        body: state.kept ? tells.kept : tells.shared
    };
}
