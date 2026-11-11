// A notice the notices daemon serves, as the shell is handed it, run in Qt's
// own JavaScript engine, the one the shell runs it in: what the cards read
// of it, and each act an ask of the daemon, an action's carrying the token
// niri gave. The `a-live-notice-asks-the-daemon` check copies
// modules/quickshell/LiveNotice.js and tests/lib/Expect.js beside this file
// and runs `qml -platform offscreen live-notice.qml`.
import QtQuick
import "Expect.js" as Expect
import "LiveNotice.js" as LiveNotice

QtObject {
    Component.onCompleted: {
        try {
            const asked = [];
            const ask = said => asked.push(said);
            const given = token => answer => answer(token);
            const record = {
                id: 7, app: "Slack", icon: "slack", entry: "slack", summary: "Ana",
                body: "<b>hi</b>", picture: "file:///ana.png", urgency: "critical",
                hints: { "x-waiting": "todo:1" }, expire: 0, transient: false,
                resident: false, reply: "Reply to Ana",
                actions: [{ id: "default", text: "Open" }, { id: "read", text: "Mark as read" }]
            };
            const card = { from: { glyph: "S", hue: "person", name: "Slack" },
                words: { plain: "hi", styled: "<b>hi</b>", links: [] } };
            const slack = LiveNotice.of(record, ask, given("tok"), card);
            Expect.same("its card's sender and words are the daemon's", [slack.from,
                slack.words], [card.from, card.words]);

            Expect.same("it reads as Quickshell's did", [slack.id, slack.appName,
                slack.appIcon, slack.desktopEntry, slack.summary, slack.body, slack.image,
                slack.expireTimeout],
                [7, "Slack", "slack", "slack", "Ana", "<b>hi</b>", "file:///ana.png", 0]);
            Expect.same("its urgency is Quickshell's number", slack.urgency, 2);
            Expect.same("its hints are its own", slack.hints["x-waiting"], "todo:1");
            Expect.same("it takes a reply, said as it asks", [slack.hasInlineReply,
                slack.inlineReplyPlaceholder], [true, "Reply to Ana"]);
            Expect.same("its actions are named as they were", slack.actions.map(action =>
                [action.identifier, action.text]),
                [["default", "Open"], ["read", "Mark as read"]]);

            slack.actions[0].invoke();
            slack.dismiss();
            slack.expire();
            slack.sendInlineReply("on my road");
            Expect.same("each act is an ask, the action's with its token", asked, [
                { ask: "invoke", notice: 7, action: "default", token: "tok" },
                { ask: "dismiss", notices: [7] },
                { ask: "expire", notices: [7] },
                { ask: "reply", notice: 7, text: "on my road" }
            ]);

            asked.length = 0;
            const plain = LiveNotice.of({ id: 8, app: "mail", urgency: "low", reply: null },
                ask, given(""));
            Expect.same("one plain takes no reply and offers nothing", [plain.hasInlineReply,
                plain.inlineReplyPlaceholder, plain.actions.length], [false, "", 0]);
            Expect.same("low is Quickshell's low", plain.urgency, 0);
            LiveNotice.of(Object.assign({}, record, { id: 9 }), ask, given(""))
                .actions[1].invoke();
            Expect.same("with no token, the action is still asked", asked,
                [{ ask: "invoke", notice: 9, action: "read", token: "" }]);
            Expect.same("no notice says nothing and offers nothing", [LiveNotice.none.summary,
                LiveNotice.none.actions.length, LiveNotice.none.urgency,
                LiveNotice.none.from.name, LiveNotice.none.words.styled], ["", 0, 1, "", ""]);
            Qt.exit(0);
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
