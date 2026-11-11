// A notice the notices daemon serves, as the shell is handed it, run in Qt's
// own JavaScript engine, the one the shell runs it in: what the cards read
// of it, and each act an ask of the daemon, an action's carrying the token
// niri gave. The `a-live-notice-asks-the-daemon` check copies
// modules/quickshell/LiveNotice.js beside this file and runs
// `qml -platform offscreen live-notice.qml`.
import QtQuick
import "LiveNotice.js" as LiveNotice

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
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        const asked = [];
        const ask = said => asked.push(said);
        const given = token => answer => answer(token);
        const record = {
            id: 7, app: "Slack", icon: "slack", entry: "slack", summary: "Ana",
            body: "<b>hi</b>", picture: "file:///ana.png", urgency: "critical",
            hints: { "x-waiting": "todo:1" }, expire: 0, transient: false, resident: false,
            reply: "Reply to Ana",
            actions: [{ id: "default", text: "Open" }, { id: "read", text: "Mark as read" }]
        };
        const slack = LiveNotice.of(record, ask, given("tok"));

        holds("it reads as Quickshell's did", [slack.id, slack.appName, slack.appIcon,
            slack.desktopEntry, slack.summary, slack.body, slack.image, slack.expireTimeout],
            [7, "Slack", "slack", "slack", "Ana", "<b>hi</b>", "file:///ana.png", 0]);
        holds("its urgency is Quickshell's number", slack.urgency, 2);
        holds("its hints are its own", slack.hints["x-waiting"], "todo:1");
        holds("it takes a reply, said as it asks", [slack.hasInlineReply,
            slack.inlineReplyPlaceholder], [true, "Reply to Ana"]);
        holds("its actions are named as they were", slack.actions.map(action =>
            [action.identifier, action.text]), [["default", "Open"], ["read", "Mark as read"]]);

        slack.actions[0].invoke();
        slack.dismiss();
        slack.expire();
        slack.sendInlineReply("on my road");
        holds("each act is an ask, the action's with its token", asked, [
            { ask: "invoke", notice: 7, action: "default", token: "tok" },
            { ask: "dismiss", notices: [7] },
            { ask: "expire", notices: [7] },
            { ask: "reply", notice: 7, text: "on my road" }
        ]);

        asked.length = 0;
        const plain = LiveNotice.of({ id: 8, app: "mail", urgency: "low", reply: null },
            ask, given(""));
        holds("one plain takes no reply and offers nothing", [plain.hasInlineReply,
            plain.inlineReplyPlaceholder, plain.actions.length], [false, "", 0]);
        holds("low is Quickshell's low", plain.urgency, 0);
        LiveNotice.of(Object.assign({}, record, { id: 9 }), ask, given("")).actions[1].invoke();
        holds("with no token, the action is still asked", asked,
            [{ ask: "invoke", notice: 9, action: "read", token: "" }]);
        holds("no notice says nothing and offers nothing", [LiveNotice.none.summary,
            LiveNotice.none.actions.length, LiveNotice.none.urgency], ["", 0, 1]);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
