// What Tuple's links say, and how a call is put in words, for Tuple.qml
// and Call.qml, and how Recent's calls read, for Calls.qml. Kept apart
// from Quickshell's types, so the `the-shell-follows-tuple-calls` check
// can run it in Qt's own engine (tests/tuple.qml). How a call stands, and
// which calls Recent keeps, is tuple-call's to say (call/).
.pragma library
.import "When.js" as When

const MINUTE = 60_000;
const HOUR_MINUTES = 60;
const DIGITS = 2;

// A call link, as tuple call join takes one.
const CALL_LINK = /https:\/\/(?:production\.)?tuple\.app\/c\/[A-Za-z0-9_-]+/;

// The first call link in some text, as tuple call join takes one, or "".
function link(text) {
    const found = CALL_LINK.exec(text ?? "");
    return found ? found[0] : "";
}

// The people in a call as a sentence says them, from their names, "" for
// one not known: "with Aadi", "with Aadi and Ben", "with Aadi and one other",
// "with 3 others", or "on your own".
function company(names) {
    if (names.length === 0) {
        return "on your own";
    }
    const known = names.filter(name => name !== "");
    const rest = names.length - Math.min(known.length, 1);
    if (known.length === 0) {
        return names.length === 1 ? "with someone" : `with ${names.length} others`;
    }
    if (names.length === 2 && known.length === 2) {
        return `with ${known[0]} and ${known[1]}`;
    }
    if (rest === 0) {
        return `with ${known[0]}`;
    }
    return `with ${known[0]} and ${rest === 1 ? "one other" : `${rest} others`}`;
}

// How long a call has gone on, as a sentence says it.
function length(minutes) {
    if (minutes < 1) {
        return "under a minute";
    }
    if (minutes === 1) {
        return "a minute";
    }
    if (minutes < 60) {
        return `${minutes} minutes`;
    }
    const hours = `${Math.floor(minutes / 60)} h`;
    return minutes % 60 === 0 ? hours : `${hours} ${minutes % 60} min`;
}

// Words with what's in braces filled in: "{first} and {second}".
function filled(words, fill) {
    let said = String(words ?? "");
    for (const key of Object.keys(fill)) {
        said = said.replace(`{${key}}`, String(fill[key]));
    }
    return said;
}

// Recent's calls as tuple-call says them, newest first by when each began,
// as the times their rows say read down, any that isn't a call left out.
// tuple-call keeps nine, as many as a program's Recent lists.
function newestFirst(recent) {
    return (recent ?? [])
        .filter(past => past && Array.isArray(past.people) && typeof past.since === "number"
            && typeof past.ended === "number")
        .sort((a, b) => b.since - a.since);
}

// Who a past call was with, as its row is named, from tuple-call's people,
// each by their name or else their address, in `said`'s words: "Ana",
// "Ana and Ben", "Ana and one other", "Ana and 2 others", and `someone`
// for whoever Tuple never named: "Someone", "Someone and 2 others".
function who(people, said) {
    const named = people.map(one => one.name || one.email || "").filter(name => name !== "");
    const known = named.length > 0 ? named : [said.someone];
    if (people.length <= 1) {
        return known[0];
    }
    if (people.length === 2 && known.length === 2) {
        return filled(said.two, { first: known[0], second: known[1] });
    }
    const rest = people.length - 1;
    return filled(rest === 1 ? said.other : said.others, { first: known[0], count: rest });
}

// The act inside a past call's row that calls them again, first inside
// it so Return twice calls, as a person's row does (VKCF3Z, 2026-11-08):
// "Call Ana Again" for one, by the name Tuple gave them, and "Call Them
// Again" for several or for one Tuple named only by address.
function again(people, said) {
    const name = people.length === 1 ? String(people[0].name ?? "") : "";
    return name === "" ? said.againAll : filled(said.again, { name });
}

// How long a past call went on, from when it connected to its end, in
// `said`'s words: "under a minute", "42 min", "1 h", "1 h 5 min".
function lasted(since, ended, said) {
    const minutes = Math.floor(Math.max(0, ended - since) / MINUTE);
    if (minutes < 1) {
        return said.under;
    }
    if (minutes < HOUR_MINUTES) {
        return filled(said.minutes, { minutes });
    }
    const left = minutes % HOUR_MINUTES;
    return filled(left === 0 ? said.hours : said.hoursMinutes,
        { hours: Math.floor(minutes / HOUR_MINUTES), minutes: left });
}

// The time of day on the clock here, "14:10".
function clock(time) {
    const at = new Date(time);
    const two = number => String(number).padStart(DIGITS, "0");
    return `${two(at.getHours())}:${two(at.getMinutes())}`;
}

// When a past call began, as the launcher says when (When.js): its time
// alone today, else its day before it, "yesterday 14:10", "Monday 09:05",
// "28 Oct 16:40".
function began(time, now, said) {
    return [When.day(time, now, said), clock(time)].filter(part => part !== "").join(" ");
}

// A past call's note, after its name: how long it went on, or that it
// was missed in place of a length it never had, then when it began:
// "42 min \u00b7 yesterday 14:10", "missed \u00b7 09:12".
function note(past, now, said) {
    const long = past.missed ? said.missed : lasted(past.since, past.ended, said);
    return `${long} \u00b7 ${began(past.since, now, said)}`;
}

// Who rang whom on a past call, in `said`'s words: "missed" for a ring
// Joel didn't answer; "you called" where tuple-call saw Joel begin it, by
// opening his room; "Ada Brightwater called" where it followed her ring,
// by the name Tuple gave her, as Call Ada Brightwater Again says it; and
// "" where tuple-call saw neither, as when Joel called through niri or
// the call was kept before it heard who rang (call/src/lib.rs, began),
// since a guess would say it as surely as what was seen.
function rang(past, said) {
    if (past.missed) {
        return said.missed;
    }
    if (past.caller === "") {
        return said.youCalled;
    }
    const caller = typeof past.caller === "string"
        ? past.people.find(one => String(one.id) === past.caller) : undefined;
    return caller === undefined ? "" : filled(said.called, { name: who([caller], said) });
}

// A past call's drawer, each thing said once (VKCF3Z, 2026-11-08): over
// its name where it was and what it is, as every eyebrow says them, "on
// Tuple \u00b7 call" or "missed call", in Note since Tuple's name is the
// world's, when it was being the row's note's; who rang whom first, left out where it isn't known; With, by name, only for
// two or more, since the name already says one; and their addresses, as
// Tuple knew them. How long it went on and when it ended are left to the
// row's note, "9 min \u00b7 07:19", and so is how long a missed call
// rang, which only says again that nobody answered. Tuple's stream holds
// nothing else of a call Joel reads: its id, its sessions' and whether
// Tuple's server carries it are Tuple's own, as is Tuple's id for each,
// a number Joel never sees in Tuple, used only to call one who has no
// address. `said` gives the labels too.
function details(past, said) {
    const names = past.people.map(one => one.name).filter(name => name !== "");
    const emails = past.people.map(one => one.email).filter(email => email !== "");
    const facts = [];
    const whoRang = rang(past, said);
    if (whoRang !== "") {
        facts.push({ label: said.rang, value: whoRang });
    }
    if (past.people.length >= 2 && names.length > 0) {
        facts.push({ label: said.with, value: names.join(", ") });
    }
    if (emails.length > 0) {
        facts.push({ label: emails.length === 1 ? said.email : said.emails,
            value: emails.join(", ") });
    }
    return {
        eyebrow: past.missed ? said.onMissed : said.onCall,
        eyebrowNames: true,
        title: who(past.people, said),
        facts,
        body: ""
    };
}
