// One person per email address, from whatever every source knows of
// them, as People.qml lists them: who they are, how they stand, the
// order they're listed in, and what the drawer says of them. Kept apart
// from the shell so the-people-are-one-person-each runs it in Qt's own
// engine (tests/person.qml).
.pragma library
.import "Order.js" as Order

// An address as it's compared: trimmed and in lower case, since Google
// and Tuple write the same one in different cases.
function address(email) {
    return String(email ?? "").trim().toLowerCase();
}

// Whether a name is only an address, as some sources give for someone
// they know nothing more of, which a real name always wins over.
function bare(name) {
    return name === "" || name.includes("@");
}

// Everyone the sources know, joined into one person each: two are one
// when they share an address, and a source that knows several addresses
// for someone joins them all. Each source is { name, people }, in the
// order they're consulted, and each of its people { emails, name, facts,
// recent, spans }. Each person is { key, email, emails, name, facts,
// recent, spans, from }: what they're known by, their first address, every
// address, the first real name a source gives (else their address), the
// facts each source adds under its own label, whether any source has them
// as someone Joel dealt with lately, when each thing with them is, and the
// sources that know them, in order. Someone a source knows by no address
// but by an id of its own, as Tuple can call a contact by its id, is a
// person of that source alone, known as "<source>:<id>", since nothing
// else could say they're the same; someone known by neither is left out.
function merged(sources) {
    const byAddress = new Map();
    const persons = [];
    for (const source of sources) {
        for (const known of source.people ?? []) {
            const emails = (known.emails ?? []).map(address).filter(email => email !== "");
            if (emails.length === 0) {
                const id = String(known.id ?? "");
                if (id !== "") {
                    take(fresh("", persons, `${source.name}:${id}`), source.name, known, []);
                }
                continue;
            }
            const found = [...new Set(emails.map(email => byAddress.get(email)).filter(Boolean))]
                .sort((a, b) => persons.indexOf(a) - persons.indexOf(b));
            const person = found[0] ?? fresh(emails[0], persons);
            for (const other of found.slice(1)) {
                joined(person, other);
            }
            take(person, source.name, known, emails);
            for (const email of person.emails) {
                byAddress.set(email, person);
            }
        }
    }
    return persons.filter(person => !person.gone).map(person => ({
        key: person.key,
        email: person.email,
        emails: person.emails,
        name: bare(person.name) ? person.email || person.key : person.name,
        facts: person.facts,
        recent: person.recent,
        spans: person.spans,
        from: person.from
    }));
}

function fresh(email, persons, key = email) {
    const person = {
        key, email, emails: [], name: "", facts: [], recent: false, spans: [], from: []
    };
    persons.push(person);
    return person;
}

// What one source says of someone, added to the person they are.
function take(person, from, known, emails) {
    for (const email of emails) {
        if (!person.emails.includes(email)) {
            person.emails.push(email);
        }
    }
    const name = String(known.name ?? "").trim();
    if (bare(person.name) && !bare(name)) {
        person.name = name;
    }
    for (const fact of known.facts ?? []) {
        if (fact.value && !person.facts.some(had => had.label === fact.label)) {
            person.facts.push(fact.zone
                ? { label: fact.label, value: fact.value, zone: true }
                : { label: fact.label, value: fact.value });
        }
    }
    person.recent = person.recent || Boolean(known.recent);
    // When each thing with them is, by any source, from its start to its
    // end, in milliseconds since the epoch.
    for (const span of known.spans ?? []) {
        if (Number.isFinite(span.from) && Number.isFinite(span.to)) {
            person.spans.push({ from: span.from, to: span.to });
        }
    }
    if (from !== null && !person.from.includes(from)) {
        person.from.push(from);
    }
}

// Two found to be one, the later kept in the earlier.
function joined(person, other) {
    take(person, null, other, other.emails);
    for (const from of other.from) {
        if (!person.from.includes(from)) {
            person.from.push(from);
        }
    }
    other.gone = true;
}

// How someone stands, from every source that says, each source's
// standing being its stands by address, or by what a person of its own
// alone is known as: { said, order, favourite, recent, mark }, as Tuple
// says "available", 0, and Slack "active", 0, with a status's emoji as
// its mark. The note says what most limits reaching them now, whichever
// source knows it (VKCF3Z, e633d330): in a call or a huddle, then away,
// then available or active, then offline, which only says a program is
// closed; a tie goes to the source asked first. The first mark follows
// it. They're a favourite, or recent, if any source says so. Nobody says
// anything of most, who come after everyone who's said to stand anyhow.
const UNSAID_ORDER = 4;
const UNSAID = { said: "", order: UNSAID_ORDER, favourite: false, recent: false };
// How much each order limits reaching them, by order: available 0, busy
// 1, away 2, offline 3, unsaid 4.
const LIMITS = [2, 4, 3, 1, 0];

function standing(person, stands) {
    const known = person.emails.length > 0 ? person.emails : [person.key];
    const said = [];
    for (const by of stands) {
        const stood = known.map(email => by[email]).find(Boolean);
        if (stood) {
            said.push(stood);
        }
    }
    if (said.length === 0) {
        return UNSAID;
    }
    const limit = stood => (stood.said ? LIMITS[stood.order ?? UNSAID_ORDER] ?? 0 : 0);
    const most = said.reduce((held, stood) => (limit(stood) > limit(held) ? stood : held));
    const mark = said.map(stood => stood.mark).find(Boolean);
    return {
        said: [most.said, mark].filter(Boolean).join(" "),
        order: most.said ? (most.order ?? UNSAID_ORDER) : UNSAID_ORDER,
        favourite: said.some(stood => stood.favourite === true),
        recent: said.some(stood => stood.recent === true)
    };
}

// When the last thing between them and Joel was, in milliseconds since
// the epoch: the end of a meeting they had, now for one under way, or
// when a mail came; or null when there's been nothing yet, as for
// someone only asked to a meeting to come.
function last(person, now) {
    const past = (person.spans ?? []).filter(span => span.from <= now)
        .map(span => Math.min(span.to, now));
    return past.length === 0 ? null : Math.max(...past);
}

// Whether they're listed with nothing typed: a favourite, someone there's
// been something with, or someone a source says Joel dealt with lately.
// Everyone else is found by typing.
function listed(person, stood, now) {
    return stood.favourite || stood.recent || person.recent || last(person, now) !== null;
}

// Those listed in order: favourites, then by the last thing between them
// and Joel, newest first, then by name. How they stand isn't part of it,
// since it changes while the list is open; it's said in their note.
// It's measured from now only as they're ordered, so the minute passing
// moves nobody under the keys. Each one's measures are read once, before
// the sort, rather than at each of its comparisons: read there, how each
// stands and their last thing were worked out twice a comparison, and
// ordering 500 took some 250 ms of a launcher's opening in a VM
// (2026-11-10).
function ordered(persons, standOf, now) {
    const measured = persons.map(person => ({ person, first: standOf(person).favourite,
        last: last(person, now), pieces: Order.pieces(person.name) }));
    return Order.sorted(measured, Order.returned({ first: one => one.first,
        last: one => one.last, pieces: one => one.pieces })).map(one => one.person);
}

// When the order and who's listed next change with the clock alone: the
// first moment after now that something with someone starts or ends,
// or Infinity when nothing will. Until then, with everyone and how they
// stand the same, those listed and their order are as they were.
function changesAt(persons, now) {
    const ends = persons.map(person => (person.spans ?? [])
        .map(span => (span.from > now ? span.from : span.to))
        .filter(at => at > now));
    return Math.min(Infinity, ...[].concat(...ends));
}

// Their details, as the launcher's drawer shows them beside their row:
// their name, every address they have, and each fact a source gave. A
// fact that's their time zone, by tzdata's name (`zone`), is followed by
// the time there, as `there` tells it for that zone, when it can.
function details(person, there = () => "") {
    const addresses = person.emails.length === 0 ? [] : [{
        label: person.emails.length === 1 ? "Email" : "Emails",
        value: person.emails.join(", ")
    }];
    const facts = [];
    // One the daemon lists comes with no facts of its own: they're in the
    // drawer it serves.
    for (const fact of person.facts ?? []) {
        facts.push({ label: fact.label, value: fact.value });
        const now = fact.zone ? there(fact.value) : "";
        if (now !== "") {
            facts.push({ label: "Time There", value: now });
        }
    }
    return {
        title: person.name,
        facts: addresses.concat(facts),
        body: ""
    };
}

// Every source's acts for them in the one order: the main ways to reach
// someone first, in the order the sources are consulted (Write in Slack,
// then Call Them, then Write a Mail), and then each source's others.
function acts(each) {
    const all = [].concat(...each);
    return all.filter(act => act.main).concat(all.filter(act => !act.main));
}

// What a thing Joel had with someone is: a meeting or a call they were
// both on, or something they wrote each other, as a source's `recent`
// tells it (PersonSource.qml). One that says no kind was written.
const MET = "met";
const WROTE = "wrote";

// What's between Joel and someone, as their page lists it in place of one
// Recent, which named when rather than what (Joel, 2026-11-11: "the use
// of recent in contacts ... is ambiguous"; VKCF3Z, 0cea8a93): what waits
// (`waiting`), the meetings and calls they shared (`met`), and what they
// wrote each other (`messages`). `had` is each source's recent things and
// `waits` each source's waiting ones, each { key, at, row } with a recent
// one's `kind`, and `once` is Recent.js's listed, which lists a thing
// once, newest first, nine at most. What waits isn't listed again in
// Messages.
function between(had, waits, once) {
    const waiting = once(waits, []);
    const of = kind => had.map(source => (source ?? [])
        .filter(one => (one.kind ?? WROTE) === kind));
    return {
        waiting,
        met: once(of(MET), []),
        messages: once(of(WROTE), waiting.map(one => one.key))
    };
}

// What Last Met says at its side, of the newest it holds: when it was, as
// the launcher says when (`when`, When.js's came), and that it was the two
// of them, where it was: "Tuesday \u00b7 1:1", by `said.one`.
function metSide(newest, when, said) {
    const at = when(newest.at);
    return newest.one === true ? String(said.one).replace("{when}", at) : at;
}

// What Messages says at its side, of the newest it holds: who wrote last
// and when, "they wrote \u00b7 14:02" or "you wrote \u00b7 yesterday", by
// `said.theirs` and `said.mine`.
function wroteSide(newest, when, said) {
    return String(newest.mine === true ? said.mine : said.theirs)
        .replace("{when}", when(newest.at));
}

// What a thing waiting says at its side, standing alone: that it needs
// him, or that he wrote last and waits on them, what it is and when,
// "needs you \u00b7 mail \u00b7 14:02", by `said.needs` and `said.sent`,
// what it is by its `what` in `said` or as the source says it.
function waitSide(entry, when, said) {
    const what = said[entry.what] ?? entry.what ?? "";
    return String(entry.mine === true ? said.sent : said.needs).replace("{what}", what)
        .replace("{when}", when(entry.at));
}

// What a thing says at its side standing alone, in place of the heading
// that would have held it, since nothing above it says why it's there
// (VKCF3Z, 17e0f150): its heading's side, in `said.alone` where it has
// one, "met \u00b7 {side}".
function aloneSide(side, said) {
    return String(said.alone ?? "{side}").replace("{side}", side);
}

// Someone's time at their row's side, only where it's apart from Joel's
// (VKCF3Z, 0cea8a93), since the same time says nothing: where they are,
// by their zone's last part ("Europe/London" is London), and the time
// there, `time`, by `said`'s {place} and {time}, "London \u00b7 17:02".
// `apart` is how many minutes their time is from his.
function awaySide(zone, apart, time, said) {
    if (apart === 0) {
        return "";
    }
    const place = String(zone).split("/").pop().replace(/_/gu, " ");
    return String(said).replace("{place}", place).replace("{time}", time);
}

// The first of each for their drawer, ahead of its facts (VKCF3Z,
// 0cea8a93), what needs him first: the oldest thing waiting, their next
// meeting, `coming` as its row is named and says when, and the newest
// they met at, each "Lunch on Friday \u00b7 2 h ago", when said by `when`
// and labelled by `labels`' waiting, next and met. `told` is each
// source's firsts, { met, wait }, either null.
function firsts(told, coming, when, labels) {
    const each = field => told.map(one => one?.[field]).filter(Boolean);
    const wait = each("wait").reduce((oldest, one) => oldest && oldest.at <= one.at
        ? oldest : one, null);
    const met = each("met").reduce((newest, one) => newest && newest.at >= one.at
        ? newest : one, null);
    const said = (name, side) => side ? `${name} \u00b7 ${side}` : name;
    return [
        wait ? { label: labels.waiting, value: said(wait.name, when(wait.at)) } : null,
        coming ? { label: labels.next, value: said(coming.name, coming.side) } : null,
        met ? { label: labels.met, value: said(met.name, when(met.at)) } : null
    ].filter(Boolean);
}

// How one of those stands on their page: not at all while it holds
// nothing; the one thing itself where it holds one, as `alone` makes it,
// so Return does its act with no step between (VKCF3Z, 0cea8a93, after
// #3519's rule that a row never opens on one thing); and else a row
// stepping into them all, as `holding` makes it.
function placed(entries, holding, alone) {
    if (entries.length === 0) {
        return [];
    }
    return entries.length === 1 ? [alone(entries[0])] : [holding(entries)];
}
