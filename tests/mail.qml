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
                    heard: 1793042446024
                }],
            ["a state with nothing heard yet",
                '{"account":"none","meetings":[],"mail":[],"unread":null,"heard":null}',
                { account: "none", meetings: [], mail: [], unread: null, heard: null }],
            ["a state from before mail",
                '{"account":"ready","meetings":[],"heard":1}',
                { account: "ready", meetings: [], mail: [], unread: null, heard: 1 }],
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
