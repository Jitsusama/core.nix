.pragma library
// Who a folder the disk guard measured belongs to, and how Disk's Talk to
// Its Session row says them (Machine.qml), so Joel can tell the session
// working there what to do about it.

// A slash or several closing a path, which a cwd may or may not end with.
const TRAILING_SLASHES = /\/+$/;

// The sessions a holder belongs to, from sessions as Sessions lists them,
// each with `changed`, when its record last changed, and the guard's
// holder, { path, owner: { kind, name } }: those at work on the quest a
// quest's workspace is for, or else those whose record's cwd is the holder
// or inside it, as a tree a session works in, since a session's `folder`
// is only its last part; the one that worked last first (VKCF3Z,
// 2026-11-08). None where none is.
function owners(sessions, holder) {
    const owner = holder.owner ?? {};
    const path = holder.path ?? "";
    const questOf = one => one.presence?.questInfo?.id ?? one.presence?.quest ?? null;
    const inside = folder => typeof folder === "string" && path !== ""
        && (folder === path || folder.startsWith(`${path}/`));
    const owns = owner.kind === "quest" ? one => questOf(one) === owner.name
        : one => inside(String(one.presence?.cwd ?? "").replace(TRAILING_SLASHES, ""));
    return sessions.filter(owns).sort((one, other) => (other.changed ?? 0) - (one.changed ?? 0));
}

// The quests among the guard's holders, by id, which Machine names to
// Quests as they arrive, so each title is known before Joel looks.
function quests(holders) {
    return (holders ?? []).filter(holder => holder.owner?.kind === "quest")
        .map(holder => holder.owner.name);
}

// What Disk's What's Filling It calls a holder, in `worded`, the words'
// phrase for its kind, such as "{name}'s workspace": a quest by its title
// from `titled`, else its id; a worktree without its prefix; any other
// folder by its path from `home`.
function called(holder, worded, titled, home) {
    const owner = holder.owner ?? {};
    const name = owner.kind === "quest" ? titled(owner.name) || owner.name
        : owner.kind === "worktree" ? String(owner.name).replace(/^worktree-/, "")
        : owner.kind === "other" ? holder.path.replace(home, "~")
        : owner.name;
    const made = (worded ?? "{name}").replace("{name}", name);
    return made.charAt(0).toUpperCase() + made.slice(1);
}

// A talk row's side: the session's title, then how it stands, each an
// item, so where room runs short the state gives way whole and who the
// talk goes to is kept, never cut to its last word ("Quest \u00b7
// listening" for Listening Quest).
function talkSide(title, stands) {
    return [{ said: title ?? "", keep: 2 }, { said: stands ?? "", keep: 1 }]
        .filter(item => item.said !== "");
}
