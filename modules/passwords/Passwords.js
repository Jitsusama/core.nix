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
// details, own, health }, with `details` only once it was read in full and
// no hidden detail carrying its value, so nothing here could show a secret
// if it tried, and `health` only once Joel has had every password
// checked: { reused, breached }, how many other entries share its
// password and how many times Have I Been Pwned has seen it in a breach,
// breached left out where the service couldn't be asked.
.pragma library
.import "Ago.js" as Ago
.import "Numbers.js" as Numbers

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

// 1Password's ratings of a password too weak to trust, which its Strength
// draws in the warning's role; its others are good, very good, excellent
// and fantastic.
const WEAK = ["terrible", "weak", "fair"];
// A website's address sent in the clear, which its health says.
const UNSECURED = /^http:\/\//i;
// A website a browser opens, so its manager's extension can fill it.
const BROWSED = /^https?:\/\//i;

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

// The website its manager's extension fills in a browser: its primary
// one, where that's a web page, or "".
function fillsAt(record) {
    const address = website(record);
    return BROWSED.test(address) ? address : "";
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

// What its row says below its name: its website's host, since its side
// says its username (side); led by its password manager's name where rows
// name them.
function note(record, manager) {
    return [manager ?? "", host(website(record))].filter(part => part !== "").join(" \u00b7 ");
}

// What its row says at its side: its username, as every login's row says
// it, so the drawer needn't (VKCF3Z's ruling), or "".
function side(record) {
    return username(record);
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

// Its health as the last check of every password found it, or null where
// none has been run since it last changed, so nothing is guessed.
function audited(record) {
    const found = record.health;
    return found !== null && typeof found === "object" ? found : null;
}

// Whether the last check found its password shared or breached.
function troubled(record) {
    const found = audited(record);
    return found !== null && (Number(found.reused) > 0 || Number(found.breached) > 0);
}

// Those the last check found a problem with, by title, as Needing
// Attention lists them.
function needing(records) {
    const title = record => String(record.title ?? "");
    return records.filter(troubled).sort((a, b) => title(a).localeCompare(title(b)));
}

// What's wrong with it that its Strength doesn't say, in the words `said`
// gives it, or "": its password shared with other entries, "reused
// password" with one and "reused in 3 other entries" with more; seen in a
// breach, "breached password" once and "seen in 1,204 breaches" more
// often, the figure grouped as the launcher's numbers are; and a website
// sent in the clear. The first two only once Joel has had every password
// checked. A weak password is its Strength's to say, drawn in the
// warning's role, so Health never says it again (VKCF3Z's ruling); with
// nothing wrong it says nothing.
function health(record, said) {
    const found = audited(record) ?? {};
    const reused = Number(found.reused) || 0;
    const breached = Number(found.breached) || 0;
    const counted = (one, many, count) => count === 1 ? said[one] ?? ""
        : fill(said[many], { count: Numbers.format(count, 0) });
    const wrong = [];
    if (reused > 0) {
        wrong.push(counted("reused", "reusedIn", reused));
    }
    if (breached > 0) {
        wrong.push(counted("breached", "breaches", breached));
    }
    if (websites(record).some(address => UNSECURED.test(address))) {
        wrong.push(said.unsecured ?? "");
    }
    return wrong.filter(part => part !== "").join(", ");
}

// A password's strength in words, as 1Password rates it: "very good" of
// VERY_GOOD.
function strength(word) {
    return String(word ?? "").toLowerCase().replace(UNDERSCORES, " ");
}

// Whether a strength, in words, is too weak to trust.
function weak(rated) {
    return WEAK.includes(rated);
}

// Each detail read in full that isn't said elsewhere, as facts: those in
// no section first, then each section under its name as a heading, each
// by its label, a hidden one saying `hidden` whatever it holds. Said
// elsewhere: the username at the row's side, the one-time code as a fact
// of its own, and the notes as the drawer's words. Left out: the password,
// which Return copies and whose Strength says all a drawer can of it, so a
// fact saying only "hidden" told nothing (VKCF3Z's ruling); and a detail
// with nothing in it, which 1Password's templates keep and which says
// nothing.
function detailFacts(given, hidden) {
    const elsewhere = one => one.empty === true || one.purpose === "username"
        || one.purpose === "password" || one.purpose === "notes" || one.type === "code";
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

// Its drawer's eyebrow: the vault it's in and its kind, "in Keeping ·
// login", by `said`'s words, or its kind alone where it names no vault.
function eyebrow(record, said) {
    const kind = String(record.category ?? "");
    const vault = String(record.collection?.name ?? "");
    return vault === "" ? kind : fill(said.within, { vault, kind });
}

// Its drawer, as the launcher's shows it beside its row: every figure the
// manager gives but the secret. `wording` is { name, said, now, manager }:
// `name(id, state)` names each fact as the vocabulary does
// (passwords:<id>), `said` is the place's own words, and `manager` its
// manager's name where rows name them, else "". Its eyebrow says the vault
// it's in and its kind, as written, since a vault's name is the world's.
// Then what the design gives it, in its order (VKCF3Z's ruling): its
// Health, in the warning's role (`warns`), saying only what its Strength
// doesn't; its Strength, in the warning's role too where it's weak; its
// One-Time Code, saying only whether it has one; Last Changed; its
// Website; its Tags; and when it was Created. Then the password manager,
// where rows name it, how many Files it holds, and each other detail, a
// hidden one reading "hidden", never dots and never a count of them.
// Strength, the code and the other details come only once it's read in
// full, since a summary can't say them; Health reads only its websites
// and the last check of every password, which a summary has. Its notes
// are its words.
// Left out: its username, which its row's side says; its vault and its
// kind, which the eyebrow says; whether it's a favourite, which its row's
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
    fact("health", health(record, said), "", true);
    if (read) {
        const rated = strength(password(record)?.strength
            ?? record.details.find(one => (one.strength ?? "") !== "")?.strength ?? "");
        fact("strength", rated, "", weak(rated));
        fact("code", code(record) === null ? said.none : said.set);
    }
    fact("changed", Number.isFinite(record.changed) ? when(record.changed, now) : "");
    fact("websites", addresses.join(", "), addresses.length > 1 ? "several" : "");
    fact("tags", (record.tags ?? []).join(", "));
    fact("created", Number.isFinite(record.created) ? when(record.created, now) : "");
    fact("manager", manager ?? "");
    fact("files", Number.isFinite(own.files) && own.files > 0 ? String(own.files) : "");
    const notes = read ? record.details.find(one => one.purpose === "notes") : null;
    return {
        eyebrow: eyebrow(record, said),
        // A vault's name is the world's, never re-cased.
        eyebrowNames: true,
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

// Whether a copy is still for its row and the foot to say: while it's
// asked, and once refused, why. Once it's on the clipboard the launcher has
// gone and the margin's clipboard mark says it till it clears, so the row
// opened again says its username alone, rather than its title again in a
// copy's words cut short (VKCF3Z, 2026-11-08).
function speaks(copy) {
    return Boolean(copy) && (!copy.clears || (copy.why ?? "") !== "");
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
