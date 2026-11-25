// The shapes and words by which Presented.js's keyShown, kindsLed, appShown and actShown tell how
// much of a row's key a journal line may say with reports off: what the launcher and its daemons
// write there, as against what someone else did. Data alone, kept apart from the code that reads
// it, so Presented.js stays one file biome can hold to its length.
.pragma library

// The kinds of key the launcher gives its own rows, whose rest, where it has an id's shape
// (ownId), says nothing a person or another program wrote, so it's said whole: a collection's is
// the name its place has in the Nix config.
const keysOwn = ["place:", "category:", "collection:"];

// How many letters of such a key are said with reports off, so even an id can't run a line on.
const ownLength = 64;
// What follows an own kind as the launcher writes it, an id its Nix config names, as `emoji`
// or `Asks`: a daemon's row keyed so may hold a name, a line break or a walk's forged steps
// there, which no id holds, so such a key is said by its kind alone. A served row is never
// keyed so (ReadyRows.js's keyMade), so this holds only for another way a row is keyed.
const ownId = /^[A-Za-z0-9][A-Za-z0-9._-]*$/u;

// A kind's word, as the launcher and its daemons write one, at most 32 letters, and the keys
// whose kind keyShown says, as actShown leaves them, since their later parts may be someone's
// words.
const kindWord = /^[a-z][a-z0-9-]{0,31}$/;

// The words a key may say after its kind with reports off: the acts the daemons and the launcher
// build their keys from, as `verb:agent:close`'s close or `displays:sleep`'s sleep, and what some
// act on, as `verb:recent:site:open`'s site. A part's shape can't tell an act from a lowercase
// name, a person's in `person:ada` or a hex id (PV3TE0's F12), so only these are said, and a new
// act is said by its kind alone until it's named here.
const actWords = new Set(`act add address afresh again ahead archive ask attachments audit awake
    back bluetooth branches call cancel changes clear close code copied copy copy-marked copy-name
    device download drawing edit end expect favourite file find folder follow-room forget further
    gather go guests here install join keep landed lap layer let-in link look louder low machine
    mail map mark markdown marked maybe message move mute muted network next night-light no offer
    on onward open orchestrate page password paste path pause picture play previous pull put-back
    quiet quieter quit read reopen replace reply repository reset run save search see seen set
    settle setup sharing show sign-in site slack sleep speedtest sqm-off sqm-on ssh start
    start-anywhere start-new-work starts stop switch take-back talk terminal timers title
    to-reopen transcript unlock unmark-all update username waiting wake wave website window write
    yes`.split(/\s+/));
// Keys actShown says only as keyShown does, whose second part may be an act's word and say more
// than keyShown would: a record's `row:network:<name>`, or a window keyed `window:mail` with no
// title. A `verb:` key needs no place here, since keyShown already keeps as much of it.
const keysSaidByKind = /^(row|window):/;

// A program's own id, as kitty, 1Password or org.gnome.Evolution. A browser's app window may build
// one from a page's address, with underscores for its slashes, so one that isn't shaped like this
// is said as no app at all, as is a window that names none: niri's records leave its app empty,
// and a key built from a window read past them would say null or undefined.
const appWord = /^[A-Za-z0-9][A-Za-z0-9.-]*$/;
const appsUnnamed = ["null", "undefined"];
