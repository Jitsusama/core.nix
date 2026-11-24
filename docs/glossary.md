# Glossary

What every thing on Joel's machines and in their tooling is called, used the
same way in code, directories, docs, on screen and in what agents say. [The
design][design] says how things look and behave; this says what they're
called. A word that isn't here yet hasn't been settled, and a word under
[Still to Settle](#still-to-settle) waits on one choice.

The words are kept in bounded contexts, one section each. Inside a context,
each concept has one term, and the term is what's written, what's shown and
what Joel types: there are no synonyms, so a term is learned once. An old
name still finds what it named, never shown, so a rename loses nothing
(Joel, 2026-10-26: "i didn't know its name had changed, i was used to
calling it caffeinate"). A word may mean its own thing in another context, and
the launcher's place, with its crumbs, says which: Network › Address is the
machine's, Agents › Sessions › Language › Address is a session's.

Each table has the same columns:

- **Word**: the term, as it's written in prose, shown on screen and typed.
  In prose it's in lower case unless it's a name, such as Bluetooth or Keep
  Awake, which is written as its title everywhere.
- **Means**: what it is, in a few plain words.
- **Not**: names it replaces. They're never written or shown, in any
  context; typed, one still finds the thing it named, whose row says
  "Once called" it.
- **In code**: how code spells it, where code has to spell it otherwise, such
  as a program's name for a module or NixOS's own option.

A title, the name of a place, a row or an act on screen, is in Title Case:
every word takes a capital but a, an, the, and, or, but, of, to, in, on, at,
by, for and with, which take one only as the first or last word, as in Keep
Awake, Do Not Disturb, Start a Session and Turn the Radio Off (Joel,
2026-10-26: "i hate things that look like sentences without punctuation"). A
note, a notice and what an empty list says are sentences, and a command is
written as it's typed, in lower case.

The-glossary-names-each-thing-once holds every table to that: no word names two
things in one context, and no replaced name is written for anything.
No-name-says-a-replaced-word holds core's own names to it: no check's name
and no `jitsusama.*` option's says a replaced name, but the ones
`tests/repository/names-kept.nix` keeps as another sense of the word or as a rename still
waiting on its owner, and what it keeps only shrinks. A repository built on
core holds its own names the same way, through `core.lib.namesSayTheGlossary`.
A launcher row that names a thing gives its term as `Context/word`, and a
place named for a context gives the context alone, as `Network`;
the-launcher-names-each-thing-its-term holds the row's name to it.

## The Context Map

Ten contexts have places in the launcher. Nine are home's rows, in its
order after Now: Agents, Code, Programs, People, Passwords, Desktop, Machine,
Network and Keyboards, so home has ten rows. Disk's place is inside Machine,
beside Processor, Memory and Heat, and Mail and Calendar are places inside
Programs. Security, which the others lean on, has one place, Passwords, on a
machine with a password manager. The Library and Checks, which are core.nix's
own, are never places.

- **People reaches across Programs**: a person is found in mail, meetings,
  calls and chat at once, each program a source of people.

- **Desktop shows every context.** Each launcher place belongs to one; Now is a
  view onto the others' rows, each kept in its own place, with rows of its
  own first: For You, the moment (the time, the date, the location, the
  weather and the next meeting), then Lately's. A filling
  disk shows in Now, with its forecast, while it fills.
- **Agents works in Code**: a session works in a tree of a repository, and
  its work is put up as a change.
- **Agents uses Machine's queue**: a session's heavy work waits there, shown on
  the session as what it's waiting on, and listed whole in Agents' Queue.
- **Disk acts on Machine's jobs**: the disk guard freezes and thaws them.
- **Security seals Disk and signs Agents' work**: the TPM unlocks the disk,
  and Joel's key and the agents' key sign commits.
- **Network carries every context's traffic**, and the shaper keeps a call
  quick under it.
- **The Library builds every context**, and Checks prove it.
- **The system reads four at once**: Machine for every machine of the realm,
  Agents' mycelium, Network and Code, each part good, watch or bad as its own
  owner answers, and the whole as its worst part. `system` says it.

## Agents

The models working for Joel, what they work on, and what they cost. Its
sessions stand in one tree, each under its orchestrator, the orchestrators
closed at its end. Its places are Quests, Orchestrators, the Queue, Runs
and AI spend, then Budgets, how each budget stands, and Spec, how the
agents deliver against the bars Joel set, always last. A
session can be talked to, gone to, paused and stopped, and a closed
orchestrator reopened; a quest can start a session, orchestrate, go first in
the queue and conclude. The trees, checkouts and changes agents work in are
[Code](#code)'s words.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| agent | a model working for Joel, as a session or a subagent | AI, assistant, bot | |
| sandbox | the fence around a command an agent runs: what it may read, write, reach and be told | | `PI_SANDBOX`, landrun |
| broker | a program outside the sandbox holding a credential or a door the sandbox shuts, which hands what's inside only a short-lived or narrow part of it | | `github-broker`, `gcloud-broker`, `jobs-broker` |
| quest | a piece of Joel's work, with its own folder, README and documents | task, project, todo | `QEST-` ids, the `quest` tool |
| short name | a quest's name in two or three words, on its windows and in the launcher | | `short:` |
| subquest | a quest that is part of another's work | | `kind: subquest` |
| sidequest | a quest that came up beside another, not part of it | | `kind: sidequest` |
| plan | a quest's document for building something, through think, draft and build | | `PLAN-` ids |
| research | a quest's document for finding something out: question, method, findings, verdict | | `RSCH-` ids |
| brief | a quest's document asking someone for something: audience, ask, background, recommendation | | `BRIF-` ids |
| report | a quest's document on what happened, its outcomes and lessons | | |
| landing | what a session finished and handed Joel: a merged change, a built plan or a hand-off, at its ref, with a line saying what to try | | presence `landings` |
| Landed | the group of landings in Agents, named for its state as Unread and Recent are | | `agents:landed` |
| read | a landing Joel has seen, by its drawer or Return, which leaves For You for its History, as mail he has opened is read | | `agents:landed`'s mark |
| Journey | a quest's dated log of what happened | | `quest journal` |
| session | a pi working for Joel on a quest, in a window of its own | | `agents` in the launcher |
| carry | moving a quest's work and its session to another machine; Agents lists the day's in a row of their own, each by its session's title, else its quest's, how it stands, where it went and when it was ordered, and since a carry asks nothing of Joel, none stands in For You | | `carry`, `agents:carried` |
| stuck | the node's word for work standing still: a carry it judges stuck, by its own rule, said in alert in Agents' row of carries with the node's reason, or a network's cell standing paused and wanting nothing, which the node counts stuck in that network's counts, Agents' row of that network says, and its site draws stuck | | a carry's `stuck`, a network's `stuck` count |
| orchestrator | a session whose quest holds other quests: it hands out their work and gathers what waits on Joel | steward | `orchestrator: true` |
| orchestration | an orchestrator and every session on a quest below it | | `#orchestration/<address>` |
| talk | Joel's words to an orchestrator or a session, typed in its field from outside its window | steer | `steer/<ADDRESS>/` |
| offered work | words a page's mycelium: link puts in Start New Work's From a Link, which start only on Joel's own Return once he has checked them; the session the page says they're from is its claim, never vouched for | suggested work | `newWork`, Sprouting.js's `offered` |
| talk line | one line at the top of the screen, on Super+S, to say something to the focused window's session, else The Void, without opening the launcher | | `agents:line`, TalkLine.qml |
| pause | Joel stopping a session after its turn, from outside its window, until he resumes it, talks to it or types in it | | presence `paused` |
| resume | Joel letting a paused session carry on where it left off | | |
| Pause All Agents | Joel's own stop on every agent, from the launcher, here and on each machine that stops with this one, until he resumes them from its row in For You | | `agents:pause-all`, `pineapple serve`'s `pause` |
| park | a session ending itself while idle to give back its memory, until a message or a key wakes it in its window | | presence `parked`, park-workflow |
| reopen | opening a closed session again where it left off, in its own window and folder | restore | `restore.json` |
| Sessions to Reopen | the sessions the last boot's end took away or had parked, and those another machine carried here or pi's host gave back, each reopened only when Joel asks | Reopen Last Boot's Sessions | `toReopen` |
| carried | a session another machine handed this one as it went away, to reopen here | | `carried` in `restore.json` |
| given back | a session pi's host on this machine ended and handed back to plain pi, to reopen here; the host's own name is only an identifier | released | `released` in `restore.json` |
| waiting on | what a session can't go on without: Joel, an answer, or its turn in the queue | | |
| Working on | the line a session tells its peers about its work | Works on | `worksOn` |
| Doing | the step a session is on now: thinking, or a tool and what it's on | | `stepDetail` |
| Needs You For | what a session wants of Joel, said on it: an answer, a yes or an unblock | | `agents:wants` |
| waiting mark | the person with a clock, in the alert's hue, that a session's row wears in place of its own mark while it waits on Joel, so a row cut narrow still says so | | `standing.rs`'s `glyph` and `hue`, `waitsOnJoel` |
| put off | a want Joel set aside for an hour with Put It Off: it rings for nothing and leaves For You, and its session keeps the waiting mark in its own hue, lighting nothing above it, until the hour is up | | `putOff`, wants.rs's `PutOff` |
| turn | one round of a session's work, from the message or key that starts it to the answer it ends on | | `attention: "turn"`, for a mesh message that starts one at once |
| peer | another session on the same machine, reached over the mesh | instance, swarm | |
| mesh | how sessions on one machine message each other | | `mesh_send` |
| node | mycelium's program on a machine, carrying its mesh to the machines it links with | | `mycelium node` |
| link | two machines' nodes joined, carrying the mesh's messages and sessions between them | | `mycelium link`, `jitsusama.mycelium.links` |
| realm | a set of machines whose nodes link, such as Joel's work or personal ones, with nothing crossing between realms | | `jitsusama.mycelium.realm` |
| link key | the key a realm's node signs in to the realm's other machines with, held apart from every other realm's | | `jitsusama.linkKeys`, `<realm>-link` |
| mycelium | every cell on Joel's machines and the networks they share, seen and steered as one living thing | | `mycelium`, `jitsusama.mycelium` |
| cell | one living thing in the mycelium, such as a session or a receptor, with its own identity and purpose | | `mycelium cells`, a cell's `kind` |
| network | a channel cells that engage with each other share: messages passed between them, a shared file surface or a tree on a repository, which a worktree, being one session's alone, can't be | | `mycelium networks`, `tree.<repo key>:<branch>`, shown as `<repo>@<branch>` |
| feeds | what a machine does for a network it runs cells in: a network is fed by every machine its cells run on and belongs to none of them; a cell is never fed this way, only by a receptor (see fed) | hosts, owns | |
| receptor | a cell that takes in what comes from outside the mycelium, such as Slack, mail or a call, onto a network of its own | | cell type `receptor`, `outside.<source>` networks |
| fed | what a receptor did when it passed a cell something from outside, such as a review asked of Joel or a call's summary; the map draws it as a dot-and-dash arc for ten minutes; a network is fed in the other sense (see feeds) | handed | the node's `fed` |
| delegate | what a cell does when its purpose outgrows it but stays one aim: it keeps its identity and purpose, hands needs to other cells and judges the whole itself | | |
| split | what a cell does when its purpose has become two aims: it ends as an ancestor, and each part goes to a new cell born of it, with its own purpose and networks | | |
| forage | a cell's own purpose, with the research, plans and reports it keeps for it, which go with the cell and end with it | | `mycelium forage`, the `forage` kind |
| address | a peer's own group on the mesh, six letters | | |
| group | a set of peers a mesh message goes to, such as an orchestration's | | `#quest/<id>` |
| question | a mesh message asking peers for an answer | mesh question | `intent: question` |
| unanswered question | a question no peer has answered in five minutes | stuck question | |
| glance | a mesh message given to a peer as one line, since it's another peer's own business | | |
| hold | a peer asking every other to put something off, such as a switch, until it says it's over | | `intent: hold` |
| pivot | an orchestrator changing what a session it orchestrates is doing, each answering once it has changed course | | `intent: pivot` |
| fyi | a mesh message given as context only, asking nothing | | `intent: fyi` |
| hand | a subagent, seen on the mesh as its parent's hands | | `hand.ts` |
| cut | making a tree | scaffold | `work tree` |
| release | giving back a tree one holds | prune, reap | `work release`; `quest tree-release` |
| tidy | reporting which trees on the machine are spent, removing nothing | | `work tidy` |
| reclaim | taking back the trees nobody owns | | `work reclaim` |
| tree sweep | the harness giving back ended sessions' spent trees, each hour by a timer of its own, or by each pi host on its thread unless that timer's sweep finished in the last two hours and judged every provider the host has, so always on a Mac, which runs no timer | | `pi-sweep`, `jitsusama.pi.sweep` |
| scratch folder | a quest's folder for clones, data and builds, its tmp/ cleared when it concludes | | the quest workspace |
| version | one commit of a pi package installed on the machine, which a pi runs | | `versions/<commit>` |
| sweep | pi-packages removing the versions no pi runs, keeping current and the one before, at each pi's start or end; not the tree sweep | | `pi-packages sweep` |
| background command | a command a session runs without waiting on it | background job | the `jobs` tool |
| background work | a session's background commands, fleets and review rounds together | | `/jobs` |
| fleet | subagents run at once on one piece of work | fan-out, batch | the `subagent` tool |
| subagent | a pi a session starts to do part of its work | helper, sub-agent, worker, lane | the `subagent` tool |
| starts stopped | Joel's stop on every session an agent asks to start, until he lets them start again; sessions he starts himself still start | | `mycelium starts stop` |
| run | one running of subagents, a fleet, a council or a judge's round, as a whole | | `runs` in the launcher |
| council | reviewing models each reading one change on their own | | `review_ask council` |
| judge | the model that consolidates a council's findings | | |
| finding | something a review or a peer found, for whoever owns what it's about | | `intent: finding` |
| AI spend | what agents have cost | cost, usage | spend-watch |
| model | the AI a session or a subagent runs on, such as Claude Opus | | `provider/model` |
| context | what a session's model is sent each turn: its conversation so far and what it read | | |
| cache | the part of a session's context its model's provider kept from the turn before, sent again for less | | the prompt cache |
| rebuild | a session's context summed up and started afresh once it fills, its log kept whole | compaction | pi's compaction |
| tokens | what a model is sent and writes, counted as OpenTelemetry's gen_ai names them, never money | | `gen_ai.usage.*` |
| bill | what a provider charged for AI spend, as its feed reports it, rather than estimated from list prices | | |
| budget | a named source of money a model call draws on: a Claude plan, an API key, the AI proxy, a team's token | | `agents:budgets`, AI spend's Budgets |
| Spec | the bars Joel set for how the agents deliver, read on ten axes; an axis or a reading is in spec or out of spec | | `agents:spec`, cadence-spec |
| AI proxy | the service a work machine's model calls go through, whose feed gives spend-watch its bill | model gateway, LLM gateway, LLM proxy | |
| Claude plan | Joel's Claude subscription, whose allowances stop turns when spent | subscription | `claude-plan` |
| allowance | how much of a budget one window allows: a share of a Claude plan's five hours or week, or dollars of the AI proxy's week | limit | `5h`, `wk` |
| window | the stretch of time an allowance counts over, such as five hours or a week | | |
| Joel | the human every session works for | user, human, operator | |
| status line | the line under a session saying how it stands | status bar | |

## Code

Repositories, every copy of each on a machine, and the work put up on them.
The words are git's, with a source for each kind of remote, as Network has
one for each vendor: git (the disk), GitHub and GitLab are core's, and a
machine adds its own. A vendor's own words, such as a pull request's number
or a CI job's name, stay in its source's section of the record. Where a
checkout lives is a machine. Its command is `repos`.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| repository | one history of code, listed once however many checkouts of it a machine holds | repo | `repos`, `repos.repositories/1` |
| key | how a repository is named once: its remote's host and path, or its folder's when it has none | | `github:owner/name`, `local:<folder>` |
| remote | where a repository is kept beyond the machine, by its name and address | | `git remote` |
| source | a kind of remote repos reads, each with its own section of the record | forge | |
| checkout | a repository's whole copy on a machine, which trees are cut from; bare when it holds no files to work in | clone | |
| tree | a folder holding a repository's files to work in, cut from a checkout: a worktree or a snapshot | working tree | the `work` tool |
| worktree | a tree checked out at a branch, one session's alone | | `git worktree` |
| snapshot | a tree pinned at a commit, read only and shareable | | |
| gone | a tree its checkout still lists whose folder isn't there | | `prunable` |
| branch | a named line of commits a tree can be checked out at | | `refs/heads/` |
| commit | one recorded state of a repository's files | | |
| uncommitted | a checkout's tracked files changed since its last commit, staged or not | dirty | `git status` |
| trunk | the branch a repository's changes merge into, as its remote names it | default branch | `origin/HEAD` |
| change | work put up for review, whichever source holds it | PR, pull request, merge request, proposal | |
| stack | changes each built on the one before | | |
| check | what proves a change before it merges, run where its source runs it | CI, pipeline | |
| merge | a change taken into its trunk | | |

## Programs

What Joel runs, and the terminal he types commands into, and the mail,
meetings and calls he has through them. A
program can be opened, opened below, given a new window and quit. A meeting
can be joined or shown in Calendar, and an invitation answered yes, maybe or no; mail can
be read, marked unread and archived.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| program | something Joel runs, with or without a window | app, application | the `applications` provider |
| shell | zsh, which Joel types commands into in a terminal | | zsh |
| prompt | the line the shell writes before each command | | |
| command | a program of ours typed into the shell, by Joel or by an agent, keeping one shape for both | | `docs/commands.md` |
| answer | what a command writes on stdout, and nothing else does: the records, the value asked for, or the JSON | | stdout |
| form | how a command's answer is written: text for a person, json or jsonl for a machine | | `--format`, `terminal_kit::Format` |
| Google account | the account Joel's mail and calendar are kept in, which the desktop's own program holds | | the `google` program |
| mail | Joel's messages in his Google account, and the place in Programs holding them | | Gmail |
| wants you | mail waiting on Joel's answer | | |
| unread | mail Joel hasn't opened yet, or a Slack conversation with messages he hasn't read, which Slack's program lists as its Unread | | |
| read | mail Joel has opened | | |
| archive | taking mail out of the inbox, kept where it's found again | | |
| calendar | Joel's meetings in his Google account, and the place in Programs holding them | | Google Calendar |
| meeting | an event on Joel's calendar with a call or with others | | |
| invitation | a meeting someone asks Joel to, answered yes, maybe or no | | |
| going | an invitation Joel answered yes | accepted | `yes` |
| maybe | an invitation Joel answered maybe | tentative | |
| not going | an invitation Joel answered no | declined | `no` |
| not answered | an invitation Joel hasn't answered yet | needs action | `waiting` |
| call | a talk with others over Meet or Tuple, with sound and sometimes pictures | | Meet, Tuple |
| transcript | what was said on a call, Joel's words and the others' in the order they were said, written on the machine as the call goes | | `call-transcripts` |
| summary | a call's transcript summed up once it ends, on the machine: the lines a model picked as worth reading, each quoted word for word with who said it to whom and when, kept under the Summary heading of the call's `transcript.md`, over the transcript | | `## Summary` |
| noise cancelled | a microphone's voice with the sounds around it taken out on the machine, offered as a microphone of its own named for it | noise suppression, denoised | `noise-cancelled` |
| noise cancelling | which cleaner the microphones' noise is cancelled by now, an input's part, or none | | the trial daemon's `running` |
| cleaner | one way of cancelling noise, DeepFilterNet full or gentle or RNNoise, or none, the microphone as calls hear it | denoiser | `raw`, `deepfilternet`, `deepfilternet-gentle`, `rnnoise` |
| choose by ear | the blind trial picking an input's cleaner: a few seconds recorded, cleaned by each cleaner as clips A to D, played and one picked | A/B | `trial:now`, `NoiseCancelling.qml` |
| clip | one of a choose by ear's recordings, cleaned by a cleaner it doesn't name until a pick | | `A` to `D` |
| spot | where the machine sits, such as the desk or the couch, told apart by the home network's node it's on or the network away from home, and the screens and docks plugged in, and named in Joel's words once he names it; a microphone keeps a choose by ear pick for each, apart from a launcher's place | | `spot.rs`, the trial daemon's `spot` |
| join | going into a meeting's call, or a call another sends | | |
| cancel | taking a meeting Joel holds off everyone's calendar, Google telling each guest | delete | |
| Slack message | a message in Slack, found by searching Slack and opened in its conversation | | the Slack daemon's `slack-message` |
| Media | the place holding every player and what each is playing, beside Sound, which holds the devices it plays through | | `program:media` |
| player | a program playing sound or video that the desktop can pause, skip and go to, such as Spotify or a browser's tab | | MPRIS, `Playing.qml` |
| track | what a player is playing: one song, episode or video, with its artist and album | | MPRIS's `xesam` metadata |
| link | the web address a track, a page or a message opens at, which Copy the Link copies | | MPRIS's `xesam:url` |
| pause | a player holding what it plays where it is, until it plays on, apart from a session's pause | | MPRIS's `Pause` |
| resume | a video or song file playing on from where Joel stopped it last | | mpv's `watch_later` |
| podcast | a series of spoken episodes a player plays, such as one on Spotify | | Spotify's `show` |
| episode | one part of a podcast | | Spotify's `episode` |
| speed | how fast a player plays its track against the track's own pace, such as one and a half times | | MPRIS's `Rate` |
| download | a file the browser saved from the web, listed in its Recent while it's still where it was saved | | Chrome's `downloads` table, `chrome-recent-sites downloads` |
| kept here | the copies of Drive's files on this machine, to open, and how much of the room set aside for them they take | | the cache home's `google/drive`, Google's `kept` |
| site | a page or set of pages Joel opens in his browser from the sites' server on this machine, each a row on the sites' home | | `http://sites.localhost:8484/<site>/` |
| live site | a site built by Nix from TypeScript on the kit, drawing a daemon's records as they change | | `localSite`, `jitsusama.local-sites.mounts` |
| document | a site a session writes, kept as state rather than built, carrying a copy of the web kit as its own `kit/` and taking its keys from the kit | | `~/.local/share/jitsusama/sites/` |
| feed | the WebSocket the sites' server bridges to one daemon's socket, with the kinds and fields any page here may read of it | | `/feed/<name>`, `jitsusama.local-sites.feeds` |
| kit | what every live site shares from the sites' server: its feeds, records, keys and theme switch, served once; the web-design skill says the kit for either this or the web kit, and what it names says which | | `modules/local-sites/kit/`, `/local-sites/kit/` |
| web kit | the theme written for the web, its styles and fonts, served once for every live site | | `modules/theme/web/`, `/local-sites/web/` |

## People

The people Joel deals with, whichever program they're in. A person can be
written to, called, met with and made a favourite.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| person | someone Joel deals with, the same one by their email across mail, meetings, calls and chat | | |
| source | a program people are known from, such as Google, Tuple or Slack, each one an option | | the people daemon's sources |
| favourite | a person Joel starred, listed first in People, apart from a password manager's favourite | | `star`, `unstar` |

## Desktop

The screen and everything on it around the windows, the launcher among them.
A window can be gone to, closed, floated or tiled, moved and sized; a screen's
brightness, scale and rotation can be set; an item in a list, such as a notice
or a copy, can be opened, copied or forgotten.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| desktop | the screen and everything on it around the windows: the launcher, notices, the margin and niri's workspaces | | niri, Quickshell |
| screen | a monitor, built in or plugged in | display, monitor | niri's outputs |
| screen sharing | the screen shown to others, in a call or a cast, which niri tells the desktop of | screencast | niri's casts |
| screen mode | a size and refresh rate a screen can run at: the one it runs at now, and the ones it offers | | niri's `modes` |
| colour depth | how many bits a screen gives each of its red, green and blue | | the EDID's bit depth |
| colours | the range of colour a screen can show, said against sRGB's, apart from the theme's colour | | the EDID's gamut |
| white point | the white a screen's panel gives of itself, in kelvin and where it sits, apart from warmth, the white the desktop sets it to | | the EDID's white |
| primaries | where a screen's red, green and blue sit, the corners of its colours | | the EDID's primaries |
| brightness | how bright a screen is, which follows the room's light | | the `backlight` program |
| warmth | how warm a light's white is, in kelvin: a screen's, which follows the room's light or the night light, or the desk light's, apart from a panel's white point | | the `backlight` program's colour temperature |
| night light | a screen warmed to a set white from sunset to sunrise, wherever the machine is, whatever the room's light | | the `backlight` program's night |
| workspace | one of niri's rows of windows on a screen | | |
| overview | niri's view of every workspace and its windows, on Super+A | | |
| window | a program's rectangle on the screen | | |
| This Window | the launcher's first row in Desktop: the window Joel is in | | |
| wallpaper | the picture behind every window | painting | |
| notice | what a program said, on the arrival line as it comes and after in Now's Desktop, but Slack's in Slack's row and only while Slack's daemon can't answer, and the shell's own for a to-do or pi in neither | notification | `Notifications.qml`, `org.freedesktop.Notifications` |
| Now's Desktop | Lately's row, which reads "Desktop" on screen, for every notice but Slack's and the shell's own (a to-do's and pi's): those unread inside it and those read in its History; never home's Desktop, the place for the screens and windows | | `Lately.js`'s `desktop` |
| arrival line | one line at the top of the screen, where the launcher stands, showing the notice that waits on Joel as it comes, without taking the keys | | `NoticeLine.qml` |
| notice kind | what a notice is about, as its program files it, said in its drawer's eyebrow: message, mail, device, network, status or transfer, with failed or bounced after it when it went wrong | | `notices:kind`, the notification's `category` hint |
| Do Not Disturb | the setting that holds notices back while it's on, and on by itself while the screen may be shared | dnd, quiet | `session:quiet` |
| dictation | what Joel says, typed where the focus is, while he holds Super+D or after Return on its row | | `modules/dictation`, `Dictated.qml` |
| question | what the desktop asks Joel: a centred box for a password or a yes, or the file chooser's which file, or where to save | | `Polkit.qml`, `Askpass.qml`, the file chooser's `said` |
| mark | one drawn symbol, such as each one in the margin | glyph, icon | `glyphs.nix` |
| margin | the row of marks at the top of the screen saying what's true now | corner | `Margin.qml`, `margin.pieces`, `Pieces.qml` |
| margin's cat | the small cat sitting in the gap above the column Joel is in, which says nothing | | `modules/margin-cat`, `MarginCat.qml`, `MarginCatScreen.qml` |
| margin's puppy | a second small animal for the margin beside the cat, drawn in the theme but not yet shown | | `glyphs.nix`'s `puppySitting`, `puppyBowing` and `puppyWalkingA` to `D` |
| meter | a level shown while it changes, such as the volume | overlay | `Meter.qml` |
| card | the one frame the launcher and the arrival line's notice are drawn in, all it holds the inset from its edge | | `Card.qml` |
| drawer | the card's continuation out of its right side, a plate about the selected row: what it has beyond its one line | | `Drawer.qml`, a row's `drawer` |
| plate | a small page about one thing: its eyebrow, name, facts, preview and description | | |
| eyebrow | a plate's first line, above its name, saying where the thing is and then its kind, never its state | | a drawer's `eyebrow` |
| fact | one label and its value on a plate, such as a network's signal | | a drawer's `facts` |
| preview | a picture or a file's first lines, right under the name on its plate | | a drawer's `preview`, `FileFacts.js` |
| launcher | the window on Super+Space that finds programs, windows, actions and answers | palette, command palette | `Launcher.qml` |
| place | a list the launcher opens, belonging to one context, such as Agents or Network | category, collection | |
| row | one thing in a place | | |
| home | the launcher's first place: Now, Agents, Programs, People, Passwords, Desktop, Machine, Network and Keyboards | | |
| crumbs | where Joel is in the launcher, from home | | |
| Recent | the group of the last things done with what holds it, as in a program, a person or Capture: newest first, each once, never one open or shown already | | `*:recent` |
| field | the line at the top of every place: the find line, or the command line in its place | | |
| back | going up one place, to the row he was on, with h, - or Backspace | | the `out` act |
| action | what a row can do: the same set for every row of its kind | verb | |
| go to | the action moving to something open on the desktop, a window or a tab such as a session's, and nothing else; a row named Go To with where at its side | | |
| show | the action opening a place or a program on one thing in it, such as Show in Bluetooth or Show in Calendar, and the one word for looking at what a row holds | see | |
| open | the action starting something, such as a program or a file | | |
| turn on | the action starting a setting, a radio or a light | | |
| turn off | the action stopping a setting, a radio or a light | | |
| forget | the action taking an item out of its list, such as a notice, a copy or what's waiting | let it go | |
| foot | the status line at the bottom of the launcher: the mode, the glance and what the keys do | | `foot` |
| Now | the launcher's view of what's on and waiting, on Super+O | What's on | `Now.qml` |
| For You | the one place for what needs Joel, with where each came from: Now's first row, weighed, the most pressing first | | `ForYou.js` |
| to-do | what only Joel can do, raised by the session that needs him and listed in For You | Needs You item, waiting on you | `WaitingOnYou.qml` |
| to-do daemon | what finds a to-do done or ready by itself, running its check and hearing its signal | the desktop's daemon | `modules/to-do`, `verdicts.json` |
| check | the command that settles a to-do, which succeeds once it's done, apart from the checks core.nix holds | | `doneWhen.check` |
| agents asking | For You's row gathering the sessions waiting on Joel, once two are, the most pressing first | | `for-you:kind:agent` |
| look | something a session points Joel at to see, a pull request, a page, a file, a diff or a terminal, kept in For You until he opens or clears it | | `on:waiting:look`, the looks folder |
| asker | who a look comes from, in plain words: what its session writes, else its quest's name without its id, said in its row's note and its drawer's eyebrow | the quest's title, its id | a look's `asker` |
| ask | what a look asks of Joel, at its row's side, with its choices first in its step, each answered by one Return once Joel has stepped in; Return on the row opens it and never answers | | a look's `ask` |
| if you do nothing | what happens if Joel leaves a look as it is: what its session writes, else that it expires into History at its time where it has one, or goes quiet into History three days after it came | | a look's `ifNothing` |
| to look at | a look For You holds in its row of them, one that neither blocks a session nor is due | | `for-you:kind:look` |
| to try | a landing whose line asks Joel to try something, kept in For You until he opens or clears it, or a day from when it landed | | |
| to read | a look that asks nothing of Joel, only to be seen, kept in For You's last row 24 hours and then put in History as expired | | `for-you:kind:read` |
| Lately | what came for Joel that asks nothing of him, one row for each place it came from, among Now's own rows after the moment's; a row stands in For You while one of its own notices is critical, unread and off the screen, so Mail's never does, and stays there until the launcher next opens unless it comes to hold nothing | | |
| What Moved | Now's row, after the location, for each workstream that moved since Joel last looked, one row each named by its quest: what landed, else that a date moved, else what it took on, and the next date it owes; it asks nothing of him, so it's never in For You, and leaving the launcher after stepping in counts what it showed as he stepped in looked at | what's new | `on:moved`, the quests daemon's `moved:quests` |
| clipboard | what was copied last, and what was copied before it | | |
| selected text | what Joel has selected in any window, Wayland's primary selection, read only when asked and never kept | | `selection:primary` |
| On the Selection | the place on Super+X: the selected text at its head, and what an agent can do with it as pi-verbs serves them, Ask last | | `OnSelection.qml`, `pi-verbs` |
| timer | a span Joel sets counting down, which rings as a notice when it ends, kept by its daemon so it rings with the launcher closed; Timers is the place holding them | countdown, alarm | `HeldTimers.qml`, the `timers` program |
| stopwatch | the one count up from when Joel starts it, which keeps its laps; it's listed last in Timers | | `HeldTimers.qml`, the `timers` program |
| lap | one stretch of the stopwatch, from its start or the lap before to when Joel marks it | | `Timers.qml` |
| pause | a timer or the stopwatch held where it is, until Joel resumes it, apart from a session's pause | | the `timers` program's `pause` |
| capture | taking a screenshot or a recording of the screen, or a photo or a video with the camera, and the place holding them | | |
| screenshot | a picture Joel takes of the screen | screen shot | |
| recording | a video Joel takes of the screen, of all of it, a window or a region | | |
| photo | a picture Joel takes with the camera | | |
| screenshot text | the words in a screenshot, read on the machine as it's taken, so it's found by them and they can be copied | OCR | `screen-text` |
| location | where the laptop is, its town and the zone it keeps, Now's row and the one place the weather and everything that depends on it read it from | | `Location.qml`, the `location` daemon's `here:laptop` (built from `whereabouts/`) |
| time server | a clock elsewhere that the machine sets its own time by, over the network, listed in Time's Servers | | chrony's sources, its `server` mode |
| time peer | a time server that sets its time by the machine's too, each keeping the other right | | chrony's `peer` mode |
| reference clock | a clock wired to the machine that it sets its time by, such as a GPS receiver | | chrony's `refclock` |
| here | where the weather is when no town is typed: the location, else the town set, else the clock's time zone's city | | the weather provider's `place` |
| conditions | what the weather is like this moment where it's asked for: its sky and every figure of it | | Open-Meteo's `current` |
| local time | the clock and weekday where the weather is asked for, when that isn't here | | the forecast's `current.time` |

## Machine

The computer and its parts, its power and its sleep. A Bluetooth device is
paired, connected, disconnected and forgotten in Bluetooth, and used in its
kind's place, such as a headset's volume in Sound; a setting can be set,
raised and lowered; the machine can sleep, lock, log out, reboot and power
off.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| machine | one computer | host, computer | `nixosConfigurations.laptop`; `hostName`, which NixOS requires |
| firmware | the program in a chip that runs it: the machine's, which starts it and keeps its settings, or a device's, such as a keyboard's or a sound card's | BIOS, UEFI | |
| revive | sending a Mac that won't start Apple's firmware again from another machine over USB-C, keeping its files; a restore does the same and erases it | DFU restore | `idevicerestore`, without and with `--erase` |
| processor | what runs every program | CPU | |
| core | one of the processor's parts that each run a program's work at once, beside the others | | Linux's `cpu{n}` |
| performance core | a core built to run one program's work as fast as it can | P-core | Linux's `cpu_core` |
| efficient core | a core built to do more work for its power than a performance core, at a lower clock speed | E-core | Linux's `cpu_atom` |
| clock speed | how many cycles a second the processor runs at now, which it raises and lowers with its work | | `scaling_cur_freq` |
| cache | memory inside the processor holding what it used last, nearer to it than the machine's memory, in levels one to three | | `L{level}` caches |
| model | a device's maker's name for what it is, such as a processor's, a disk's, a battery's or a keyboard's | | |
| version | which release of Linux or of a device's firmware is in it, or which Bluetooth or TLS a link speaks, as its makers number them | | fwupd's `version` |
| Linux name | the name Linux gives a part of the machine, such as a sensor's chip or a battery | kernel name | hwmon's `name`, power_supply's name |
| graphics | the processor's part that draws what the screens show | | the GPU, Xe |
| NPU | the processor's part that runs models on the machine | | the `intel_vpu` driver |
| media engine | the processor's part that encodes and decodes video | | |
| display engine | the processor's part that sends each screen its picture | | |
| chipset | the chip beside the processor that reaches the disk, USB and the rest | | the PCH |
| power controller | the processor's part that sets its power and clocks | | |
| held back | the processor kept below the speed it asked for, by a power or heat limit, and for how long, apart from what an update or a sleep is blocked by | | `HeldBack` |
| blocked by | what stops an act now: a device's firmware update, such as low power or the lid shut, or the machine's deepest sleep | | fwupd's `problems`, the sleep's blocking device |
| security level | how fwupd rates the machine's firmware's defences, from 0 to 5, a ! when something running lowers it | | fwupd's HSI |
| read | one figure a sensor gave at a moment, of those a spark draws, as heat's Last Reads are, apart from the disk's Data Read | | `Seen.js`'s samples |
| throttled | the processor slowing itself to cool down, counted each time | | the kernel's `throttle_count` |
| kept from resting | a part keeping the processor out of its deepest rest while the machine idles, and for how long | | the telemetry's `kept` |
| memory | what programs run in while the machine is on | RAM | |
| swap | memory set aside out of the way, compressed first, then in the swap file | swapfile | zswap |
| memory pressure | how long programs are waiting for memory | | PSI |
| memory standstill | how much of the busy processors' time stood still because every program on them was waiting for memory | | PSI's `full` |
| processor pressure | how long programs are waiting for the processor | | PSI's `cpu` |
| disk pressure | how long programs are waiting for the disk, not how full it is | | PSI's `io` |
| lag | how long the desktop takes to answer a key or a click, and what made it wait | slow, jank, stutter | `lag-watch` |
| trace | one thing a program did, followed through its steps, each a span with a start and an end | | OpenTelemetry's traces |
| collector | the program keeping the machine's traces, to read any moment back | observability backend | `collector` |
| last seconds | what the machine did each second of a boot's last ten minutes, kept on the disk for after a reset | | `last-seconds` |
| recorder | the program writing a boot's last seconds, from its start until it powers off | | `last-seconds-keep` |
| ring | a file of a fixed size whose newest entry is written over its oldest, as each boot's last seconds are kept | | last-seconds' `ring` |
| cargo's builds | the filesystem of cargo's own, an image on the system's disk, where every account's builds keep their intermediates | build scratch | `cargo-builds.service`, `jitsusama.cargo.builds` |
| compile cache | sccache's store of compiled crates, keyed by their inputs, which every worktree's cargo build reads before compiling one | build cache | `sccache`, `~/.cache/sccache` |
| job | work Joel or an agent starts, run below the desktop so it gives way to it | | `jobs.slice` |
| kind of work | what a job is, which says how much the machine favours it: what Joel waits on at a terminal, timed work or agents' work | job class | the slices `jobs-awaited`, `jobs-timed` and `jobs-work` |
| shared work | a job other sessions wait on, such as a push holding one of a repository's walk slots, run beside jobs and ahead of it on the processor | | `shared.slice`, `SHARED_SLICE` |
| backlog | the requests waiting on a disk at once, on average and at most, apart from the machine's queue | | the disk's queue depth |
| speed | how fast a disk reads and writes now, in data and in requests a second, as the network's speed is for the connection | | the disk's `stat` |
| queue | heavy work, such as builds and VM tests, waiting its turn on the machine | machine queue | |
| warm | of an offload builder for VM tests, holding what they need, having built it as it stood; a fresh one is cold | primed | `there::warm`, `--warm` |
| offload builder | a builder elsewhere lent to the machine while its queue has builds one could take, and given back once idle | remote builder | `nix-offload` |
| gate | a commit's changed checks run on a machine that keeps the pass a merge reads; a Mac, with no way to the builders, asks another machine for one through mycelium | gate run | `nix-offload ask`, `nix-offload answer` |
| key gate | the places whose processes the TPM agent holding Joel's keys refuses, the agents' slices among them, read again at each connection; the agent's config and journal call it its gate, which never means a gate run there | | `modules/ssh-tpm-agent/gate.conf`, `SSH_TPM_AGENT_GATE` |
| elsewhere | the machine of the realm mycelium places an agent's heavy job of a kind opted in on, when it isn't this one, given the job's tracked files through the quest's handover and running it by mycelium | | the `elsewhere` program, `jitsusama.elsewhere` |
| slice | a share of the machine's processor, memory and disk that a group of programs runs in | | a systemd `.slice` |
| agents' slice | the slice agents' sessions run in, beside Joel's programs rather than among them | | `agents.slice` |
| power profile | how hard the processor and firmware run: performance, balanced or power saver | thermal mode, platform profile | power-profiles-daemon |
| chosen | of a setting's choices, the one it's at, whoever picked it: Joel, the charger or a profile | | |
| in use | held by a program, as a camera Chrome has open or a microphone in a call | | |
| charger | the power cable, and what it's plugged into | mains | AC |
| Keep Awake | the setting that keeps the machine from sleeping or locking while it's on | Caffeinate | |
| sleep | the machine paused with its memory kept, until it wakes | suspend | `systemctl suspend`, s2idle |
| wake | coming back from sleep, as the machine does, from a park, as a session does, or from standby, as a television does when net wakes it | | `net devices NAME wake` |
| idle | of the machine, every session left untouched past the screens going off, and nothing holding it awake | inactive | logind's `IdleHint` |
| idle sleep | the sleep a laptop falls into left idle on battery, never on the charger | autosuspend | `idle-sleep`, logind's `IdleAction` |
| hibernate | the machine off with its memory written to the swap file, until it starts again from it | | `systemctl hibernate` |
| log out | ending the desktop, back to the greeter | sign out | `session:` |
| reboot | starting the machine again | restart | |
| power off | turning the machine off | shut down | |
| lid | the laptop's lid, whose closing puts it to sleep away from a desk | | `lid-docked`, `lid-hinge` |
| hinge | the lid's angle from the keyboard, as a sensor in the laptop reads it | | IIO's `hinge` |
| docked | the laptop at a desk with a monitor plugged in, kept awake with its lid shut | on a dock, at a desk | |
| Bluetooth | the radio that reaches headphones, keyboards and mice, and the place listing every device it reaches, to pair and connect | | BlueZ |
| Nearby | the Bluetooth devices heard but not paired | | |
| Sound | the place holding what the machine plays through and hears with, built in or plugged in, and what each program plays and records | | `Sound.qml`, PipeWire |
| volume | how loud a device or a program plays, as a share of its full level | | |
| recording | programs taking sound from a microphone now, apart from the screen's recording | | PipeWire's capture streams |
| microphone | what the machine hears with: built in, a headset's or plugged in, chosen in Sound | | |
| hardware | a device's own chips and what they carry, as the kernel reports them: its card, its USB ID and its resolution | | ALSA's and USB's properties |
| card | the chip a sound device plays through or hears with, as the kernel lists it | | ALSA's card |
| connection | how a device is joined to the machine: USB and how fast it runs, or Bluetooth, apart from the network's connection | | the device's bus |
| profile | what a headset carries now: music one way, or both ways for a call | | Bluetooth's A2DP and HFP |
| level | how loud a sound device plays or hears at this moment, as its meter draws it, apart from its volume | | `sound:level` |
| channel | one of the separate signals a sound device carries at once, such as left and right | | `sound:channels` |
| graph | how the machine's sound joins each device to each program playing or recording, where a device's Graph ID says which it is and its Graph Format the audio it moves | | PipeWire's graph, its node id |
| format | how a device lays out the sound or picture it carries: a stream's rate and sample type, or a camera's encoding, such as MJPEG | | PipeWire's format, V4L2's pixel formats |
| stream | what one program plays through a sound device or records from one, each with a panel of its own in Sound | | a PipeWire stream node |
| role | what a stream is for, as its program says: music, a film, a call and the like | | PipeWire's `media.role` |
| process ID | the number Linux gives a running program, which says whose a stream is | | `application.process.id` |
| client ID | the number the machine's sound gives a program joined to it, one however many streams it plays | | PipeWire's `client.id` |
| camera board | the part in the lid holding the built-in camera, which Dell fits in more than one kind | camera module | `jitsusama.dellXps14.cameraBoard` |
| image processor | the part turning the built-in camera's raw picture into an image | ISP, IPU | |
| IR camera | the infrared camera beside the built-in camera, for signing in by face | | `ir-camera`, Howdy |
| desk light | the light on the screen, lit while a camera is in use | | Litra |
| brightness | how bright the desk light shines, apart from a screen's brightness | | Litra's `brightness` |
| console | the text screen the machine starts on, and the ones past Ctrl+Alt+F2 | | kmscon |
| splash | the picture shown while the machine starts and stops | boot screen | Plymouth |
| device check | the program that tells Joel a device stopped working | | `device-check`, smartd |
| VM | a computer run as a program on this one | virtual machine | libvirt, QEMU |
| Linux builder | a VM on a Mac that builds what Linux machines run, so the Mac's work builds there | | `darwinModules.linux-builder`, `darwin.linux-builder-vz` |
| container | a program run apart from the machine, with its own files | | Podman |
| update | the machine switched to its repository's main, signed, by root's own choosing when asked | upgrade, self-update | `update-to-main` |
| quest backup | the agents' quests pushed to a repository of their own each hour, by a service core.nix doesn't run; `system` says it worth a look once nothing has backed up for over 2 hours, or when its time file gives no time it can count from, and bad from 7 whole hours | | `quest-backup`, its time file `landed` |

## Network

How the machine reaches out: Wi-Fi, Ethernet, a VPN and tethering, and the
home network it reaches out through at home. A network can be connected,
signed in to and forgotten; Wi-Fi's radio can be turned off.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| connection | the network link out of the machine | line, uplink | |
| speed | how fast the connection, or a device on the home network, carries data now, each way | network speed, connection speed, bandwidth | `traffic` |
| in | of data at this machine's own links, coming to it, as its traffic and a link's speed count it; the internet's speed says down | | `traffic:in`, `wifi:in` |
| out | of data at this machine's own links, leaving it, as its traffic and a link's speed count it; the internet's speed says up | | `traffic:out`, `wifi:out` |
| down | of the internet's speed, what it carries to the home network, as a speed test measures it; this machine's links say in | | `eero:internet:down` |
| up | of the internet's speed, what it carries out of the home network, as a speed test measures it; this machine's links say out | | `eero:internet:up` |
| address | an address on a network: one the machine has, or the one a node, a device, a server or another machine is reached at | network address, IP | `networks` |
| hardware address | the fixed address a network card or radio carries from where it was made, apart from the addresses a network gives it | | MAC addresses, a BSSID |
| lease | an address a network's server lends for a while, to the machine or to the gateway node, with the name servers and domain that come with it | | DHCP, `leases` |
| subnet | the run of addresses one of the home network's networks hands out, the main one or the guest one, as its address and prefix | | `subnets_config`, `Subnet` |
| reserved address | an address the home network keeps for one device, so it has the same one each time it joins | static IP | `reservations`, `Reservation` |
| neighbour | what a cable's other end says it is, such as a switch, and the port of it the machine is plugged into | | LLDP, `neighbours` |
| Wi-Fi | the network without a cable | WLAN, wireless | `wifid`, `wifi` |
| Ethernet | the network through a cable | wired | |
| tethering | reaching out through a phone's connection, over its cable | | |
| sign-in page | the page a network shows before it lets the machine out, as a hotel's does | captive portal, network sign-in page | `wifid-sign-in` |
| VPN | a private link to another network, carried through this one | tunnel | `jitsusama.shaper.vpns` |
| tunnel mode | how WARP carries the machine's traffic, as warp-cli names it: all of it through WARP, DNS alone, or as a proxy | | warp-cli's `mode`, `warp:settings:mode` |
| WARP | the VPN warp-cli runs, which carries the work network | | `warp`, `warp-cli` |
| work network | the place in Network saying how the machine's link to work stands, through WARP | | `warp` |
| firewall | what keeps others' connections out | | nftables |
| shaper | the program that keeps the network's queue on the machine, sorted by slice, so a call stays quick | | `shaper`, CAKE |
| house | of the shaper, another machine on the same line out, which says each second what it sends up; this machine keeps the most the two may send between them, and holds its own upload under what the other leaves of it | | `jitsusama.shaper.networks.<router>.house`, `house.rs` |
| home network | the network at home, the eero's, its nodes and the devices on them, and its place in the launcher | eero network, mesh network | `eero` |
| node | one eero of the home network | satellite, extender | `eeros` |
| gateway node | the node the internet comes in through | base station, primary node | `gateway` |
| backhaul | how a node reaches the node before it, over Ethernet or Wi-Fi | mesh link, backbone | |
| device | one box on the home network, whatever it does, listed once with each role it plays | station | `clients`, net's `devices` |
| role | what a device does on the home network: modem, router, access point, switch, receiver, printer, television or client, one box playing several, as one eero is a router and an access point | | net's `roles` |
| modem | the device that turns the internet provider's line into Ethernet, with its channels and event log | cable modem | net's `modem` |
| router | the device that hands out the home network's addresses and carries its traffic out to the internet | | |
| access point | the device whose radios devices join over Wi-Fi; each node is one | AP | `access-point` |
| switch | the device that joins cables to the home network, port by port | | net's `switch` |
| receiver | a device that plays what's sent to it over the home network into speakers, as an AV receiver does, with its volume and what it's playing | stereo, AV receiver | net's `receiver` |
| printer | a device that prints what's sent to it over the home network, with its ink and what it's doing | | net's `printer` |
| television | a device that shows what it's sent on a screen of its own, with whether it's on or in standby | TV, smart TV | net's `television` |
| client | a device that only joins the home network, as a laptop or a phone does | | `client` |
| port | a socket on a device that a cable plugs into, with its link and speed | | net's `ports` |
| link | of the home network, a cable or a radio joining two devices, port to port or radio to radio; the home network's map is drawn from them | | net's `links` |
| source | of the home network, a vendor's way of saying what its devices are: eero's record, the modem's status pages, the switch's web page; all it says is kept as its detail | | net's `sources`, `detail` |
| internet | the home network's way out, through its gateway node, with its speed each way as last measured | WAN, ISP connection | net's `internet` |
| internet provider | who carries the internet to the home network, as eero names it | carrier | `isp` |
| path | a device's way to the internet: the devices its traffic crosses, in order, up to the modem | | net's `path` |
| room | how much a link or the internet can still carry: a cable's speed, or a radio link's less the time its radio is busy | | net's `room_mbps` |
| chokepoint | the hop on a path with least room, one each way | bottleneck | net's `down`, `up` |
| concern | something on the home network worth a look, worked out from its figures: a busy radio, a weak signal, a slow cable, a modem channel losing data, a device using most of the internet one way, or the internet as a chokepoint | | net's `concerns` |
| outage | a time the internet was down | downtime | |
| update | the home network's firmware installing itself, at the hour it's set to, apart from the machine's update | | the eero's `update_hour` |
| radio | a Wi-Fi or Bluetooth transmitter on one band: the machine's card's or adapter's, or one of a node's | | `bluetooth:powered` |
| band | the frequencies a radio uses: 2.4, 5 or 6 GHz | frequency band | |
| channel | the slice of a band a radio uses, with its width, such as Wi-Fi's or the ones Bluetooth hops between | | `bluetooth:channels` |
| stream | one of the signals a Wi-Fi link carries side by side at once, each adding to its speed | | `wifi:streams`, MIMO's spatial streams |
| signal | how strongly one radio hears another, in dBm | RSSI, signal strength | |
| rate | how fast a radio link can carry data each way as it's set now, and how it's made: MCS, streams, width and guard interval | link speed, PHY rate, bitrate | `rate_bps` |
| speed test | eero measuring the most the internet carries each way, from the gateway node | speedtest | `speedtest` |
| traffic | how much data the machine's link, the home network, a node, a profile or a device moved | data usage | `usage`, `data_usage`, `wifi:carried` |
| read | one figure taken of a link at a moment, of those a spark draws, as WARP's Reads count and the internet's Last Read says when, apart from mail read | | `WarpSeen.js`'s samples |
| profile | a group of devices eero lets through or holds off together | | `profiles` |
| light | a node's status light, on or off and how bright | | eero's `led_on`, `led_brightness` |
| mode | how the gateway node gets its address: lent by the internet provider's server, set by hand, or through a sign-in | | eero's `lease.mode` |

## Disk

The disk, its filesystems, and the guard that keeps them from filling, whose
place is inside Machine. A filesystem can say what's filling it, be cleared
now, and keep or stop keeping a folder.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| disk | the machine's storage | drive, SSD, storage | NVMe; `jitsusama.disk`, from disko |
| spare | the share of a disk's reserve blocks left, which it puts in place of worn ones | | SMART's `avail_spare` |
| boot partition | the disk's small first part, which the firmware starts from | ESP | `/boot` |
| filesystem | how a disk's room is kept as files | | Btrfs, a subvolume |
| snapshot | a filesystem's copy kept from a moment, which the guard may clear | | a Btrfs snapshot |
| disk guard | the program that keeps each filesystem from filling: one for the machine, one for the account | | `disk-guard system`, `disk-guard user` |
| freeze | the disk guard stopping a program that fills a filesystem, until there's room | frozen | `SIGSTOP` |
| thaw | letting a frozen program carry on | | `disk-guard thaw` |
| filling | the disk guard's first band, under a quarter free, when it starts taking room back | pressure | `filling` |
| nearly full | the disk guard's second band, under a tenth free, when it takes more | urgent | `nearly-full`, a config's `nearly_full` |
| critically full | the disk guard's last band, under 4% free, when it may freeze what fills the disk | critical | `critically-full`, a config's `critically_full` |
| forecast | how long a filesystem has until it's full, at the rate it's filling, or while above Nix's floor until it reaches that | | |
| Nix's floor | the free space Nix keeps on its store's filesystem by collecting its own garbage | | Nix's `min-free`, a status's `nix_floor` and `floor_at` |
| holder | a folder the disk guard names as taking a filesystem's room, with whose it is | | `holders` |
| ledger | the disk guard's record of what held each filesystem over two days, so a holder says how much it grew | history | `ledger.json`, a holder's `grew` |
| keep | Joel telling the disk guard never to clear a folder, with his reason | pin, protect, exclude | `disk-guard keeps` |
| clear | the disk guard removing builds, caches and waste it may, as a band allows or at once when Joel asks | purge, wipe | `disk-guard clear` |
| footprint | what files take on a filesystem, which compressed can be far less than their size | | `disk-guard footprint`, `Request::Footprint` |

## Keyboards

The keyboards plugged in, their keys, layers and light. A keyboard can switch
layer, lock a layer on and release it, and set its light; its keys and
shortcuts are listed inside it.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| keyboard | one keyboard, built in or plugged in | | |
| key | one key on a keyboard | | |
| layer | what a keyboard's keys do, one set of several, which a key switches between | Keys and layers | the ErgoDox's layers, `Layer.qml` |
| map | the keyboard's layout, shown on screen | | `KeyboardMap.qml` |
| release | letting go of a layer locked on, back to the one beneath | | |
| light | the keyboard's own light | backlight | |
| chord | the keys pressed to reach something, drawn key by key, such as Super + O or Ctrl + W l | | |
| shortcut | a chord and the one thing it does, as Shortcuts lists them | binding, bind | niri's `binds` |
| key language | the one set of keys every surface of ours speaks, Vim's as Joel's Neovim has it | | `modules/keys/language.toml` |
| act | one thing the key language's keys do, named once, as down or close | | `acts.<name>` |
| surface | a program of ours keys act on, of one or more kinds: field, list, page or player | | `jitsusama.surfaces` |
| mode | what keys do at a moment, each with one purpose: normal, the find line, the command line, insert, visual or an operator waiting | | `modes` |
| normal | the mode standing on a thing and acting on it, where nothing types | | |
| find line | the / line that narrows a list as it's typed | | the `find` mode |
| search | a page's /, moving to a match and hiding nothing | | |
| command line | the : line, where a command is named | | the `command` mode |
| menu | what the command line lists: every command the cursor row, the selection or the surface takes | | |
| insert | the mode writing a prompt's words | | |
| prompt | a line of words written to be handed over: a reply, a new name, a save name | | the `prompt` kind |
| secret | a one-line password, with no modes | | the `secret` kind |
| cursor row | the one row normal stands on | | |
| visual | the mode choosing several things | | |
| selection | what visual has chosen | | |
| gathered | files collected across folders for one answer: Neovim's with :arga, or the file chooser's marked | | the arglist, the file chooser's `marked` |
| done | a notice forgotten, marked read first where its sender offers it | | |
| leader | Space, starting a sequence of keys in normal and visual | | `mapleader` |
| mark | a place kept under a letter, set with m and gone back to with ' | | |

## Security

What proves it's Joel or his agents, keeps his passwords, keeps the disk
sealed, and installs or recovers a machine from a stick. Its one launcher
place is Passwords; each of its other things shows where it's used. An
entry can be copied from, opened at its site and filled in.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| TPM | the chip that keeps the machine's keys and seals its disk | security chip | |
| firmware password | the password that keeps the firmware's settings shut | BIOS password | |
| login key | the SSH key that signs Joel in to other machines, kept in the TPM | everyday SSH key | `id_ecdsa.tpm`, ssh-tpm-agent |
| spare | the login key's copy on the YubiKey, for when the TPM can't be used | | `<account>@yubikey` |
| Joel's key | the key that signs Joel's own commits | Joel's signing key | `joel-signing`, `joelKey` |
| agents' key | the key that signs agents' commits | agents' signing key, agent's key | `agent-signing`, `agentKey` |
| passkey | a key the TPM holds for signing in to websites | FIDO security key | `linux-id` |
| keyring | where passwords programs keep are held, opened at sign-in | | gnome-keyring |
| Passwords | the place holding Joel's entries from every password manager | | |
| password manager | a program keeping Joel's entries, such as 1Password, each one an option | | |
| entry | one thing a password manager keeps, called by its kind: a login, a card, a note, an identity or a key | | 1Password's items |
| favourite | an entry Joel starred in his password manager, apart from People's favourite | | 1Password's favourites |
| detail | one part of an entry, as its username, its password or its one-time code | | 1Password's fields |
| one-time code | the code an entry gives for signing in, new every half-minute | OTP, TOTP | |
| hidden | of a part of an entry, shown only when asked, as a password is | concealed | 1Password's `CONCEALED` |
| audit | every entry's password checked for reuse and breaches, only when Joel asks, as Check Every Password does | | the daemon's `audit` |
| greeter | what asks Joel to sign in when the machine starts | | greetd, tuigreet |
| lock screen | what hides the desktop while Joel is away, until he signs in again | | swayidle |
| stick | the USB stick the guide writes for one machine, to install or reinstall it | reinstall stick | |
| repair stick | a stick written for no machine, to repair any | | |
| writer | the machine writing a stick for another, which the guide shows beside the machine it's for | | `guide host` |

## The Library

The words core.nix is built from. Never a launcher place.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| core | core.nix, the library every machine is built from, and the commit of it a machine runs | | the `core` input |
| machine repository | a repository of machines built on core.nix, adding its own modules | | any flake with core.nix as an input |
| module | one program or subsystem, configured | | `modules/bat/` becomes `homeModules.bat` |
| kernel module | a part of the kernel loaded on its own, always written whole | | `boot.kernelModules` |
| role | what a machine is for | | `roles/workstation/` becomes `nixosModules.workstation` |
| hardware | one machine model, and what it needs | | `nixosModules.dell-xps-14-da14260` |
| identity | who Joel is on a machine | | `jitsusama.identity.email` |
| age key | the file that opens what was sealed to it with age | age identity | |
| theme | the look every program Joel sees shares | | `jitsusama.theme` |
| terminal kit | the library every Rust command builds on for its shared flags, ink, layout, help, errors and exit codes | | `modules/terminal-kit/` |
| crate reach | which of core's Rust crates read which, by their manifests' paths, so merge-net and a gate's replay judge alike what main moved beside a change | | `modules/crate-reach/` |
| colour | what a colour means in the theme, such as the accent, apart from a screen's colours | colour role | `jitsusama.theme.colours`, a theme file's `[colours]` |
| face | one of the theme's typefaces, named by its kind: mono, sans, serif or script | typeface | `jitsusama.theme.font` |
| serif face | the theme's serif: italic, the desktop's voice, what the launcher says to Joel; upright, the words to read beside a mono name | voice face | `jitsusama.theme.font.serif`, `Theme.serif` |
| serif text | the serif face upright at large, cut into a font of its own, for a program that can't set a variable font's axes | | `jitsusama.theme.font.serif.text`, `modules/fonts/serif-text.nix` |
| sans face | the theme's sans-serif, only for other people's programs that draw their own interface | proportional face | `jitsusama.theme.font.sans` |
| step | one size on the theme's type scale, by its power of the ratio, such as large | | `jitsusama.theme.type` |
| level | one rank of text on the desktop: its face, step, weight, case and colour together, such as Name or Note | style, text style | `jitsusama.theme.type.levels`, `Theme.level` |
| pitch | how far apart two lines of one thing at a level sit, baseline to baseline | | `jitsusama.theme.type.levels.<level>.pitch`, `Theme.level.<level>.pitch` |
| rung | the space one level of structure sits apart by, wider than the rung inside it, such as items or frames | | `jitsusama.theme.space.rung`, `Theme.space.rung` |
| beat | half the large pitch, which every space between blocks down a surface is a whole number of | | `jitsusama.theme.space.beat`, `Theme.space.beat` |
| script face | Noto, script by script, for the letters the other faces lack, with a cut a script beside the serif and mono faces | | `jitsusama.theme.font.script` |
| account | the account on a machine that home-manager configures | | `home-manager.users.<account>` |

## Checks

How core.nix proves what it promises. A check's name is a sentence saying what
it holds, subject first, with no prefix but `vm-`, which marks a VM test and
nothing else; its description in `check-descriptions.nix` finishes the
sentence. It may name the program its module is named after, since that
says which module broke.

| Word | Means | Not | In code |
| ---- | ----- | --- | ------- |
| check | a check that `nix flake check` builds, one promise held | | `checks.x86_64-linux.<name>` |
| VM test | a check that boots machines in VMs | | a `vm-` name |
| subtest | one of a VM test's pytest tests, as the driver's log names it, which can run alone | | `test_` functions in a `*-vm-test.py` |
| example machine | a whole machine built only from core's outputs, as a machine repository would | | `example-nixos`, `example-laptop`, `example-apple-laptop`, `example-darwin` |
| bare machine | the least machine of a class that evaluates, which a check adds one module to, for checks only | | `lib.bareNixosModule`, `lib.bareDarwinModule`, `bareNixos` |
| stand-in | something standing in for a real program or device in a check | fake | |
| shot | a picture a VM test takes of one state of the screen | photograph | `screendump`, `shots-changed` |
| decision record | a document saying why the design is what it is | ADR | `docs/decisions/` |

## Still to Settle

Words that wait on one choice, and move into a table above once it's made.
Joel chose VM and VPN (2026-10-26). The mycelium's new words wait on the
design of what a cell is (Joel, 2026-11-15: "we need to build the correct
mental model of what cells are, what their identity and purpose are"), and
each keeps to fungal biology or plain words. Delegate and split were
settled by what each does to a cell's identity (Joel, 2026-11-15, of the
page that set it out: "i love this page and what it describes"):

- **Whether forage replaces quest**: a forage is read today from the quest
  its cell works on (Joel, 2026-11-15: "each cell should have a forage"),
  so quest stays the word for the work and its folder. If forage takes its
  place, quest joins forage's Not.
- **The tie between two cells**: link already means two machines' nodes
  joined, so it needs another word.
- **What a cell lives on**: host already names a machine, which is its word,
  and a pi host is the harness's process; the choice is whether machine
  stays the word.
- **An audit of a forage**: audit already means Security's record of what
  was done, so judging a forage's outcome and a cell's fitness to its
  purpose may need its own word.
- **fruiting body**: what a forage puts out when it's done, such as a merged
  change, waiting on whether it's a landing.
- **basin**: the mycelium map's mark for a pool of borrowed cloud
  machines, drawn as one bowl since none of them is drawn alone. Each is a
  machine cells run on, not a cell (the design page's owner, 2026-11-17),
  so the choice sits beside what a cell lives on: whether basin is the
  word for the pool, or stays in the map's legend alone.
- **knot**: the mycelium map's mark for a network nobody leads, shared by
  cells under two networks or more: a ring with spokes, empty in the
  middle, standing among its cells so they're the bridge between their
  networks. Once no network has a leader (the design page's owner,
  2026-11-17), every network may be drawn this way, so the choice sits
  beside the tie between two cells: whether knot is the word for a
  network's mark, or stays in the map's legend alone.

[design]: design.md
