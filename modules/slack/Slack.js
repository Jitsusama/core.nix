// What Slack's source in People (SlackSource.qml) decides from what
// slack-people said, kept apart from the reader it asks so Qt's own
// engine can test it (tests/slack.qml): who to ask and when, what stands
// in Write in Slack's place, and the drawer's facts. `known` is what Slack
// said of each address, by address: { email, id, team, name, zone,
// presence, status, emoji }, with no id for one it knows nobody by.
.pragma library

// What Slack found of any of their addresses, or null.
function found(emails, known) {
    return emails.map(email => known[email]).find(said => said?.id) ?? null;
}

// What stands in Write in Slack's place for someone Slack hasn't found
// (VKCF3Z, 3bbad443 and 8a0f14ea): "looking" while one of their addresses
// is still to be answered and there's a sign-in to ask with, "missing"
// once a step-in found nobody by any, until the launcher opens again, or
// "" for no row at all, as for someone already known not to be in Slack.
function waiting(emails, known, missed, asks) {
    if (asks && emails.some(email => !known[email])) {
        return "looking";
    }
    if (emails.some(email => missed[email])) {
        return "missing";
    }
    return "";
}

// Their addresses to ask about as Joel steps into them: each not known to
// be nobody, not answered in the last `when.fresh` milliseconds and not
// asked about in them either, so an ask Slack never answered is asked
// again. `when` is { answered, asking, now, fresh }: when each address
// was answered and asked, by address, the time now, and how long either
// holds.
function due(emails, known, when) {
    const recently = at => at !== undefined && when.now - at < when.fresh;
    const nobody = email => known[email] && !known[email].id;
    return emails.filter(email => !(nobody(email) || recently(when.answered[email])
        || recently(when.asking[email])));
}

// The addresses of those listed to ask about ahead, in the order they're
// listed, each once: none Slack has answered for, and none sent ahead
// already.
function ahead(listed, known, sent) {
    const asked = [];
    const seen = Object.assign({}, sent);
    for (const person of listed) {
        for (const email of person?.emails ?? []) {
            if (!(known[email] || seen[email])) {
                seen[email] = true;
                asked.push(email);
            }
        }
    }
    return asked;
}

// The drawer's facts of someone Slack found, each only while it says.
function facts(said) {
    const status = [said.emoji, said.status].filter(word => word).join(" ");
    const presence = said.presence
        ? said.presence[0].toUpperCase() + said.presence.slice(1) : "";
    return [
        { label: "Status", value: status },
        { label: "Presence", value: presence },
        { label: "Time Zone", value: said.zone, zone: true }
    ].filter(fact => fact.value);
}
