// What the Passwords place works out of the entries the passwords daemon
// holds: each kind's mark, who's listed with nothing typed and in what
// order, what a row says, which acts an entry holds and in what order, its
// link in its manager's program, what its drawer says and that it never
// says a secret, its health once every password is checked, those
// needing attention, what the foot says of a copy as its time runs out,
// a manager's entries by vault and what its drawer says it holds, each
// entry's row kept while the daemon lists it again saying the same, and
// For You's one to-do from what the last check found: whether it stands
// or is done, what its side says and what For You weighs, run in Qt's own
// JavaScript engine, the one the shell runs it in. The `the-passwords-show-every-figure-but-the-secret`
// check copies Passwords.js, Managers.js and Checked.js, and the
// launcher's Ago.js and Numbers.js they read, beside this file and runs
// `qml -platform offscreen passwords.qml`.
import QtQuick
import "Checked.js" as Checked
import "Passwords.js" as Passwords
import "Managers.js" as Managers

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        let asked = 0;
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(
                    `${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const day = 86_400_000;
        const now = new Date(2026, 10, 7, 21, 0).getTime();
        // What a stand-in secret would read as, were one ever let through:
        // never real, and never to be seen in anything said.
        const secret = "stand-in-secret";

        // The kinds' marks, a kind no mark names wearing any entry's.
        check("a login's mark", Passwords.mark("login"), "login");
        check("a licence's mark", Passwords.mark("driver's licence"), "driversLicence");
        // Identity is the glossary's word for who Joel is on a machine, so
        // the kind's mark is named for what it is.
        check("an identity's mark", Passwords.mark("identity"), "identityCard");
        check("a kind no mark names", Passwords.mark("tax return"), "entry");
        check("no kind at all", Passwords.mark(undefined), "entry");

        // GitHub as the daemon lists it, then as it holds it read in full.
        const listedGitHub = {
            key: "entry:1password:gh", manager: "1password", id: "gh", title: "GitHub",
            category: "login", username: "ada",
            websites: [{ address: "https://github.com/login", primary: false },
                { address: "https://github.com", primary: true }],
            collection: { id: "v1", name: "Private" }, favourite: true, tags: ["work", "code"],
            created: new Date(2024, 1, 29, 12, 0).getTime(), changed: now - 3 * day,
            version: 4, own: { account: "ACCT1", summary: "ada", files: 1 }
        };
        const gitHub = Object.assign({}, listedGitHub, {
            details: [
                { id: "username", label: "username", type: "text", hidden: false,
                    purpose: "username", empty: false, value: "ada" },
                // A hidden detail that kept its value, which the daemon never
                // lets happen, still reads only "hidden".
                { id: "password", label: "password", type: "hidden", hidden: true,
                    purpose: "password", empty: false, strength: "very_good", value: secret },
                { id: "TOTP_1", label: "one-time password", type: "code", hidden: true,
                    section: "Security", empty: false },
                { id: "recovery", label: "recovery codes", type: "hidden", hidden: true,
                    section: "Security", empty: false },
                { id: "team", label: "team", type: "text", hidden: false, section: "Work",
                    empty: false, value: "Shell" },
                { id: "pin", label: "PIN", type: "text", hidden: false, empty: true },
                { id: "notesPlain", label: "notesPlain", type: "text", hidden: false,
                    purpose: "notes", empty: false, value: "Two keys, one in the drawer." }
            ]
        });
        const bank = {
            key: "entry:1password:bank", manager: "1password", id: "bank", title: "Bank Card",
            category: "card", collection: { id: "v2", name: "Shared" }, changed: now - day
        };
        const door = {
            key: "entry:1password:door", manager: "1password", id: "door", title: "",
            category: "note", collection: { id: "v1", name: "Private" }
        };
        const lately = Array.from({ length: 25 }, (_, at) => ({
            key: `entry:1password:l${at}`, manager: "1password", id: `l${at}`,
            title: `Login ${at}`, category: "login", changed: now - (at + 2) * day
        }));

        // Listed: the favourites by title, then what changed lately, newest
        // first, twenty at most; one that never said when isn't listed.
        const listed = Passwords.listed([door, bank].concat(lately, [gitHub]));
        check("favourites first, then the newest", listed.slice(0, 3).map(one => one.title),
            ["GitHub", "Bank Card", "Login 0"]);
        check("twenty changed lately at most", listed.length, 21);
        check("one that never says when it changed", listed.includes(door), false);

        // A row's words: its username at its side, its website's host
        // below, and a manager named only while two are here.
        check("one manager isn't named", Passwords.named([{ name: "1password" }]), false);
        check("two are", Passwords.named([{ name: "1password" }, { name: "bitwarden" }]), true);
        check("its side, its username", Passwords.side(listedGitHub), "ada");
        check("no username, no side", Passwords.side(bank), "");
        check("its note, its website's host", Passwords.note(gitHub, ""), "github.com");
        check("its manager leading it", Passwords.note(gitHub, "1Password"),
            "1Password \u00b7 github.com");
        check("no website, no note", Passwords.note(bank, ""), "");
        check("its primary website first", Passwords.websites(gitHub),
            ["https://github.com", "https://github.com/login"]);
        check("a row never reads blank", Passwords.titleOf(door), "Note");
        check("found by its username, websites and tags", Passwords.words(listedGitHub),
            ["ada", "https://github.com", "https://github.com/login", "github.com", "github.com",
                "work", "code"]);

        // Its acts, in the design's order, each only where it has what it
        // needs: the link only where its manager can say one.
        const pattern = "onepassword://view-item?a={account}&v={collection}&i={id}";
        const link = Passwords.shows(gitHub, pattern);
        check("its link in its manager", link, "onepassword://view-item?a=ACCT1&v=v1&i=gh");
        check("no link without its account", Passwords.shows(bank, pattern), "");
        check("no link where its manager has none", Passwords.shows(gitHub, ""), "");
        check("every act, in order", Passwords.acts(gitHub, link),
            ["password", "code", "username", "website", "show", "link"]);
        // Its website opens where its manager fills it only on a web page,
        // its primary website; anything else opens as any address does.
        check("its primary website opens where it's filled", Passwords.fillsAt(gitHub),
            "https://github.com");
        const elsewhere = address => Object.assign({}, listedGitHub,
            { websites: [{ address: address, primary: true }] });
        check("in the clear, still a page", Passwords.fillsAt(elsewhere("http://router.example")),
            "http://router.example");
        check("a server isn't a page", Passwords.fillsAt(elsewhere("ssh://git.example")), "");
        check("nor an address with no scheme", Passwords.fillsAt(elsewhere("github.com")), "");
        check("no website, nothing to open", Passwords.fillsAt(bank), "");
        // The link copied is the one Show opens, so neither goes without it.
        check("before it's read, no one-time code yet", Passwords.acts(listedGitHub, ""),
            ["password", "username", "website"]);
        check("a card has no password to copy", Passwords.acts(bank, ""), []);
        check("a login's password before it's read", Passwords.passworded(listedGitHub), true);
        check("a card read in full, by its details",
            Passwords.passworded(Object.assign({}, bank, { details: [
                { id: "ccnum", type: "hidden", hidden: true }] })), false);

        // Its drawer: every figure but the secret, in the design's order.
        const names = {
            websites: "Website", manager: "Password Manager", tags: "Tags", created: "Created",
            changed: "Last Changed", strength: "Strength", code: "One-Time Code",
            health: "Health", files: "Files"
        };
        const name = (id, state) => (id === "websites" && state === "several" ? "Websites"
            : names[id]);
        const said = {
            copying: "copying {title}'s {detail}\u2026",
            copied: "copied {title}'s {detail} \u00b7 clears in {seconds} s",
            coded: "copied {title}'s {detail} \u00b7 {left} s left of {period}",
            lapsed: "copied {title}'s {detail} \u00b7 lapsed",
            cleared: "{title}'s {detail} is cleared from the clipboard",
            unfinished: "{title}'s {detail} not copied \u00b7 {why}",
            hidden: "hidden", set: "set up", none: "none", unsecured: "unsecured website",
            reused: "reused password", reusedIn: "reused in {count} other entries",
            breached: "breached password", breaches: "seen in {count} breaches",
            within: "in {vault} \u00b7 {kind}"
        };
        const wording = manager => ({ name: name, said: said, now: now, manager: manager });
        const drawer = Passwords.details(gitHub, wording(""));
        const lines = drawer.facts.map(fact => fact.heading !== undefined ? `## ${fact.heading}`
            : `${fact.label}: ${fact.value}`);
        // The design's own, in its order, then the rest; a healthy one says
        // no health, and its username, its vault and its password say
        // nothing here: its side says the first, its eyebrow the second,
        // and the third is only copied.
        check("every figure, the secret left out", lines, [
            "Strength: very good",
            "One-Time Code: set up",
            "Last Changed: 3 d ago",
            "Websites: https://github.com, https://github.com/login",
            "Tags: work, code",
            "Created: 982 d ago",
            "Files: 1",
            "## Security",
            "Recovery codes: hidden",
            "## Work",
            "Team: Shell"
        ]);
        check("its eyebrow, its vault and its kind, as written",
            [drawer.eyebrow, drawer.eyebrowNames], ["in Private \u00b7 login", true]);
        check("no vault, its kind alone", Passwords.details(Object.assign({}, bank,
            { collection: null }), wording("")).eyebrow, "card");
        check("its notes are its words", drawer.body, "Two keys, one in the drawer.");
        check("no secret is said", JSON.stringify(drawer).includes(secret), false);
        check("no fact warns of a healthy one", drawer.facts.some(fact => fact.warns), false);
        check("its manager named where rows name them, after when it was created",
            Passwords.details(gitHub, wording("1Password")).facts[6],
            { label: "Password Manager", value: "1Password" });
        // A weak password, on a website sent in the clear: its Strength
        // warns of the first, and its Health, first, of the second alone.
        const weak = Object.assign({}, gitHub, {
            websites: [{ address: "http://router.example", primary: true }],
            details: [{ id: "password", label: "password", type: "hidden", hidden: true,
                purpose: "password", empty: false, strength: "weak" }]
        });
        check("its health and its strength warn", Passwords.details(weak, wording("")).facts
            .slice(0, 2), [{ label: "Health", value: "unsecured website", warns: true },
            { label: "Strength", value: "weak", warns: true }]);
        // Once every password is checked: shared and breached, in words,
        // the figures grouped; before, or found well, nothing more.
        const checked = health => Passwords.details(Object.assign({}, gitHub, { health }),
            wording("")).facts.find(fact => fact.label === "Health") ?? null;
        check("shared with one other", checked({ reused: 1, breached: 0 }),
            { label: "Health", value: "reused password", warns: true });
        check("shared with three others and often breached",
            checked({ reused: 3, breached: 1204 }).value,
            "reused in 3 other entries, seen in 1,204 breaches");
        check("breached once", checked({ reused: 0, breached: 1 }).value, "breached password");
        check("found well, it says nothing", checked({ reused: 0, breached: 0 }), null);
        check("breaches unasked, it says reuse alone", checked({ reused: 2 }).value,
            "reused in 2 other entries");
        check("before any check, nothing", checked(undefined), null);
        check("a breach beside a website in the clear", Passwords.details(Object.assign({},
            weak, { health: { reused: 0, breached: 1 } }), wording("")).facts[0].value,
        "breached password, unsecured website");
        check("a summary says what the check found", Passwords.details(Object.assign({},
            listedGitHub, { health: { reused: 1, breached: 0 } }), wording("")).facts[0],
        { label: "Health", value: "reused password", warns: true });
        check("no secret is said once checked", JSON.stringify(Passwords.details(
            Object.assign({}, gitHub, { health: { reused: 3, breached: 9 } }), wording("")))
            .includes(secret), false);
        // Needing Attention: those the check found a problem with, by title.
        const shared = Object.assign({}, bank, { health: { reused: 1, breached: 0 } });
        const breached = Object.assign({}, gitHub, { health: { reused: 0, breached: 4 } });
        const well = Object.assign({}, door, { health: { reused: 0, breached: 0 } });
        check("needing attention, by title", Passwords.needing([breached, well, shared,
            listedGitHub]).map(record => record.id), ["bank", "gh"]);
        check("none needing attention before a check", Passwords.needing([gitHub, bank]), []);

        const summary = Passwords.details(listedGitHub, wording("")).facts
            .map(fact => fact.label);
        check("a summary can't say what's read in full", summary.includes("Strength")
            || summary.includes("One-Time Code"), false);
        check("one with no code says so", Passwords.details(Object.assign({}, bank,
            { details: [] }), wording("")).facts.find(fact => fact.label === "One-Time Code"),
        { label: "One-Time Code", value: "none" });

        // The foot, as a copy's time runs out.
        const asking = { title: "GitHub", detail: "password", clears: 0, lapses: 0 };
        check("while it's asked", Passwords.told(asking, now, said),
            "copying GitHub's password\u2026");
        const copied = Passwords.answered(asking, { copied: true, clears: now + 90_000 }, now,
            "away");
        check("once it's copied", Passwords.told(copied, now, said),
            "copied GitHub's password \u00b7 clears in 90 s");
        check("counting down", Passwords.told(copied, now + 30_500, said),
            "copied GitHub's password \u00b7 clears in 60 s");
        check("once it's cleared", Passwords.told(copied, now + 90_000, said),
            "GitHub's password is cleared from the clipboard");
        const code = Passwords.answered(Object.assign({}, asking, { detail: "one-time code" }),
            { copied: true, clears: now + 90_000, period: 30, left: 12 }, now, "away");
        check("a code's seconds left in its period", Passwords.told(code, now + 2_000, said),
            "copied GitHub's one-time code \u00b7 10 s left of 30");
        check("a code that lapsed", Passwords.told(code, now + 12_000, said),
            "copied GitHub's one-time code \u00b7 lapsed");
        check("one refused says why", Passwords.told(Passwords.answered(asking,
            { error: { code: "signed-out", message: "1Password asks you to let the launcher in" } },
            now, "away"), now, said),
        "GitHub's password not copied \u00b7 1Password asks you to let the launcher in");
        check("with nobody to ask", Passwords.told(Passwords.answered(asking, null, now,
            "the passwords daemon isn't answering"), now, said),
        "GitHub's password not copied \u00b7 the passwords daemon isn't answering");
        check("nothing copied says nothing", Passwords.told(null, now, said), "");
        // The row and the foot say a copy only while it's asked, or why it
        // was refused; once on the clipboard, the margin's mark says it.
        check("a copy asked is the row's to say", Passwords.speaks(asking), true);
        check("one on the clipboard is the margin's", Passwords.speaks(copied), false);
        check("one refused is the row's", Passwords.speaks(Passwords.answered(asking, null, now,
            "away")), true);
        check("nothing copied isn't", Passwords.speaks(null), false);

        // For You's one to-do, from what the daemon says the last check
        // found: nothing before any check, nor from what isn't one.
        check("no figures before a check", Checked.figures({ at: null, since: null,
            cleared: null, breached: 0, reused: 0, troubled: 0 }), null);
        check("no figures from nothing", Checked.figures(null), null);
        const hour = 3_600_000;
        const found = Checked.figures({ at: now - hour, since: now - 3 * hour, cleared: null,
            breached: 3, reused: 1204, troubled: 1205 });
        check("the figures as the daemon said them", found, { at: now - hour,
            since: now - 3 * hour, cleared: null, breached: 3, reused: 1204, troubled: 1205 });
        check("figures read the same are the same", Checked.sameFigures(found,
            Checked.figures(Object.assign({}, found))), true);
        check("figures that differ by a count differ", Checked.sameFigures(found,
            Checked.figures(Object.assign({}, found, { reused: 1203 }))), false);
        check("a check finding trouble stands", Checked.stands(found), true);
        check("one standing isn't done", Checked.cleared(found), false);
        const todo = { breached: "{count} breached", reused: "{count} reused" };
        check("its side, breached first, the figures grouped", Checked.todoSide(found, todo),
            "3 breached \u00b7 1,204 reused");
        check("none breached is left out", Checked.todoSide(Object.assign({}, found,
            { breached: 0 }), todo), "1,204 reused");
        check("none reused is left out", Checked.todoSide(Object.assign({}, found,
            { reused: 0 }), todo), "3 breached");
        check("its parts give way reused first", Checked.todoSideItems(found, todo),
            [{ said: "3 breached", keep: 2 }, { said: "1,204 reused", keep: 1 }]);
        check("a count of none says nothing", Checked.counted(0), "");
        // Why it's there, both counts whole, the logins named on the first.
        const sentence = "The check at {at} found {found}.";
        const why = Object.assign({}, todo, { and: " and ", breachedLogin: "{count} breached login",
            breachedLogins: "{count} breached logins", reusedLogin: "{count} reused login",
            reusedLogins: "{count} reused logins" });
        check("why, both counts", Checked.todoWhy(found, why, sentence, "05:10"),
            "The check at 05:10 found 3 breached logins and 1,204 reused.");
        check("why, one breached", Checked.todoWhy(Object.assign({}, found,
            { breached: 1, reused: 0 }), why, sentence, "05:10"),
            "The check at 05:10 found 1 breached login.");
        check("why, reused alone named", Checked.todoWhy(Object.assign({}, found,
            { breached: 0 }), why, sentence, "05:10"),
            "The check at 05:10 found 1,204 reused logins.");
        check("why, nothing found", Checked.todoWhy(Object.assign({}, found,
            { breached: 0, reused: 0 }), why, sentence, "05:10"), "");
        check("what For You weighs", Checked.todoThing(found), { kind: "todo",
            since: now - 3 * hour, takes: 1205 * 3, ready: true });
        // Each changed since the check took leaves the counts, and with
        // none left it no longer stands, though no check has cleared it.
        const changed = Object.assign({}, found, { breached: 0, reused: 0, troubled: 0 });
        check("with every one changed, it doesn't stand", Checked.stands(changed), false);
        check("nor is it done before a check says so", Checked.cleared(changed), false);
        // A later check finding none clears it, into History.
        const clear = Checked.figures({ at: now, since: null, cleared: now, breached: 0,
            reused: 0, troubled: 0 });
        check("a check finding none doesn't stand", Checked.stands(clear), false);
        check("a check finding none is done", Checked.cleared(clear), true);
        // One that found none before any trouble is neither.
        const never = Checked.figures({ at: now, since: null, cleared: null, breached: 0,
            reused: 0, troubled: 0 });
        check("never troubled, it neither stands nor is done", [Checked.stands(never),
            Checked.cleared(never)], [false, false]);

        // By vault, as Programs › 1Password holds them, each by title.
        check("by vault", Managers.vaults([gitHub, bank, door]).map(vault =>
            [vault.name, vault.records.map(record => record.id)]),
        [["Private", ["door", "gh"]], ["Shared", ["bank"]]]);
        // What a vault holds says what it counts, its thousands grouped.
        const counts = { entry: "{count} entry", entries: "{count} entries" };
        check("a vault of one", Managers.holding(1, counts), "1 entry");
        check("a vault of many", Managers.holding(1201, counts), "1,201 entries");
        check("an empty vault", Managers.holding(0, counts), "0 entries");
        // Its note in Programs sits under its program, so it leaves its name out.
        check("a note leaves the manager's name out", Managers.noted(
            "1Password asks you to let the launcher in", "1Password"),
        "asks you to let the launcher in");
        check("a note naming no manager is said whole", Managers.noted(
            "it stopped answering", "1Password"), "it stopped answering");
        check("a note with no reason says nothing", Managers.noted(undefined, "1Password"), "");

        // An entry's row is made once, and kept as the daemon lists it
        // again as a new object saying the same, as each step in has it.
        let made = 0;
        const make = record => {
            made += 1;
            return { key: record.key, name: record.title };
        };
        const first = Managers.rowsKept([gitHub, bank], {}, make);
        check("each entry listed first is made", [made, first.rows.map(row => row.name)],
            [2, [gitHub.title, bank.title]]);
        made = 0;
        const again = Managers.rowsKept([Object.assign({}, gitHub), Object.assign({}, bank)],
            first.kept, make);
        check("listed again saying the same, none is made again", made, 0);
        check("and each keeps its row", again.rows.map((row, at) => row === first.rows[at]),
            [true, true]);
        const renamed = Object.assign({}, bank, { title: "Harbour Credit Union" });
        const retitled = Managers.rowsKept([gitHub, renamed], again.kept, make);
        check("one saying something new is made again, alone",
            [made, retitled.rows[0] === first.rows[0], retitled.rows[1].name],
            [1, true, "Harbour Credit Union"]);
        check("and one no longer listed isn't kept",
            Object.keys(Managers.rowsKept([gitHub], retitled.kept, make).kept), [gitHub.key]);

        // What 1Password's drawer in Programs says it holds.
        const holds = { inVault: "{count} in 1 vault", inVaults: "{count} in {vaults} vaults" };
        const many = Array.from({ length: 1200 }, (_, at) => Object.assign({}, door,
            { id: `d${at}`, favourite: at < 2 }));
        check("what a manager holds", Managers.held(many.concat([bank]), holds),
            [{ id: "passwords:entries", value: "1,201 in 2 vaults" },
                { id: "passwords:favourites", value: "2" }]);
        check("one vault, no favourites", Managers.held([Object.assign({}, bank,
            { favourite: false })], holds),
        [{ id: "passwords:entries", value: "1 in 1 vault" }]);
        check("nothing held, nothing said", Managers.held([], holds), []);

        // Each manager that can't read leads the place with a row of its own:
        // Unlock while it waits on Joel or its app, and its program while it's
        // refused, never nothing (2026-11-10's empty place).
        const kinds = ["one", "two", "three"].map(name => ({ name: name }));
        check("a manager that can't read is never left without a row",
            Managers.unread(kinds, {
                one: { state: "refused", reason: "needs a setting" },
                two: { state: "answering" },
                three: { state: "signed-out", reason: "is closed" }
            }).map(unread => [unread.manager.name, unread.row]),
            [["one", "settle"], ["three", "unlock"]]);
        check("one the daemon says nothing of is left alone",
            Managers.unread(kinds, {}), []);

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
