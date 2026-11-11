// What the shell reads of the google program's records, each change
// remaking only the parts of the kinds it names, when mail came in
// words, and the lines marking it read or archiving it, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-desktop-reads-the-mail` check copies Google.js, Mail.js and the
// files they import beside this file and runs `qml -platform offscreen
// mail.qml`.
// Times are made on the local clock, so the cases hold in whatever zone the
// check runs in.
import QtQuick
import "Google.js" as Google
import "Mail.js" as Mail

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
        // Monday 26 October 2026, 15:20 here.
        const at = (month, day, hour, min) => new Date(2026, month, day, hour, min).getTime();
        const now = at(9, 26, 15, 20);
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(`${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        // How mail says when, as Mail.qml gives it: the vocabulary's words,
        // and the locale's names, the check's being English.
        const said = {
            now: "just now",
            minutes: "{minutes} min ago",
            yesterday: "yesterday",
            tomorrow: "tomorrow",
            to: "to {who}",
            toTwo: "{one} and {other}",
            toMore: "{who} and {count} more",
            toPeople: "{count} people",
            date: (date, format) => Qt.formatDate(date, format)
        };
        const standup = {
            id: "standup", title: "Standup", start: now, end: now, allDay: false,
            answer: "yes", link: null, kind: null, page: ""
        };
        const mail = {
            id: "m_1", from: "Ada", subject: "Lunch", at: now,
            page: "https://mail.google.com/mail/u/0/#inbox/t_1"
        };

        // What the daemon holds, by key, as src/daemon.rs makes each record.
        const unmade = { granted: false, made: null };
        const placeless = { files: [], more: false };
        const undriven = {
            listings: [], downloads: [], uploads: [],
            places: { recent: [], starred: placeless, shared: placeless, drives: placeless },
            kept: { held: 0, most: 0 }
        };
        const record = (kind, id, thing, heard) => ({
            [`${kind}:${id}`]: { key: `${kind}:${id}`, kind, heard, [kind]: thing }
        });
        const account = (standing, more, heard) => record("account", "google",
            Object.assign({ standing, unread: null, fetched: null, meet: unmade,
                drive: undriven }, more), heard);
        const none = {
            account: "none", meetings: [], mail: [], finished: [], unread: null, documents: [],
            held: [],
            lately: [], calls: [], heard: null, fetched: null, meet: unmade, drive: undriven
        };
        const later = Object.assign({}, standup, { id: "later", start: now + 60_000 });
        const older = Object.assign({}, mail, { id: "m_0", at: now - 60_000 });
        const sheet = { id: "s1", kind: "spreadsheet", viewed: now };
        const doc = { id: "d1", kind: "document", viewed: now - 60_000 };
        const newer = { id: "d2", kind: "document", viewed: now };
        const followed = { code: "abc-defg-hij", granted: true, people: [] };
        // Mail done elsewhere, kept a while apart from what still wants Joel,
        // the latest done first.
        const read = Object.assign({}, mail, { id: "m_2",
            done: { how: "readInGmail", at: now - 1000 } });
        const archived = Object.assign({}, mail, { id: "m_3",
            done: { how: "archivedInGmail", at: now } });
        // [what, what's held, whether the daemon answers, what's read of it]
        const held = [
            ["the whole account",
                Object.assign({}, record("meeting", "standup", standup, 1),
                    record("mail", "m_1", mail, 1),
                    account("ready", { unread: 3 }, 1793042446024)),
                true,
                {
                    account: "ready", meetings: [standup], mail: [mail], finished: [], unread: 3,
                    documents: [], held: [], lately: [], calls: [], heard: 1793042446024,
                    fetched: null, meet: unmade, drive: undriven
                }],
            ["each kind in its order, whatever order it's held in",
                Object.assign({}, record("meeting", "later", later, 1),
                    record("meeting", "standup", standup, 1), record("mail", "m_0", older, 1),
                    record("mail", "m_1", mail, 1), record("mail", "m_2", read, 1),
                    record("mail", "m_3", archived, 1), record("file", "s1", sheet, 1),
                    record("file", "d1", doc, 1), record("file", "d2", newer, 1),
                    record("held", "later", later, 1), record("held", "standup", standup, 1),
                    record("lately", "m_0", older, 1), record("lately", "m_1", mail, 1),
                    record("call", "abc-defg-hij", followed, 1), account("ready", {}, 2)),
                true,
                {
                    account: "ready", meetings: [standup, later], mail: [mail, older],
                    finished: [archived, read], unread: null, documents: [newer, doc, sheet],
                    held: [standup, later],
                    lately: [mail, older], calls: [followed], heard: 2, fetched: null,
                    meet: unmade,
                    drive: undriven
                }],
            ["a signed-out account, alone", account("signedOut", {}, 3), true,
                Object.assign({}, none, { account: "signedOut", heard: 3 })],
            ["nothing held yet", {}, true, none],
            ["no account once the daemon stops answering",
                Object.assign({}, record("meeting", "standup", standup, 1),
                    account("ready", {}, 1)),
                false, none]
        ];
        for (const [what, records, answering, meant] of held)
            check(what, Google.gathered(records, answering), meant);
        check("making a meeting granted", Google.gathered(account("ready",
            { meet: { granted: true, made: null } }, 1), true).meet,
            { granted: true, made: null });

        // A change remakes only the parts whose keys it names (Daemon.qml's
        // heard), each kind known by how its key begins, so a key gone is
        // too: the account told again a minute on remakes the account's
        // own parts alone, never a list, and a mail heard the mail and
        // what's finished alone.
        const holds = Object.assign({}, record("meeting", "standup", standup, 1),
            record("mail", "m_1", mail, 1), account("ready", { unread: 3 }, 1));
        const byKey = Google.keeping();
        const remade = kinds => Object.keys(Google.partsOf(byKey, kinds, true)).sort();
        check("every kind at first",
            Object.keys(Google.keep(byKey, holds, Object.keys(holds))).sort(),
            ["account", "mail", "meeting"]);
        check("kept as gathered", Google.stateOf(Google.partsOf(byKey, Google.everyKind(), true)),
            Google.gathered(holds, true));
        Object.assign(holds, account("ready", { unread: 3 }, 2));
        const aMinuteOn = Google.keep(byKey, holds, ["account:google"]);
        check("the account told again moves the account alone", Object.keys(aMinuteOn),
            ["account"]);
        check("and remakes its own parts alone", remade(aMinuteOn),
            ["account", "drive", "fetched", "heard", "meet", "unread"]);
        Object.assign(holds, record("mail", "m_0", older, 1));
        check("a mail heard remakes the mail and what's finished alone",
            remade(Google.keep(byKey, holds, ["mail:m_0"])), ["finished", "mail"]);
        delete holds["meeting:standup"];
        const gone = Google.keep(byKey, holds, ["meeting:standup"]);
        check("a meeting gone remakes the meetings alone", Google.partsOf(byKey, gone, true),
            { meetings: [] });
        check("a key of no kind it keeps moves nothing",
            Google.keep(byKey, holds, ["weather:here", "account:other"]), {});
        check("nothing once the daemon stops answering", Google.partsOf(byKey, gone, false),
            none);
        check("an account to say", Google.ready(byKey, true), true);
        check("none while not answering", Google.ready(byKey, false), false);
        check("none before the account is said", Google.ready(Google.keeping(), true), false);

        // A new Meet meeting, asked for at 100: how it stands as the
        // program says.
        const meet = made => ({ granted: true, made: made });
        const link = "https://meet.google.com/new-abcd-efg";
        check("the line making a meeting", Google.make(), '{"make":{}}\n');
        check("how a meeting asked for stands", [
            [meet(null), null],
            [meet(null), 100],
            [meet({ asked: 90, link: link, failed: false }), 100],
            [meet({ asked: 100, link: null, failed: false }), 100],
            [meet({ asked: 101, link: link, failed: false }), 100],
            [meet({ asked: 101, link: null, failed: true }), 100]
        ].map(([standing, since]) => Google.making(standing, since)), [
            { state: "", link: "" },
            { state: "making", link: "" },
            { state: "making", link: "" },
            { state: "making", link: "" },
            { state: "made", link: link },
            { state: "failed", link: "" }
        ]);

        // When it came: [what, when, said]
        const cames = [
            ["a moment ago", now - 30 * 1000, "just now"],
            ["as it is now", now, "just now"],
            ["twelve minutes ago", at(9, 26, 15, 8), "12 min ago"],
            ["59 minutes ago", at(9, 26, 14, 21), "59 min ago"],
            ["an hour ago", at(9, 26, 14, 20), "14:20"],
            ["this afternoon", at(9, 26, 14, 2), "14:02"],
            ["just after midnight", at(9, 26, 0, 5), "00:05"],
            ["late last night", at(9, 25, 23, 50), "yesterday"],
            ["on Thursday", at(9, 22, 9, 0), "Thursday"],
            ["on Tuesday last week", at(9, 20, 9, 0), "Tuesday"],
            ["a week ago today", at(9, 19, 9, 0), "19 Oct"],
            ["last month", at(8, 30, 9, 0), "30 Sep"],
            ["last year", new Date(2025, 9, 28, 9, 0).getTime(), "28 Oct 2025"],
            ["later today, by a wrong clock", at(9, 26, 17, 0), "17:00"],
            // A day ahead reads as a day behind does (VKCF3Z, 2026-11-08).
            ["tomorrow, by a wrong clock", at(9, 27, 9, 0), "tomorrow"]
        ];
        for (const [what, time, meant] of cames)
            check(`came ${what}`, Mail.came(time, now, said), meant);

        // When something was, as a drawer's fact says it: its day as a row
        // says one, then its time, never minutes ago. [what, when, said]
        const moments = [
            ["twelve minutes ago", at(9, 26, 15, 8), "15:08"],
            ["this morning", at(9, 26, 6, 6), "06:06"],
            ["yesterday", at(9, 25, 14, 10), "yesterday 14:10"],
            ["on Thursday", at(9, 22, 9, 0), "Thursday 09:00"],
            ["a week ago today", at(9, 19, 9, 0), "19 Oct 09:00"],
            ["last year", new Date(2025, 9, 28, 9, 0).getTime(), "28 Oct 2025 09:00"],
            ["tomorrow, by a wrong clock", at(9, 27, 9, 0), "tomorrow 09:00"]
        ];
        for (const [what, time, meant] of moments)
            check(`a moment ${what}`, Mail.moment(time, now, said), meant);

        check("the line marking it read", Mail.tidy(mail, "read"), '{"read":{"id":"m_1"}}\n');
        check("the line archiving it", Mail.tidy(mail, "archive"),
            '{"archive":{"id":"m_1"}}\n');
        check("the line marking it unread again", Mail.tidy(mail, "unread"),
            '{"unread":{"id":"m_1"}}\n');
        check("read, once it's gone", Mail.tidied("read"), "read");
        check("unread, once it's back", Mail.tidied("unread"), "unread");
        check("archived, once it's gone", Mail.tidied("archive"), "archived");
        check("nothing for what isn't asked", Mail.tidied("delete"), "");

        // What a piece of mail's row and its details say.
        const lunch = {
            id: "m_2", from: "Ada Lovelace", address: "ada@example.com",
            to: "Joel Gerber, Grace Hopper", subject: "Lunch & a walk?", at: at(9, 26, 14, 2),
            page: "https://mail.google.com/mail/u/0/#inbox/t_2", body: "Friday?\n\nAda"
        };
        check("its side, who it's from and when", Mail.beside(lunch, now, said),
            "Ada Lovelace \u00b7 14:02");
        check("its side, when alone, with no name",
            Mail.beside({ from: "", at: at(9, 26, 9, 0) }, now, said), "09:00");
        // Mail Joel sent names who it went to, as the program gives them,
        // never him: one or two by name, then how many more.
        const wentSide = went => Mail.beside({ from: "Sam Rivers", to: "Sam Rivers, Ada Lovelace",
            sent: true, went: went, at: lunch.at }, now, said);
        check("its side, when Joel sent it, who it went to", wentSide(["Ada Lovelace"]),
            "to Ada Lovelace \u00b7 14:02");
        check("its side, when Joel sent it to two, both",
            wentSide(["Ada Lovelace", "Grace Hopper"]),
            "to Ada Lovelace and Grace Hopper \u00b7 14:02");
        check("its side, when Joel sent it to many, two and how many more",
            wentSide(["Ada Lovelace", "Grace Hopper", "Alan Turing", "Hedy Lamarr",
                "Edsger Dijkstra"]),
            "to Ada Lovelace, Grace Hopper and 3 more \u00b7 14:02");
        check("its side, when Joel sent it to nobody named, when alone, never him",
            wentSide([]), "14:02");
        // As items, its right gives way by whole names and keeps the time
        // (VKCF3Z, 635ac574).
        const wentItems = went => Mail.besideItems({ from: "Sam Rivers", sent: true, went: went,
            at: lunch.at }, now, said);
        check("its side's names give way by whole names, then to how many in all",
            wentItems(["Ada Lovelace", "Grace Hopper", "Alan Turing", "Hedy Lamarr",
                "Edsger Dijkstra"])[0].ways,
            ["to Ada Lovelace, Grace Hopper and 3 more", "to Ada Lovelace and 4 more",
                "to 5 people"]);
        check("its side's two names give way to one and 1 more",
            wentItems(["Ada Lovelace", "Grace Hopper"])[0].ways,
            ["to Ada Lovelace and Grace Hopper", "to Ada Lovelace and 1 more", "to 2 people"]);
        check("its side's one name has no other way", wentItems(["Ada Lovelace"])[0].ways,
            ["to Ada Lovelace"]);
        check("its side's names and time never give way whole",
            wentItems(["Ada Lovelace"]).map(item => item.stays), [true, true]);
        check("its side's time stays alone for nobody named",
            wentItems([]).map(item => [item.said, item.stays]), [["14:02", true]]);
        check("its side's sender stays whole beside the time",
            Mail.besideItems(lunch, now, said).map(item => [item.said, item.stays]),
            [["Ada Lovelace", true], ["14:02", true]]);
        check("when it came, in full, as a meeting's day is", Mail.dated(lunch.at, now, said),
            "Monday 26 October, 14:02");
        check("when it came last year, in full",
            Mail.dated(new Date(2025, 9, 28, 9, 0).getTime(), now, said),
            "Tuesday 28 October 2025, 09:00");
        check("its details", Mail.details(lunch, "No Subject", now, said), {
            title: "Lunch & a walk?",
            facts: [
                { label: "From", value: "Ada Lovelace <ada@example.com>" },
                { label: "To", value: "Joel Gerber, Grace Hopper" },
                { label: "Date", value: "Monday 26 October, 14:02" }
            ],
            body: "Friday?\n\nAda"
        });
        check("the details of mail copied to someone, after who it went to",
            Mail.details(Object.assign({}, lunch, { cc: "Grace Hopper" }), "No Subject", now,
                said).facts.map(fact => fact.label), ["From", "To", "Cc", "Date"]);
        check("who it was copied to",
            Mail.details(Object.assign({}, lunch, { cc: "Grace Hopper" }), "No Subject", now,
                said).facts[2], { label: "Cc", value: "Grace Hopper" });
        const menu = { name: "menu.pdf", kind: "application/pdf", size: 48213, runs: false };
        const patio = { name: "patio.jpg", kind: "image/jpeg", size: 812, runs: false };
        check("the details of mail carrying files",
            Mail.details(Object.assign({}, lunch, { attachments: [menu, patio] }), "No Subject",
                now, said).facts[3], { label: "Attachments", value: "menu.pdf, patio.jpg" });
        check("the details of mail carrying one file",
            Mail.details(Object.assign({}, lunch, { attachments: [menu] }), "No Subject", now,
                said).facts[3], { label: "Attachment", value: "menu.pdf" });
        // A file it carries: its kind by its ending, its size as the launcher
        // says one, and its details.
        check("a file's side", Mail.sized(menu, "File"), "PDF \u00b7 48 kB");
        check("a small file's side", Mail.sized(patio, "File"), "JPG \u00b7 812 B");
        check("a file with no ending", Mail.kind({ name: "README" }, "File"), "File");
        check("a hidden name is no ending", Mail.kind({ name: ".profile" }, "File"), "File");
        check("sizes below ten keep a figure", [1500, 2.5e6, 12e9].map(Mail.weighed),
            ["1.5 kB", "2.5 MB", "12 GB"]);
        check("a file's details", Mail.carried(menu, "File"), {
            title: "menu.pdf",
            facts: [{ label: "Kind", value: "PDF" }, { label: "Size", value: "48 kB" }],
            body: ""
        });
        // Fetching one asks the program by its mail and place, and its row
        // says how it stands for an ask made since.
        check("the line fetching a file", Mail.fetch(lunch, 1, "save"),
            '{"fetch":{"id":"m_2","file":1,"how":"save"}}\n');
        check("the line saving a file where Joel picked",
            Mail.fetch(lunch, 1, "save", "/home/joel/Plans/menu.pdf"),
            '{"fetch":{"id":"m_2","file":1,"how":"save","to":"/home/joel/Plans/menu.pdf"}}\n');
        const fetched = (more) => Object.assign({ mail: "m_2", file: 0, how: "open", asked: 50,
            path: null, failed: null }, more);
        check("how a fetch stands", [
            Mail.fetchedAs(fetched({}), lunch, 0, 40),
            Mail.fetchedAs(fetched({ path: "/run/user/1000/google-files/m_2/menu.pdf" }), lunch,
                0, 40),
            Mail.fetchedAs(fetched({ how: "save", path: "/home/joel/Downloads/menu.pdf" }),
                lunch, 0, 40),
            Mail.fetchedAs(fetched({ failed: "unreachable" }), lunch, 0, 40),
            Mail.fetchedAs(fetched({ path: "/x" }), lunch, 0, 60),
            Mail.fetchedAs(fetched({ path: "/x" }), lunch, 1, 40),
            Mail.fetchedAs(null, lunch, 0, 40)
        ], ["fetching", "opened", "saved", "unreachable", "", "", ""]);
        check("what its row says",
            ["fetching", "saved", "savedThere", "runs", "opened"].map(Mail.fetchedSaid),
            ["fetching\u2026", "saved in Downloads", "saved",
                "a program or a script, so only saved", ""]);
        const bare = { from: "ada@example.com", address: "ada@example.com", to: "", subject: "",
            at: lunch.at };
        check("the details of mail saying little", Mail.details(bare, "No Subject", now, said), {
            title: "No Subject",
            facts: [
                { label: "From", value: "ada@example.com" },
                { label: "Date", value: "Monday 26 October, 14:02" }
            ],
            body: ""
        });
        check("a reply, to who sent it", Mail.replying(lunch),
            "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com"
                + "&su=Re%3A%20Lunch%20%26%20a%20walk%3F");
        check("a reply to a reply, marked once",
            Mail.replying(Object.assign({}, lunch, { subject: "RE: Lunch" })),
            "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com&su=RE%3A%20Lunch");
        check("no reply with no address", Mail.replying({ address: "", subject: "Lunch" }), "");
        check("a new mail to someone", Mail.writing(["ada@example.com"], ""),
            "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com");
        check("no new mail to nobody", Mail.writing([""], ""), "");

        // Whether a meeting or a mail is with someone, for a person's next
        // meeting, Unread and Recent (GoogleSource.qml).
        const minute = 60 * 1000;
        const standupWith = {
            start: lunch.at + 61 * minute, end: lunch.at + 91 * minute,
            organizer: { name: "Ada Lovelace", email: "Ada@Example.com" },
            guests: [{ name: "Grace Hopper", email: "grace@example.com", answer: "no" },
                { name: "Room 4", email: null, answer: "yes" }]
        };
        check("a meeting with them", Google.withThem(standupWith, ["grace@example.com"]), true);
        check("mail from them", Google.withThem(lunch, ["ada@example.com"]), true);
        check("nothing with a stranger", Google.withThem(standupWith, ["x@example.com"]), false);
        // Mail Joel sent or read is with whoever sent it and whoever it went
        // or was copied to; a mail that wants him is only its sender's.
        const sent = { address: "joel@example.com", others: ["ada@example.com"] };
        check("mail sent them", [Google.mailedWith(sent, ["ada@example.com"]),
            Google.mailedWith(lunch, ["ada@example.com"]),
            Google.mailedWith(sent, ["x@example.com"]),
            Google.withThem(sent, ["ada@example.com"])], [true, true, false, false]);

        // Reply All: to who sent it, copied to everyone else, each once.
        check("a reply to all, copied to the others", Mail.replyingAll(Object.assign({}, lunch,
            { others: ["grace@example.com", "alan@example.com", "grace@example.com"] })),
            "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com"
                + "&cc=grace%40example.com%2Calan%40example.com"
                + "&su=Re%3A%20Lunch%20%26%20a%20walk%3F");
        check("no reply to all when it went to nobody else", [lunch,
            Object.assign({}, lunch, { others: [] }),
            { address: "", subject: "Lunch", others: ["grace@example.com"] }]
            .map(Mail.replyingAll), ["", "", ""]);
        // Write to the Guests: its holder and every guest with an address,
        // each once, about the meeting.
        check("a mail to a meeting's guests", Mail.toTheGuests({
            title: "Standup",
            organizer: { name: "Ada Lovelace", email: "ada@example.com" },
            guests: [{ name: "Grace Hopper", email: "grace@example.com" },
                { name: "Room 4" }, { name: "Ada", email: "ada@example.com" }]
        }), "https://mail.google.com/mail/u/0/?view=cm&fs=1"
            + "&to=ada%40example.com%2Cgrace%40example.com&su=Standup");
        check("a mail to the guests of one Joel holds", Mail.toTheGuests({
            title: "Design Review", organizer: null,
            guests: [{ name: "Grace Hopper", email: "grace@example.com" }]
        }), "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=grace%40example.com"
            + "&su=Design%20Review");
        check("no mail for a meeting with nobody else",
            Mail.toTheGuests({ title: "Focus", organizer: null, guests: [] }), "");

        // Which windows are Mail's, as Chrome names them.
        const listed = [
            { id: 1, app_id: "chrome-mail.google.com__-Default" },
            { id: 2, app_id: "chrome-mail.google.com__mail_u_0_-Default" },
            { id: 3, app_id: "chrome-meet.google.com__abc-defg-hij-Default" },
            { id: 4, app_id: "kitty" },
            { id: 5, app_id: null }
        ];
        check("Mail's page and mail opened from the shell are Mail's", Mail.windows(listed),
            [1, 2]);
        check("no window is Mail's with none open", Mail.windows([]), []);

        // Which close once mail opened while Mail was open shows.
        check("none before the new one shows", Mail.superseded([1], [1]), []);
        check("the one open before, once the new one shows", Mail.superseded([1], [1, 6]), [1]);
        check("every one open before", Mail.superseded([1, 2], [1, 2, 6]), [1, 2]);
        check("not one Joel closed meanwhile", Mail.superseded([1, 2], [2, 6]), [2]);
        check("none with none open before", Mail.superseded([], [6]), []);

        // Only the parts whose text changed are taken again.
        const answered = Google.stateOf({ account: "ready", meetings: [standup],
            mail: [mail], unread: 2, heard: now });
        const first = Google.changed({}, answered);
        check("every part is taken the first time", first.parts, Object.keys(answered));
        const minuteOn = Google.stateOf({ account: "ready", meetings: [standup],
            mail: [mail], unread: 2, heard: now + minute });
        const after = Google.changed(first.texts, minuteOn);
        check("a minute on, only when Google answered", after.parts, ["heard"]);
        const more = Google.stateOf({ account: "ready", meetings: [standup],
            mail: [], unread: 1, heard: now + 2 * minute });
        check("mail read, the mail and its count too",
            Google.changed(after.texts, more).parts, ["mail", "unread", "heard"]);

        recentCases(check, at, now);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }

    // What Calendar's Recent and Mail's list, from what the program keeps of
    // the month before: the meetings held and the mail Joel had.
    function recentCases(check, at, now) {
        const held = (id, start, end, fields) => Object.assign({
            id: id, title: id, start: start, end: end, allDay: false, answer: "yes",
            link: null, kind: null, page: ""
        }, fields ?? {});
        const ids = meetings => meetings.map(meeting => meeting.id);
        // The month before today, by start, as Google.js gathers it.
        const month = [
            held("retro", at(9, 22, 9, 0), at(9, 22, 9, 30)),
            held("declined", at(9, 23, 9, 0), at(9, 23, 9, 30), { answer: "no" }),
            held("planning", at(9, 25, 14, 0), at(9, 25, 15, 0))
        ];
        // Today's, one over, one on and one to come, and one the month
        // held too, as a meeting over midnight might be.
        const today = [
            held("planning", at(9, 25, 14, 0), at(9, 25, 15, 0)),
            held("standup", at(9, 26, 9, 0), at(9, 26, 9, 15)),
            held("review", at(9, 26, 15, 0), at(9, 26, 16, 0)),
            held("later", at(9, 26, 17, 0), at(9, 26, 17, 30))
        ];
        check("the meetings held, newest first, each once, none said no to or not over",
            ids(Google.heldLately(month, today, now)), ["standup", "planning", "retro"]);
        check("one held ends just now", ids(Google.heldLately([], [
            held("ended", at(9, 26, 15, 0), now)], now)), ["ended"]);
        check("none held with nothing kept", Google.heldLately(undefined, [], now), []);
        // As Calendar's Recent lists them, a series once at its latest, with how
        // many of it came before (VKCF3Z, ccccbe5f).
        const series = [
            held("s_20261026T130000Z", at(9, 26, 9, 0), at(9, 26, 9, 15), { series: "s" }),
            held("lunch", at(9, 25, 12, 0), at(9, 25, 13, 0)),
            held("h_20261025", at(9, 25, 0, 0), at(9, 26, 0, 0), { allDay: true, series: "h" }),
            held("s_20261023T130000Z", at(9, 23, 9, 0), at(9, 23, 9, 15), { series: "s" }),
            held("s_20261022T130000Z", at(9, 22, 9, 0), at(9, 22, 9, 15), { series: "s" })
        ];
        check("a series once, at its latest", Google.seriesOnce(series)
            .map(one => [one.id, one.earlier]),
            [["s_20261026T130000Z", 2], ["lunch", 0], ["h_20261025", 0]]);
        check("no series with none held", Google.seriesOnce(undefined), []);
        // A series split at "this and following" is two, as Google keeps it,
        // and a meeting held once whose id ends in a date is its own.
        const split = [
            held("x_R20261102T150000_20261102T150000Z", at(9, 26, 9, 0), at(9, 26, 9, 15),
                { series: "x_R20261102T150000" }),
            held("x_20261019T150000Z", at(9, 19, 9, 0), at(9, 19, 9, 15), { series: "x" }),
            held("report_20261012", at(9, 12, 9, 0), at(9, 12, 9, 15)),
            held("report", at(9, 5, 9, 0), at(9, 5, 9, 15))
        ];
        check("a split series is two, an id ending in a date its own",
            Google.seriesOnce(split).map(one => [one.id, one.earlier]), [
                ["x_R20261102T150000_20261102T150000Z", 0], ["x_20261019T150000Z", 0],
                ["report_20261012", 0], ["report", 0]
            ]);
        check("a series by Google's id for it", [
            { id: "s_20261026T130000Z", series: "s" }, { id: "once_20261026" }, { id: "once" }
        ].map(Google.seriesOf), ["s", "once_20261026", "once"]);
        const before = { earlier: "{count} in the past 30 days" };
        check("how many came before, in its words", Google.earlier(8, before),
            "8 in the past 30 days");
        check("none came before", Google.earlier(0, before), "");

        const mail = (id, came) => ({ id: id, from: "Ada", subject: id, at: came });
        const wanting = [mail("lunch", at(9, 26, 15, 18))];
        const had = [
            mail("sent", at(9, 26, 11, 0)),
            mail("lunch", at(9, 26, 15, 18)),
            mail("read", at(9, 26, 14, 0)),
            mail("old", at(9, 1, 9, 0))
        ];
        check("the mail had, newest first, none still wanting him",
            Mail.lately(had, wanting).map(one => one.id), ["read", "sent", "old"]);
        // A conversation once, by its newest; mail with no page is each its own.
        const thread = "https://mail.google.com/mail/u/0/#inbox/t1";
        const talks = [
            Object.assign(mail("a", at(9, 26, 14, 2)), { page: thread }),
            Object.assign(mail("c", at(9, 26, 10, 0)), { page: `${thread}x` }),
            Object.assign(mail("b", at(9, 26, 15, 0)), { page: thread }),
            Object.assign(mail("d", at(9, 26, 9, 0)), { page: "" }),
            Object.assign(mail("e", at(9, 26, 8, 0)), { page: "" })
        ];
        check("a conversation once, by its newest",
            Mail.lately(talks, []).map(one => one.id), ["b", "c", "d", "e"]);
        // A conversation that waits on him, its newest mail wanting him, is
        // Mail's row above, so none of its older mail is Recent either.
        const waits = [Object.assign(mail("f", at(9, 26, 15, 10)), { page: thread })];
        check("none of a conversation still wanting him",
            Mail.lately(talks, waits).map(one => one.id), ["c", "d", "e"]);
        check("at most nine", Mail.lately(Array.from({ length: 12 },
            (_, hour) => mail(`m${hour}`, at(9, 26, hour, 0))), []).length, 9);
        check("the newest of them first", Mail.lately(Array.from({ length: 12 },
            (_, hour) => mail(`m${hour}`, at(9, 26, hour, 0))), [])[0].id, "m11");
    }
}
