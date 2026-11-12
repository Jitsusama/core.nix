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

// The words beyond its name a served row is found by, in one text, or "":
// the daemon's `keywords`, a list or a text, then the place's own, a text
// or one already made ready to match (its `text`). The launcher's search
// matches only words made ready, so a row's are made from this once
// (ReadyRows.qml's thing); kept as given, a text was never searched.
function keywordsOf(served, own) {
    const said = one => {
        if (Array.isArray(one)) {
            return one.filter(word => typeof word === "string").join(" ");
        }
        if (typeof one === "string") {
            return one;
        }
        return typeof one?.text === "string" ? one.text : "";
    };
    return [said(served), said(own)].join(" ").trim();
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
    on: (time, now, said) => When.onDay(time.at, now, said),
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

// A row's note as the shell says it: words of the daemon's, or one item or
// a list of them, said as a side is.
function noteSaid(note, now, said) {
    return typeof note === "string" ? note : sideSaid([].concat(note), now, said);
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
// identifies the thing (drawn in the Name face), the role it warns in, its
// meter beside it, and whether it says words rather than a name or a figure,
// drawn in the words face.
function factOf(fact, now, said) {
    if (fact.heading !== undefined) {
        return { heading: fact.heading };
    }
    const drawn = { label: fact.name ?? "", value: itemSaid(fact, now, said) };
    for (const field of ["qualifier", "identifies", "hue", "figure", "words"]) {
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
    // What a drawer shows and the eyebrow's parts each pass through only
    // where served: a preview's path and size, never its bytes; parts
    // that, where given, are drawn over the eyebrow's one text; and, as
    // Drawer.qml reads a notice's, a body styled where it carries markup
    // or links, and its picture.
    for (const field of ["eyebrow", "preview", "eyebrowItems", "styled", "picture"]) {
        if (drawer[field] !== undefined && drawer[field] !== null) {
            drawn[field] = drawer[field];
        }
    }
    // An eyebrow that names something from the world, a budget's, set as
    // it's written rather than in the launcher's capitals.
    if (drawer.eyebrowNames === true) {
        drawn.eyebrowNames = true;
    }
    // A body that's prose, a Slack message's, set as every drawer's
    // sentences are (Drawer's `prose`), only where the daemon says so.
    if (drawer.prose === true) {
        drawn.prose = true;
    }
    return drawn;
}

// The forms that read differently only once the day turns: a moment, a
// date and a day. Every other one can as the minute does.
const DAILY = new Set(["moment", "dated", "day", "on"]);

// How often a row says something new of the clock, at its side or in its
// drawer, so it's said again only then while it's drawn: "minute", "day",
// or "" for a row that says no time.
function ticks(row) {
    const note = row?.note && typeof row.note === "object" ? [].concat(row.note) : [];
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

// What the row of `key` and its parts are made of, as text: its record,
// the drawer it names apart, and the records of the rows inside it, which
// are its parts where it steps in. The same text says its parts are as they
// were, so a closing keeps their spots (Spots.js); null where it holds no
// row.
function stampOf(keys, held, key) {
    const row = rowOf(held[key]);
    if (row === null) {
        return null;
    }
    const drawer = typeof row.drawer === "string" ? held[row.drawer] ?? null : null;
    const parts = inside(keys, held, key).map(one => held[one]);
    return JSON.stringify([held[key], drawer].concat(parts));
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
    if (typeof act?.key !== "string" || act.key === "" || misformed(act) !== "") {
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
    if (typeof act.go === "string" && act.go !== "") {
        return "go";
    }
    return act.step === "drawer" || act.step === "rows" ? "step" : "";
}

// The program an open act names its link opened in, by its desktop
// entry's id (`in`), or "" for none, which opens as any link does.
function openedIn(act) {
    return actForm(act) === "open" && typeof act.in === "string" ? act.in : "";
}

// A row's acts the shell can do, in the order served, Return's first. One
// opening in a program is offered only where `has` says that program is
// here, never opened elsewhere in its place (Y2YBTS, de23ba21).
function actsOf(row, has) {
    return (row?.acts ?? []).filter(act => actForm(act) !== ""
        && (openedIn(act) === "" || (has?.(openedIn(act)) ?? false)));
}

// What opens an open act's link: the command of the program it names,
// `commandOf`'s, with the link after it, as a desktop entry's %u takes
// it; xdg-open where it names none; or null where its program's gone.
function opening(act, commandOf) {
    const program = openedIn(act);
    if (program === "") {
        return ["xdg-open", act.open];
    }
    const command = commandOf(program);
    return command ? Array.from(command).concat([act.open]) : null;
}

// What an open act does at Return: `run`s its opening, or, where its
// program went after its rows were listed, runs nothing and says `why` in
// words.json's field.becoming, naming the program as it was shown
// (`program`), as a refused ask does, so it never fails silently (Y2YBTS
// on #3313, VKCF3Z's words, 2026-11-11).
function opened(act, commandOf, words) {
    const run = opening(act, commandOf);
    if (run !== null) {
        return { run, why: "" };
    }
    return { run: null, why: (words.missing ?? "").replace("{program}", act.program ?? "") };
}

// Why an act can't be done as it's served, or "" where it can: one that
// shuts the launcher holds nothing drawn, so it can't say what its row
// becomes.
function misformed(act) {
    return act?.closes === true && act.becomes !== undefined ? "closes and becomes" : "";
}

// Whether an ask shuts the launcher as it's asked, as a paste does, so the
// keys go back to the window under it before the daemon presses any.
function closes(act) {
    return actForm(act) === "ask" && act.closes === true;
}

// How the launcher leaves once an act is done. A step into rows or an ask
// keeps it open, so Joel sees what it became; a copy or an open closes it.
// An ask that shuts it (a paste) or brings its sender forward with a token,
// which niri gives only while the launcher holds the keys, is done at once in
// Return's own turn, the keys given back the next frame (Launcher.qml's
// choose). Going to a program, or opening a link, is done once the launcher's
// gone, since niri focuses a window only then, so its window takes the keys.
function leaving(act) {
    const now = closes(act) || (actForm(act) === "ask" && act.token === true);
    return {
        stays: act?.step === "rows" || (actForm(act) === "ask" && !now),
        now,
        later: actForm(act) === "go" || actForm(act) === "open"
    };
}

// The words a row whose Return asks is read by in the key language, its act's
// (CommandBinding.wordsFor), so Firmware's Install Now writes `install!` rather
// than running at the first Return; "" for any other, and for an ask that
// shuts the launcher, done in Return's own turn as Super+V's paste is, since
// read by its words, Return from the find would land on it.
function wordsOf(act) {
    return actForm(act) === "ask" && !closes(act) ? act.key : "";
}

// A row's mark as drawn: words.json's glyph role it names (`roles`, the theme's glyphs),
// or a mark served as `{ code }`, code points in hex a space apart so a joined emoji keeps
// each, made its characters once as the row is made (3093d93d, 477466fe); "" for any other.
const HEX = 16;
function glyphOf(glyph, roles) {
    if (typeof glyph === "string") {
        return roles[glyph] ?? "";
    }
    const points = String(glyph?.code ?? "").split(" ").map(one => Number.parseInt(one, HEX));
    return points.every(one => one > 0) ? String.fromCodePoint(...points) : "";
}

// Where an act going to a program takes Joel (ready-rows.md's `go`, its
// desktop entry id, served only where the daemon found its entry): its newest
// window, from `byApp`, Niri.qml's windows by lowercased app id, else `entry`,
// its desktop entry as the shell knows it, looked up only at Return, to open;
// null where there's neither, so Return does nothing.
function going(act, byApp, entry) {
    const window = byApp[String(act?.go ?? "").toLowerCase()] ?? null;
    if (window !== null) {
        return { window: window.id };
    }
    return entry ? { entry } : null;
}

// The ask an act the shell does itself sends once it's done, its `also`,
// as a notice's going to its program reads it; null for any other act,
// an ask's own being its ask (Y2YBTS, 2026-11-11, a9bce629).
const DONE_HERE = new Set(["go", "open", "copy"]);
function alsoOf(act) {
    return DONE_HERE.has(actForm(act)) && act.also !== null && typeof act.also === "object"
        ? act.also : null;
}

// An ask as it's sent: with the activation token the act was given where
// it carries `token: true`, so its sender can bring its window forward;
// null where it needs one and none came, so it isn't sent without.
function tokened(act, token) {
    if (act.token !== true) {
        return act.ask;
    }
    return typeof token === "string" && token !== ""
        ? Object.assign({}, act.ask, { token }) : null;
}

// An act's row as it's said: `worded`, its key's words.json name and glyph,
// with nothing at its side, but for words from outside the vocabulary, which
// the daemon serves with it: a `name`, as a notice's offer in its sender's
// words, a `side`, as a link's host, and a `glyph`, a role or `{ code }`. A
// verb whose words are the launcher's has a key of its own instead (Y2YBTS,
// 2026-11-11, a9bce629). `roles` are the theme's glyphs.
function actWorded(act, worded, roles) {
    return {
        name: typeof act.name === "string" && act.name !== "" ? act.name : worded.name,
        glyph: act.glyph === undefined ? worded.glyph : glyphOf(act.glyph, roles),
        side: typeof act.side === "string" ? act.side : ""
    };
}

// What each of a row's acts is kept by as a row of its own, since the launcher
// holds its selection by key: its key where the row holds it once; where two
// share one, as two links' "Open the Link", the key and its target, so one
// going leaves the cursor on the other, and an act with no target its place
// (Y2YBTS, 2026-11-11). A key in `repeated` stays told apart once held once,
// or the last link left would take its bare key and the cursor go from it;
// each key repeated now is added there.
function actKeys(acts, repeated) {
    const times = {};
    for (const one of acts) {
        times[one.key] = (times[one.key] ?? 0) + 1;
    }
    for (const key of Object.keys(times)) {
        if (times[key] > 1) {
            repeated[key] = true;
        }
    }
    return acts.map((one, at) => {
        if (repeated[one.key] !== true) {
            return one.key;
        }
        const target = [one.open, one.go, one.copy, one.ask?.action]
            .find(each => typeof each === "string" && each !== "");
        return `${one.key}:${target ?? at}`;
    });
}

// The verb the foot says for an act: the name the daemon served it with,
// as Unmute on a muted switch, lowercased as the foot says verbs; else
// words.json's for its key, `worded`; else the one every act of its form
// says (Y2YBTS, 2026-11-11: one shape, the daemon's words).
function verbOf(act, worded) {
    if (typeof act.name === "string" && act.name !== "") {
        return act.name.toLowerCase();
    }
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
