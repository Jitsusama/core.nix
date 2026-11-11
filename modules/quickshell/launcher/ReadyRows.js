// What the shell does with the rows a daemon serves ready, the ready-row
// form (Y2YBTS, 2026-11-10): places each by its `order` as a change names
// it, never sorting the rest, and words what reads the clock, which no
// daemon words or every row would change each minute. Every other word a
// row says is the daemon's. ReadyRows.qml holds a place's rows by it, and
// tests/ready-rows.qml runs it in Qt's own engine.
.pragma library
.import "../When.js" as When
.import "Ago.js" as Ago
.import "Interfaces.js" as Interfaces

const SECOND = 1000;
const MINUTE = 60_000;
// Up to how far ahead a meeting is said as "in 12 min" rather than its time.
const SOON = 3_600_000;
// Between the items of a side, as every side joins them.
const JOINT = " \u00b7 ";

// The row a record holds, as the crate's Kept wraps it, `{ key, kind: "row",
// row }`, or null for a record of another kind, a summary or a head.
function rowOf(record) {
    return record?.kind === "row" && record.row ? record.row : null;
}

// The keys of the rows in their order once a message said the records
// `changed` anew and let those `gone` go, the records held by key in
// `held`: a new list, so whatever reads it is told, and the one before left
// as it was. Only rows are placed; a summary beside them isn't.
function placed(keys, held, changed, gone) {
    const moved = new Set(changed.concat(gone));
    const now = keys.filter(key => !moved.has(key));
    for (const key of changed) {
        const row = rowOf(held[key]);
        if (row && typeof row.order === "number") {
            now.splice(after(now, held, row.order), 0, key);
        }
    }
    return now;
}

// Where a row of order `order` goes among `keys`: after every row ordered
// at or before it, found by halving.
function after(keys, held, order) {
    let low = 0;
    let high = keys.length;
    while (low < high) {
        const middle = Math.floor((low + high) / 2);
        if (rowOf(held[keys[middle]]).order <= order) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    return low;
}

function sameYear(at, now) {
    return new Date(at).getFullYear() === new Date(now).getFullYear();
}

// A meeting as a side says it, from `{ at, until, allDay }`: "now" while
// it's on, "in 12 min" within the hour, else "at 14:00", with its day in
// front past today; an all-day one "today" or its day. One over says when
// it was held, as `came` says it today and as a moment before (VKCF3Z,
// ccccbe5f), never "now", as Calendar.js's when does.
function meeting(time, now, said) {
    const over = typeof time.until === "number" && time.until <= now;
    if (over && !time.allDay) {
        return When.day(time.at, now, said) === "" ? When.came(time.at, now, said)
            : When.moment(time.at, now, said);
    }
    const day = When.day(time.at, now, said);
    if (time.allDay) {
        return day === "" ? said.today : day;
    }
    if (time.at <= now) {
        return said.happening;
    }
    if (time.at - now < SOON) {
        return said.soon.replace("{minutes}", Math.max(1, Math.ceil((time.at - now) / MINUTE)));
    }
    const clock = said.at.replace("{time}", When.clock(time.at));
    return day === "" ? clock : `${day} ${clock}`;
}

// The forms a time travels in (ready-rows.md's Times), each worded at
// `now` in `said`'s words: those When.js takes (now, minutes, yesterday,
// tomorrow, date), and today, happening, soon, at and opened for the forms
// of their own, with `zone(id, now)` for a time somewhere else.
const FORMS = {
    came: (time, now, said) => When.came(time.at, now, said),
    opened: (time, now, said) => said.opened.replace("{when}", When.came(time.at, now, said)),
    meeting: (time, now, said) => meeting(time, now, said),
    moment: (time, now, said) => When.moment(time.at, now, said),
    dated: (time, now, said) => `${said.date(new Date(time.at),
        sameYear(time.at, now) ? "dddd d MMMM" : "dddd d MMMM yyyy")}, ${When.clock(time.at)}`,
    ago: (time, now) => Ago.span(time.at, now),
    lasted: (time, now) => Interfaces.lasted(Math.max(0, now - time.at) / SECOND),
    day: (time, now, said) => said.date(new Date(time.at),
        sameYear(time.at, now) ? "d MMM" : "d MMM yyyy"),
    zone: (time, now, said) => said.zone(time.zone, now)
};

// Whether an item is a time, worded here, rather than words of the daemon's.
function timed(item) {
    return typeof item?.as === "string";
}

// A time as the shell says it at `now`, or "" for a form it doesn't know,
// which ready-rows.md would list first.
function timeSaid(time, now, said) {
    const form = FORMS[time.as];
    return form ? form(time, now, said) : "";
}

// An item of a side, a fact's value or a note: its words, or its time
// worded, put in its words' {time} where they hold one, as "since {time}".
function itemSaid(item, now, said) {
    if (!timed(item)) {
        return String(item?.said ?? "");
    }
    const time = timeSaid(item, now, said);
    const words = item.said;
    return typeof words === "string" && words.includes("{time}") && time !== ""
        ? words.replace("{time}", time)
        : time;
}

// A row's note as the shell says it: words of the daemon's, or one time
// item, as a side's are.
function noteSaid(note, now, said) {
    return typeof note === "string" ? note : itemSaid(note, now, said);
}

// A side's items as one line, each said and joined by a dot, those saying
// nothing left out.
function sideSaid(items, now, said) {
    return sideJoined(sideItems(items, now, said));
}

// A side's items as a row draws them apart, each said, with the width it
// keeps and the role it warns in where it says them, those saying nothing
// left out.
function sideItems(items, now, said) {
    return (items ?? []).map(item => {
        const drawn = { said: itemSaid(item, now, said) };
        for (const field of ["keep", "hue"]) {
            if (item?.[field] !== undefined) {
                drawn[field] = item[field];
            }
        }
        return drawn;
    }).filter(one => one.said !== "");
}

// Side items said, as one line joined by a dot.
function sideJoined(items) {
    return items.map(one => one.said).join(JOINT);
}

// A drawer's fact as Drawer.qml draws one: a heading its own, else named
// and saying its value or its time, with its qualifier, whether it
// identifies the thing (drawn in the Name face) and the role it warns in.
function factOf(fact, now, said) {
    if (fact.heading !== undefined) {
        return { heading: fact.heading };
    }
    const drawn = { label: fact.name ?? "", value: itemSaid(fact, now, said) };
    for (const field of ["qualifier", "identifies", "hue"]) {
        if (fact[field] !== undefined) {
            drawn[field] = fact[field];
        }
    }
    return drawn;
}

// A row's drawer as Drawer.qml draws one, its facts' times worded at `now`,
// or null where the daemon serves none.
function drawerOf(drawer, now, said) {
    if (!drawer) {
        return null;
    }
    const drawn = {
        title: drawer.title ?? "",
        facts: (drawer.facts ?? []).map(fact => factOf(fact, now, said)),
        body: drawer.body ?? ""
    };
    if (drawer.eyebrow !== undefined) {
        drawn.eyebrow = drawer.eyebrow;
    }
    return drawn;
}

// The forms that read differently only once the day turns: a moment, a
// date and a day. Every other one can as the minute does.
const DAILY = new Set(["moment", "dated", "day"]);

// How often a row says something new of the clock, at its side or in its
// drawer, so it's said again only then while it's drawn: "minute", "day",
// or "" for a row that says no time.
function ticks(row) {
    const note = row?.note && typeof row.note === "object" ? [row.note] : [];
    const times = (row?.side ?? []).concat(row?.drawer?.facts ?? [], note).filter(timed);
    if (times.length === 0) {
        return "";
    }
    return times.every(time => DAILY.has(time.as)) ? "day" : "minute";
}

// The keys of the rows inside the row of key `parent`, in their order, or
// with `parent` "" a place's own rows, those inside none: a row inside
// another names it as its `parent`, so the shell walks nothing.
function inside(keys, held, parent) {
    return keys.filter(key => (rowOf(held[key])?.parent ?? "") === parent);
}

// The drawer a row serves: its own, or, where it names one by key, the
// drawer the record beside it holds, `{ key: "drawer:<place>:<id>", kind:
// "drawer", drawer }`, so a drawer served apart changes without its row.
// Null where there's none, or none yet.
function drawerServed(row, held) {
    const drawer = row?.drawer;
    if (typeof drawer === "string") {
        return held?.[drawer]?.drawer ?? null;
    }
    return drawer ?? null;
}

// What a served act asks of the shell, by its form (ready-rows.md's
// acts): "ask", back on the socket; "copy" its words; "open" its link;
// "step", into the row's drawer or the rows inside it; or "" for a form
// the shell doesn't know, which is never guessed at, and so never offered.
function actForm(act) {
    if (typeof act?.key !== "string" || act.key === "") {
        return "";
    }
    if (act.ask !== null && typeof act.ask === "object") {
        return "ask";
    }
    if (typeof act.copy === "string") {
        return "copy";
    }
    if (typeof act.open === "string" && act.open !== "") {
        return "open";
    }
    return act.step === "drawer" || act.step === "rows" ? "step" : "";
}

// A row's acts the shell can do, in the order served, Return's first.
function actsOf(row) {
    return (row?.acts ?? []).filter(act => actForm(act) !== "");
}

// The verb the foot says for an act: words.json's for its key, `worded`,
// else the one every act of its form says.
function verbOf(act, worded) {
    if (worded) {
        return worded;
    }
    const form = actForm(act);
    if (form === "step") {
        return act.step === "rows" ? "more" : "read";
    }
    return form === "copy" || form === "open" ? form : "";
}

// Why an ask wasn't done, in words, or "" where it was: the daemon's own
// words for a refusal, else the launcher's, `said.couldNot`, and
// `said.noAnswer` where no answer came, since nothing fails silently.
function refusal(answer, said) {
    if (answer === null || answer === undefined) {
        return said.noAnswer ?? "";
    }
    if (!answer.error) {
        return "";
    }
    const own = answer.error.said;
    return typeof own === "string" && own !== "" ? own : said.couldNot ?? "";
}
