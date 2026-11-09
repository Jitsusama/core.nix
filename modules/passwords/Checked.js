// What the last check of every password found, as For You's one to-do
// for it says it (Passwords.qml's forYou), apart from drawing it: whether
// it stands or is done, what its side says and what For You weighs. One
// to-do however many logins the check found, so a check never floods For
// You (VKCF3Z [6d59cde5], WCJQSG [faf5592e]). Kept apart so
// the-passwords-show-every-figure-but-the-secret runs it in Qt's own
// engine (tests/passwords.qml).
.pragma library
.import "Numbers.js" as Numbers
.import "Passwords.js" as Passwords

// How long changing one login's password takes, in minutes, as For You's
// to-do says it takes: signing in to the website, changing it there and
// saving it in its manager.
const MINUTES_EACH = 3;

// How long each part of the to-do's side holds as its row gives way, the
// higher the longer: how many are breached outlasts how many are reused,
// since a breached password is the more pressing.
const KEEP_BREACHED = 2;
const KEEP_REUSED = 1;

// What the last check of every password found, as the daemon keeps it
// through a reboot and answers it (`checked`), or null before any check:
// { at, since, cleared, breached, reused, troubled }, each time in
// milliseconds or null, and each count how many entries are breached,
// reused, or either, by what's held of them now.
function figures(checked) {
    if (checked === null || typeof checked !== "object") {
        return null;
    }
    const time = value => Number.isFinite(value) ? value : null;
    const count = value => Number.isFinite(value) && value > 0 ? value : 0;
    const at = time(checked.at);
    if (at === null) {
        return null;
    }
    return {
        at,
        since: time(checked.since),
        cleared: time(checked.cleared),
        breached: count(checked.breached),
        reused: count(checked.reused),
        troubled: count(checked.troubled)
    };
}

// Whether two readings of the figures say the same, so For You's to-do
// is made again only when one differs.
function sameFigures(one, other) {
    return JSON.stringify(one) === JSON.stringify(other);
}

// Whether the to-do stands: a check found a password breached or reused,
// no later check has found none, and some entry is still held so, since
// one changed since leaves the counts, to be checked again by the next.
function stands(found) {
    return found !== null && found.since !== null && found.cleared === null
        && found.breached + found.reused > 0;
}

// Whether a later check found none breached and none reused, so the
// to-do is in History, done.
function cleared(found) {
    return found !== null && found.cleared !== null && !stands(found);
}

// A count as the to-do says it, grouped as the launcher's figures are, or
// "" at none, so a count of none is left out.
function counted(entries) {
    return entries > 0 ? Numbers.format(entries, 0) : "";
}

// What the to-do says at its side, by `said`'s words, as the parts its row
// gives way by whole (Reading.js's keptItems): how many are breached, then
// how many reused, each left out at none, breached kept longest.
function todoSideItems(found, said) {
    const items = [];
    if (found.breached > 0) {
        items.push({ said: Passwords.fill(said.breached, { count: counted(found.breached) }),
            keep: KEEP_BREACHED });
    }
    if (found.reused > 0) {
        items.push({ said: Passwords.fill(said.reused, { count: counted(found.reused) }),
            keep: KEEP_REUSED });
    }
    return items;
}

// Its side whole, "3 breached · 12 reused".
function todoSide(found, said) {
    return todoSideItems(found, said).map(item => item.said).join(" \u00b7 ");
}

// Why the to-do is there, its drawer's one sentence, "The check at 05:10
// found 1 breached login and 2 reused", which holds both counts whole when
// the side gives way (VKCF3Z [4e0fcf16]): the logins named once, on the
// first count, each left out at none. `sentence` is what the drawer tells,
// `said` the counts' words, and `at` the check's time as the clock says
// it; "" while nothing was found.
function todoWhy(found, said, sentence, at) {
    const parts = [];
    if (found.breached > 0) {
        parts.push({ count: found.breached, one: said.breachedLogin,
            many: said.breachedLogins, bare: said.breached });
    }
    if (found.reused > 0) {
        parts.push({ count: found.reused, one: said.reusedLogin,
            many: said.reusedLogins, bare: said.reused });
    }
    if (parts.length === 0) {
        return "";
    }
    const [first, ...rest] = parts;
    const told = [Passwords.fill(first.count === 1 ? first.one : first.many,
        { count: counted(first.count) })]
        .concat(rest.map(part => Passwords.fill(part.bare, { count: counted(part.count) })));
    return Passwords.fill(sentence, { at, found: told.join(said.and) });
}

// The to-do as For You weighs it (ForYou.js's weigh): a thing to do, ready
// now, waiting since a check first found trouble, and taking a few
// minutes a login.
function todoThing(found) {
    const logins = Math.max(found.troubled, found.breached, found.reused);
    return { kind: "todo", since: found.since, takes: logins * MINUTES_EACH, ready: true };
}
