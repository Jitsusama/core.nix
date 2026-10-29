// One person per email address, from whatever every source knows of
// them, as People.qml lists them: who they are, how they stand, the
// order they're listed in, and what the drawer says of them. Kept apart
// from the shell so the-people-are-one-person-each runs it in Qt's own
// engine (tests/person.qml).
.pragma library

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

// How someone stands, from the first source that says, each source's
// standing being its stands by address, or by what a person of its own
// alone is known as: { said, order, favourite }, as Tuple says
// "available", 0. Nobody says anything of most, who come after everyone
// who's said to stand anyhow.
const UNSAID_ORDER = 4;
const UNSAID = { said: "", order: UNSAID_ORDER, favourite: false, recent: false };

function standing(person, stands) {
    const known = person.emails.length > 0 ? person.emails : [person.key];
    for (const by of stands) {
        for (const email of known) {
            const stood = by[email];
            if (stood) {
                return Object.assign({}, UNSAID, stood);
            }
        }
    }
    return UNSAID;
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
// moves nobody under the keys.
function ordered(persons, standOf, now) {
    const rank = person => (standOf(person).favourite ? 0 : 1);
    const when = person => last(person, now) ?? Number.MIN_SAFE_INTEGER;
    return persons.slice().sort((a, b) => rank(a) - rank(b)
        || when(b) - when(a)
        || a.name.localeCompare(b.name));
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
    for (const fact of person.facts) {
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
// someone first, in the order the sources are consulted (Call Them, then
// Write a Mail, then Open in Slack), and then each source's others.
function acts(each) {
    const all = [].concat(...each);
    return all.filter(act => act.main).concat(all.filter(act => !act.main));
}
