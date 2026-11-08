// What the Passwords place works out of the entries the passwords daemon
// holds, apart from drawing them (Passwords.qml): the order they're listed
// in, each row's words and mark, which acts an entry has inside it and in
// what order, what its drawer says, and what the foot says of a copy as
// its time runs out. Kept apart so the-passwords-show-every-figure-but-the-
// secret runs it in Qt's own engine (tests/passwords.qml).
//
// An entry is a record as the daemon gives it: { key, manager, id, title,
// category, username, websites: [{ address, label, primary }],
// collection: { id, name }, favourite, tags, created, changed, version,
// details, own }, with `details` only once it was read in full and no
// hidden detail carrying its value, so nothing here could show a secret
// if it tried.
.pragma library
.import "Ago.js" as Ago

// Each kind's mark, by the kind's word as the managers say it: Passwords'
// own word where every password manager has the kind, else 1Password's
// category in Canadian spelling (modules/onepassword/manager's category).
// A kind not named here wears the mark any other entry does.
const MARKS = {
    "login": "login",
    "password": "storedPassword",
    "card": "paymentCard",
    "note": "secureNote",
    "identity": "identityCard",
    "key": "sshKey",
    "credential": "apiCredential",
    "database": "database",
    "server": "server",
    "wireless router": "router",
    "bank account": "bankAccount",
    "email account": "emailAccount",
    "driver's licence": "driversLicence",
    "outdoor licence": "outdoorLicence",
    "software licence": "softwareLicence",
    "membership": "membership",
    "passport": "passport",
    "reward program": "rewardProgram",
    "social security number": "socialSecurityNumber",
    "medical record": "medicalRecord",
    "crypto wallet": "cryptoWallet",
    "document": "keptFile"
};
const ANY_KIND = "entry";

function mark(category) {
    return MARKS[String(category ?? "")] ?? ANY_KIND;
}

// The kinds whose Return copies a password before they're read in full,
// since every one of them has one; any other copies one only once its
// details show a detail whose purpose is a password.
const PASSWORDED = ["login", "password"];

// How many entries changed lately are listed after the favourites with
// nothing typed: the rest are found by typing, as People's are, so the
// place opens on what Joel reaches for rather than on hundreds.
const RECENT = 20;

// A second, which the foot counts in.
const SECOND = 1000;

// 1Password's ratings of a password too weak to trust, which its health
// says in the warning's role; its others are good, very good, excellent
// and fantastic.
const WEAK = ["terrible", "weak", "fair"];
// A website's address sent in the clear, which its health says too.
const UNSECURED = /^http:\/\//i;

// How many managers it takes before rows name theirs.
const SEVERAL = 2;

// Where an address's scheme ends and where its host does.
const SCHEME = "://";
const HOST_ENDS = /[/?#]/;
// A part of a pattern or of the vocabulary's words to fill, as {id}.
const BRACED = /\{(\w+)\}/g;
// Each underscore in 1Password's strength's word.
const UNDERSCORES = /_/g;

// Whether rows name their password manager: only while two or more are on
// the machine, as Agents names a session's machine only where it isn't
// this one, since with one there's nothing to tell apart.
function named(managers) {
    return (managers ?? []).length >= SEVERAL;
}

// An entry's website: its primary one, else its first, or "".
function website(record) {
    const given = (record.websites ?? []).filter(one => (one.address ?? "") !== "");
    const primary = given.find(one => one.primary === true) ?? given[0];
    return primary ? String(primary.address) : "";
}

// Every website it has, the primary first.
function websites(record) {
    const first = website(record);
    const rest = (record.websites ?? []).map(one => String(one.address ?? ""))
        .filter(address => address !== "" && address !== first);
    return first === "" ? rest : [first].concat(rest);
}

// An address's host, as a row says where a login is for: "github.com" of
// "https://github.com/login".
function host(address) {
    const text = String(address ?? "");
    const rest = text.includes(SCHEME) ? text.slice(text.indexOf(SCHEME) + SCHEME.length)
        : text;
    return rest.split(HOST_ENDS)[0];
}

// Its username: the one it was listed with, else the detail whose
// purpose is one, or "".
function username(record) {
    const given = String(record.username ?? "");
    if (given !== "") {
        return given;
    }
    const detail = (record.details ?? []).find(one => one.purpose === "username");
    return detail && detail.hidden !== true ? String(detail.value ?? "") : "";
}

// A word as a name, its first letter a capital, as a detail's label is
// shown, and as a fact the launcher doesn't name yet is (Provider.qml's
// partNamed).
function labelled(label) {
    const text = String(label ?? "");
    return text.charAt(0).toUpperCase() + text.slice(1);
}

// What it's called on its row: its title, else where it's for, else whose
// it is, else its kind as a name ("Note"), so no row reads blank.
function titleOf(record) {
    const title = String(record.title ?? "").trim();
    if (title !== "") {
        return title;
    }
    return host(website(record)) || username(record) || labelled(record.category ?? record.id);
}

// What its row says below its name: its username, else its website's
// host; led by its password manager's name where rows name them.
function note(record, manager) {
    const own = username(record) || host(website(record));
    return [manager ?? "", own].filter(part => part !== "").join(" \u00b7 ");
}

// The words besides its title it's found by: its username, every
// website's address and host, and its tags.
function words(record) {
    const addresses = websites(record);
    return [username(record)].concat(addresses, addresses.map(host), record.tags ?? [])
        .filter(word => word !== "");
}

// Those listed with nothing typed: the favourites by title, then the rest
// by when each last changed, newest first, at most RECENT of them; one
// that never says when it changed is found by typing alone.
function listed(records) {
    const title = record => String(record.title ?? "");
    const favourites = records.filter(record => record.favourite === true)
        .sort((a, b) => title(a).localeCompare(title(b)));
    const lately = records.filter(record => record.favourite !== true
        && Number.isFinite(record.changed))
        .sort((a, b) => b.changed - a.changed || title(a).localeCompare(title(b)))
        .slice(0, RECENT);
    return favourites.concat(lately);
}

// Its password's detail once it's read in full: the one whose purpose is
// a password, or null.
function password(record) {
    return (record.details ?? []).find(one => one.purpose === "password") ?? null;
}

// Its one-time code's detail, or null; known only once it's read in full.
function code(record) {
    return (record.details ?? []).find(one => one.type === "code") ?? null;
}

// Whether it has a password, so Return copies it: by its kind until it's
// read in full, then by its details.
function passworded(record) {
    if (Array.isArray(record.details)) {
        return password(record) !== null;
    }
    return PASSWORDED.includes(String(record.category ?? ""));
}

// Its link in its password manager's own program, from the manager's
// pattern (jitsusama.passwords.managers.<name>.shows), as 1Password's
// onepassword://view-item?a={account}&v={collection}&i={id}: or "" where
// the manager has none, or the entry doesn't say a part of it, as an
// entry listed before its manager could say its account doesn't.
function shows(record, pattern) {
    if ((pattern ?? "") === "") {
        return "";
    }
    const parts = {
        account: record.own?.account,
        collection: record.collection?.id,
        id: record.id
    };
    let missing = false;
    const link = String(pattern).replace(BRACED, (_whole, part) => {
        const value = parts[part];
        missing = missing || value === undefined || value === null || value === "";
        return missing ? "" : encodeURIComponent(String(value));
    });
    return missing ? "" : link;
}

// The acts inside it, by name, in the one order the design gives them
// (VKCF3Z's ruling on the Passwords place): Copy the Password, Copy the
// One-Time Code, Copy the Username, Open the Website, Show in its
// manager, then Copy the Link, each only where it has what the act needs.
// Copying the password leads, though Copy is last everywhere else, since
// it's what an entry is for rather than a copy of what the row says
// (docs/design.md, One word each). The link is the entry's own in its
// manager, the one Show opens, so it goes only where Show does; the
// website's address is Open the Website's.
function acts(record, link) {
    const has = {
        password: passworded(record),
        code: code(record) !== null,
        username: username(record) !== "",
        website: website(record) !== "",
        show: (link ?? "") !== "",
        link: (link ?? "") !== ""
    };
    return ["password", "code", "username", "website", "show", "link"].filter(act => has[act]);
}

// When something was, as the launcher says it wherever it does: how long
// ago (Ago.js).
function when(at, now) {
    return Ago.ago(at, now);
}

// What's wrong with it, in the words `said` gives each, or "": a password
// 1Password rates too weak to trust, and a website sent in the clear.
// Known only once it's read in full, since a summary carries no strength;
// with nothing wrong it says nothing, since its Strength already does.
function health(record, said) {
    const rated = strength(password(record)?.strength ?? "");
    return [
        WEAK.includes(rated) ? said.weak : "",
        websites(record).some(address => UNSECURED.test(address)) ? said.unsecured : ""
    ].filter(word => (word ?? "") !== "").join(" \u00b7 ");
}

// A password's strength in words, as 1Password rates it: "very good" of
// VERY_GOOD.
function strength(word) {
    return String(word ?? "").toLowerCase().replace(UNDERSCORES, " ");
}

// Each detail read in full that isn't said elsewhere, as facts: those in
// no section first, then each section under its name as a heading, each
// by its label, a hidden one saying `hidden` whatever it holds. Said
// elsewhere: the username and the one-time code as facts of their own,
// and the notes as the drawer's words. Left out: a detail with nothing in
// it, which 1Password's templates keep and which says nothing.
function detailFacts(given, hidden) {
    const elsewhere = one => one.empty === true || one.purpose === "username"
        || one.purpose === "notes" || one.type === "code";
    const shown = given.filter(one => !elsewhere(one));
    const sections = [];
    for (const one of shown) {
        const section = String(one.section ?? "");
        if (!sections.includes(section)) {
            sections.push(section);
        }
    }
    sections.sort((a, b) => Number(a !== "") - Number(b !== ""));
    const out = [];
    for (const section of sections) {
        if (section !== "") {
            out.push({ heading: section });
        }
        for (const one of shown.filter(each => String(each.section ?? "") === section)) {
            out.push({
                label: labelled(one.label ?? one.id),
                value: one.hidden === true ? hidden : String(one.value ?? "")
            });
        }
    }
    return out;
}

// Its drawer, as the launcher's shows it beside its row: every figure the
// manager gives but the secret. `wording` is { name, said, now, manager }:
// `name(id, state)` names each fact as the vocabulary does
// (passwords:<id>), `said` is the place's own words, and `manager` its
// manager's name where rows name them, else "". First what the design
// gives it, in its order (VKCF3Z's ruling): Username, Website, Vault,
// Last Changed, Strength, One-Time Code, saying only whether it has one,
// and its health in the warning's role (`warns`). Then the password
// manager, where rows name it, its Tags, when it was Created, how many
// Files it holds, and each other detail, a hidden one reading "hidden",
// never dots and never a count of them. Strength, the code, its health and
// the other details come only once it's read in full, since a summary
// can't say them. Its notes are its words. Left out: its kind, which its
// mark and the eyebrow say; whether it's a favourite, which its row's
// place says; the account's id, which only makes its link; who last
// changed it, which op gives only as a person's id; its version, a count
// Joel never acts on; and 1Password's summary of it, which says the
// username again, or a card's number in part.
function details(record, wording) {
    const { name, said, now, manager } = wording;
    const facts = [];
    const fact = (id, value, state, warns) => {
        if (value !== "" && value !== null && value !== undefined) {
            const one = { label: name(id, state ?? ""), value: String(value) };
            facts.push(warns === true ? Object.assign(one, { warns: true }) : one);
        }
    };
    const addresses = websites(record);
    const own = record.own ?? {};
    const read = Array.isArray(record.details);
    fact("username", username(record));
    fact("websites", addresses.join(", "), addresses.length > 1 ? "several" : "");
    fact("collection", record.collection?.name ?? "");
    fact("changed", Number.isFinite(record.changed) ? when(record.changed, now) : "");
    if (read) {
        const rated = password(record)?.strength
            ?? record.details.find(one => (one.strength ?? "") !== "")?.strength ?? "";
        fact("strength", strength(rated));
        fact("code", code(record) === null ? said.none : said.set);
        fact("health", health(record, said), "", true);
    }
    fact("manager", manager ?? "");
    fact("tags", (record.tags ?? []).join(", "));
    fact("created", Number.isFinite(record.created) ? when(record.created, now) : "");
    fact("files", Number.isFinite(own.files) && own.files > 0 ? String(own.files) : "");
    const notes = read ? record.details.find(one => one.purpose === "notes") : null;
    return {
        title: String(record.title ?? ""),
        // The other details last, since their sections' headings would
        // otherwise seem to hold the facts after them.
        facts: facts.concat(read ? detailFacts(record.details, said.hidden) : []),
        body: notes && notes.hidden !== true ? String(notes.value ?? "") : ""
    };
}

// The vocabulary's words, each {part} filled.
function fill(saying, values) {
    return String(saying ?? "").replace(BRACED, (whole, part) => String(values[part] ?? whole));
}

// What the foot says of a copy, by `said`'s words, at `now`: while it's
// asked, "copying GitHub's password…"; once on the clipboard, "copied
// GitHub's password · clears in 90 s", counting down to the time the
// daemon said it clears; a one-time code instead says the seconds left in
// its period, from the daemon's answer, and once its period ends that it
// lapsed; once cleared, that it's gone; and one refused, why. A copy is
// { title, detail, clears, lapses, period, why }: `clears` and `lapses`
// are milliseconds since the epoch, 0 until the daemon answers.
function told(copy, now, said) {
    if (!copy) {
        return "";
    }
    const values = { title: copy.title, detail: copy.detail, why: copy.why ?? "" };
    if ((copy.why ?? "") !== "") {
        return fill(said.unfinished, values);
    }
    if (!copy.clears) {
        return fill(said.copying, values);
    }
    if (now >= copy.clears) {
        return fill(said.cleared, values);
    }
    if (copy.lapses) {
        const left = Math.ceil((copy.lapses - now) / SECOND);
        return left > 0 ? fill(said.coded, Object.assign(values, { left, period: copy.period }))
            : fill(said.lapsed, values);
    }
    const seconds = Math.ceil((copy.clears - now) / SECOND);
    return fill(said.copied, Object.assign(values, { seconds }));
}

// A copy as the daemon answered it, heard at `now`: when it clears, and
// for a one-time code when it lapses and its period; or why it wasn't,
// `away` when the daemon wasn't there to ask.
function answered(copy, answer, now, away) {
    if (answer === null) {
        return Object.assign({}, copy, { why: away });
    }
    if (answer.error) {
        return Object.assign({}, copy, { why: String(answer.error.message ?? answer.error.code) });
    }
    const left = Number(answer.left);
    return Object.assign({}, copy, {
        clears: Number(answer.clears) || now,
        lapses: Number.isFinite(left) && left > 0 ? now + left * SECOND : 0,
        period: Number(answer.period) || 0
    });
}

// Its entries grouped by vault, as Programs holds a manager's: each
// { id, name, records } by the vault's name, its records by title.
function vaults(records) {
    const by = {};
    const order = [];
    for (const record of records) {
        const id = String(record.collection?.id ?? "");
        if (!(id in by)) {
            by[id] = { id, name: String(record.collection?.name ?? ""), records: [] };
            order.push(id);
        }
        by[id].records.push(record);
    }
    const title = record => String(record.title ?? "");
    return order.map(id => by[id])
        .sort((a, b) => a.name.localeCompare(b.name))
        .map(vault => Object.assign(vault, {
            records: vault.records.sort((a, b) => title(a).localeCompare(title(b)))
        }));
}
