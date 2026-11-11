// A notice still open, as the notices daemon (modules/notices) serves it,
// made to answer what the shell read of Quickshell's own Notification while
// the shell served them: its words, its sender, its actions, and the acts
// it's put away, let go or answered by, each now an ask of the daemon. The
// cards, the held list and the launcher's acts keep reading it as they did
// (Notices.qml's live). Worked out apart from Notices.qml so
// tests/live-notice.qml can hold it in Qt's own engine.
.pragma library

// Its urgency in Quickshell's numbers (NotificationUrgency), so a card's
// `urgency === NotificationUrgency.Critical` reads as it did.
const urgencies = { low: 0, normal: 1, critical: 2 };

// The notice `notice` (the record's own fields) as it's handed to the
// shell. `ask` sends the daemon an ask; `token` asks niri for an
// activation token, calling back with it, or "" for none, so the program
// an action is chosen for can bring its window forward
// (Quickshell.activationToken). It's asked as the action is chosen, since
// niri gives one only for the input just made.
function of(notice, ask, token) {
    const id = notice.id;
    return {
        id: id,
        appName: notice.app,
        appIcon: notice.icon,
        // Its own picture, a file the daemon keeps where one was sent as
        // pixels, or "" where it has none.
        image: notice.picture,
        desktopEntry: notice.entry,
        summary: notice.summary,
        body: notice.body,
        urgency: urgencies[notice.urgency] ?? urgencies.normal,
        hints: notice.hints ?? {},
        expireTimeout: notice.expire,
        transient: notice.transient,
        resident: notice.resident,
        hasInlineReply: notice.reply !== null && notice.reply !== undefined,
        inlineReplyPlaceholder: notice.reply ?? "",
        actions: (notice.actions ?? []).map(action => ({
            identifier: action.id,
            text: action.text,
            invoke: () => token(given => ask({
                ask: "invoke", notice: id, action: action.id, token: given
            }))
        })),
        // Put away by Joel, as a click does.
        dismiss: () => ask({ ask: "dismiss", notices: [id] }),
        // Gone because its time is up.
        expire: () => ask({ ask: "expire", notices: [id] }),
        sendInlineReply: text => ask({ ask: "reply", notice: id, text: text })
    };
}

// No notice: what a card reads for the moment between its notice going
// and the card going with it, saying nothing and asking nothing.
const none = of({
    id: -1, app: "", icon: "", picture: "", entry: "", summary: "", body: "",
    urgency: "normal", expire: -1, transient: false, resident: false, reply: null
}, () => {}, () => {});
