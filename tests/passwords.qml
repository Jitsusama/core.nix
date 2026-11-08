// What the Passwords place works out of the entries the passwords daemon
// holds: each kind's mark, who's listed with nothing typed and in what
// order, what a row says, which acts an entry holds and in what order, its
// link in its manager's program, what its drawer says and that it never
// says a secret, and what the foot says of a copy as its time runs out,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-passwords-show-every-figure-but-the-secret` check copies
// Passwords.js, and the launcher's Ago.js it reads, beside this file and
// runs `qml -platform offscreen passwords.qml`.
import QtQuick
import "Passwords.js" as Passwords

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

        // A row's words: a manager named only while two are here.
        check("one manager isn't named", Passwords.named([{ name: "1password" }]), false);
        check("two are", Passwords.named([{ name: "1password" }, { name: "bitwarden" }]), true);
        check("its note, its username", Passwords.note(gitHub, ""), "ada");
        check("its manager leading it", Passwords.note(gitHub, "1Password"),
            "1Password \u00b7 ada");
        check("a website's host where it has no username",
            Passwords.note({ websites: [{ address: "https://bank.example/in" }] }, ""),
            "bank.example");
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
            username: "Username", websites: "Website", collection: "Vault",
            manager: "Password Manager", tags: "Tags", created: "Created",
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
            hidden: "hidden", set: "set up", none: "none", weak: "weak password",
            unsecured: "unsecured website"
        };
        const wording = manager => ({ name: name, said: said, now: now, manager: manager });
        const drawer = Passwords.details(gitHub, wording(""));
        const lines = drawer.facts.map(fact => fact.heading !== undefined ? `## ${fact.heading}`
            : `${fact.label}: ${fact.value}`);
        // The design's own first, in its order, then the rest; a healthy
        // one says no health, its Strength saying it already.
        check("every figure, the secret hidden", lines, [
            "Username: ada",
            "Websites: https://github.com, https://github.com/login",
            "Vault: Private",
            "Last Changed: 3 d ago",
            "Strength: very good",
            "One-Time Code: set up",
            "Tags: work, code",
            "Created: 982 d ago",
            "Files: 1",
            "Password: hidden",
            "## Security",
            "Recovery codes: hidden",
            "## Work",
            "Team: Shell"
        ]);
        check("its notes are its words", drawer.body, "Two keys, one in the drawer.");
        check("no secret is said", JSON.stringify(drawer).includes(secret), false);
        check("no fact warns of a healthy one", drawer.facts.some(fact => fact.warns), false);
        check("its manager named where rows name them, after its health",
            Passwords.details(gitHub, wording("1Password")).facts[6],
            { label: "Password Manager", value: "1Password" });
        // A weak password, on a website sent in the clear, warns of both.
        const weak = Object.assign({}, gitHub, {
            websites: [{ address: "http://router.example", primary: true }],
            details: [{ id: "password", label: "password", type: "hidden", hidden: true,
                purpose: "password", empty: false, strength: "weak" }]
        });
        check("its health warns", Passwords.details(weak, wording("")).facts
            .filter(fact => fact.warns === true),
        [{ label: "Health", value: "weak password \u00b7 unsecured website", warns: true }]);
        const summary = Passwords.details(listedGitHub, wording("")).facts
            .map(fact => fact.label);
        check("a summary can't say what's read in full", summary.includes("Strength")
            || summary.includes("One-Time Code"), false);
        check("one with no code says so", Passwords.details(Object.assign({}, bank,
            { details: [] }), wording("")).facts.map(fact => fact.value).slice(-1),
        ["none"]);

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

        // By vault, as Programs › 1Password holds them, each by title.
        check("by vault", Passwords.vaults([gitHub, bank, door]).map(vault =>
            [vault.name, vault.records.map(record => record.id)]),
        [["Private", ["door", "gh"]], ["Shared", ["bank"]]]);

        if (failures.length > 0) {
            console.warn(`${failures.length} of ${asked} failed:\n${failures.join("\n")}`);
            Qt.exit(1);
        } else {
            console.log(`${asked} of ${asked} held`);
            Qt.exit(0);
        }
    }
}
