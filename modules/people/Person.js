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
// recent }. Each person is { email, emails, name, facts, recent, from }:
// their first address, every address, the first real name a source
// gives (else their address), the facts each source adds under its own
// label, whether any source has them as someone Joel dealt with lately,
// and the sources that know them, in order.
function merged(sources) {
    const byAddress = new Map();
    const persons = [];
    for (const source of sources) {
        for (const known of source.people ?? []) {
            const emails = (known.emails ?? []).map(address).filter(email => email !== "");
            if (emails.length === 0) {
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
        email: person.email,
        emails: person.emails,
        name: bare(person.name) ? person.email : person.name,
        facts: person.facts,
        recent: person.recent,
        spans: person.spans,
        from: person.from
    }));
}

function fresh(email, persons) {
    const person = {
        email, emails: [], name: "", facts: [], recent: false, spans: [], from: []
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
            person.facts.push({ label: fact.label, value: fact.value });
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
// standing being its stands by address: { said, order, favourite }, as
// Tuple says "available", 0. Nobody says anything of most, who come
// after everyone who's said to stand anyhow.
const UNSAID_ORDER = 4;
const UNSAID = { said: "", order: UNSAID_ORDER, favourite: false, recent: false };

function standing(person, stands) {
    for (const by of stands) {
        for (const email of person.emails) {
            const stood = by[email];
            if (stood) {
                return Object.assign({}, UNSAID, stood);
            }
        }
    }
    return UNSAID;
}

// Whether they're listed with nothing typed: a favourite, someone dealt
// with lately, or someone a source says is so.
function listed(person, stood) {
    return person.recent || stood.favourite || stood.recent;
}

// How far from now the nearest thing with them is, one under way being
// none; for nothing, further than anything.
function apart(person, now) {
    return Math.min(Number.MAX_SAFE_INTEGER,
        ...(person.spans ?? []).map(span => Math.max(0, span.from - now, now - span.to)));
}

// Those listed in order: favourites, then whoever Joel has something with
// nearest now (a meeting soon, mail just come), then by how they stand,
// so whoever can be reached comes first, then by name. It's measured from
// now only as they're ordered, so the minute passing moves nobody under
// the keys.
function ordered(persons, standOf, now) {
    const rank = person => (standOf(person).favourite ? 0 : 1);
    const near = person => apart(person, now);
    return persons.slice().sort((a, b) => rank(a) - rank(b)
        || near(a) - near(b)
        || standOf(a).order - standOf(b).order
        || a.name.localeCompare(b.name));
}

// Their details, as the launcher's drawer shows them beside their row:
// their name, every address, and each fact a source gave.
function details(person) {
    return {
        title: person.name,
        facts: [{
            label: person.emails.length === 1 ? "Email" : "Emails",
            value: person.emails.join(", ")
        }].concat(person.facts),
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
