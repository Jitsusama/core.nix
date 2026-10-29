// What the shell reads of the google program's file, when mail came in
// words, and the lines marking it read or archiving it, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-shell-reads-the-mail` check copies Google.js, Mail.js and
// Calendar.js beside this file and runs `qml -platform offscreen mail.qml`.
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
        const standup = {
            id: "standup", title: "Standup", start: now, end: now, allDay: false,
            answer: "yes", link: null, kind: null, page: ""
        };
        const mail = {
            id: "m_1", from: "Ada", subject: "Lunch", at: now,
            page: "https://mail.google.com/mail/u/0/#inbox/t_1"
        };

        // What the file says: [what, the line, what's read of it]
        const lines = [
            ["the whole state",
                JSON.stringify({
                    account: "ready", meetings: [standup], mail: [mail], unread: 3,
                    heard: 1793042446024
                }),
                {
                    account: "ready", meetings: [standup], mail: [mail], unread: 3,
                    documents: [], heard: 1793042446024, fetched: null
                }],
            ["a state with nothing heard yet",
                '{"account":"none","meetings":[],"mail":[],"unread":null,"heard":null}',
                {
                    account: "none", meetings: [], mail: [], unread: null, documents: [],
                    heard: null, fetched: null
                }],
            ["a state from before mail",
                '{"account":"ready","meetings":[],"heard":1}',
                {
                    account: "ready", meetings: [], mail: [], unread: null, documents: [],
                    heard: 1, fetched: null
                }],
            ["half a line", '{"account":"rea', null],
            ["a line that isn't the state", '{"hello":1}', null],
            ["an account with no meetings said", '{"account":"ready"}', null]
        ];
        for (const [what, line, meant] of lines)
            check(what, Google.heard(line), meant);

        // When it came: [what, when, said]
        const cames = [
            ["this afternoon", at(9, 26, 14, 2), "14:02"],
            ["just after midnight", at(9, 26, 0, 5), "00:05"],
            ["late last night", at(9, 25, 23, 50), "yesterday"],
            ["on Thursday", at(9, 22, 9, 0), "Thursday"],
            ["on Tuesday last week", at(9, 20, 9, 0), "Tuesday"],
            ["a week ago today", at(9, 19, 9, 0), "19 October"],
            ["last month", at(8, 30, 9, 0), "30 September"]
        ];
        for (const [what, time, said] of cames)
            check(`came ${what}`, Mail.came(time, now), said);

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
        check("its side, who it's from and when", Mail.beside(lunch, now),
            "Ada Lovelace \u00b7 14:02");
        check("its side, when alone, with no name", Mail.beside({ from: "", at: now }, now),
            "15:20");
        check("when it came, in full", Mail.dated(lunch.at), "Monday 26 October, 14:02");
        check("its details", Mail.details(lunch, "No Subject"), {
            title: "Lunch & a walk?",
            facts: [
                { label: "From", value: "Ada Lovelace <ada@example.com>" },
                { label: "To", value: "Joel Gerber, Grace Hopper" },
                { label: "Date", value: "Monday 26 October, 14:02" }
            ],
            body: "Friday?\n\nAda"
        });
        check("the details of mail copied to someone, after who it went to",
            Mail.details(Object.assign({}, lunch, { cc: "Grace Hopper" }), "No Subject")
                .facts.map(fact => fact.label), ["From", "To", "Cc", "Date"]);
        check("who it was copied to",
            Mail.details(Object.assign({}, lunch, { cc: "Grace Hopper" }), "No Subject")
                .facts[2], { label: "Cc", value: "Grace Hopper" });
        const menu = { name: "menu.pdf", kind: "application/pdf", size: 48213, runs: false };
        const patio = { name: "patio.jpg", kind: "image/jpeg", size: 812, runs: false };
        check("the details of mail carrying files",
            Mail.details(Object.assign({}, lunch, { attachments: [menu, patio] }),
                "No Subject").facts[3], { label: "Attachments", value: "menu.pdf, patio.jpg" });
        check("the details of mail carrying one file",
            Mail.details(Object.assign({}, lunch, { attachments: [menu] }), "No Subject")
                .facts[3], { label: "Attachment", value: "menu.pdf" });
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
        check("what its row says", ["fetching", "saved", "runs", "opened"].map(Mail.fetchedSaid),
            ["fetching\u2026", "saved in Downloads", "a program or a script, so only saved", ""]);
        const bare = { from: "ada@example.com", address: "ada@example.com", to: "", subject: "",
            at: lunch.at };
        check("the details of mail saying little", Mail.details(bare, "No Subject"), {
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

        // Everyone the meetings and mail name, for People, by address, each
        // with when what's with them is.
        const minute = 60 * 1000;
        const standupWith = {
            start: lunch.at + 61 * minute, end: lunch.at + 91 * minute,
            organizer: { name: "Ada Lovelace", email: "Ada@Example.com" },
            guests: [{ name: "Grace Hopper", email: "grace@example.com", answer: "no" },
                { name: "Room 4", email: null, answer: "yes" }]
        };
        const alan = { address: "alan@example.com", from: "Alan", at: lunch.at - minute };
        const meetingSpan = { from: standupWith.start, to: standupWith.end };
        check("who the meetings and mail name", Google.people([standupWith], [lunch, alan]), [
            { emails: ["ada@example.com"], name: "Ada Lovelace",
                spans: [meetingSpan, { from: lunch.at, to: lunch.at }] },
            { emails: ["grace@example.com"], name: "Grace Hopper", spans: [meetingSpan] },
            { emails: ["alan@example.com"], name: "Alan", spans: [{ from: alan.at, to: alan.at }] }
        ]);
        check("a meeting with them", Google.withThem(standupWith, ["grace@example.com"]), true);
        check("mail from them", Google.withThem(lunch, ["ada@example.com"]), true);
        check("nothing with a stranger", Google.withThem(standupWith, ["x@example.com"]), false);

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

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
