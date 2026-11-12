# How a Site Reads

The words on a site are half its design. Joel has turned down more sites for
their writing than for their look: "a lot of the text in the site is not well
written, is not easy to follow, and does not communicate well, directly against
my expressed requirements" (2026-11-06). What he asks for, every time, is text
"that is approachable, written for humans, flows well, and conveys well"
(2026-11-05). This file says how, for every page and every interface's words.
The prose standard (Canadian English, no emdashes, plain punctuation) holds
throughout.

## Know the Reader

- **Name who reads it before writing a word**: what they already know, what
  they've never seen, how long they'll give it. Most of his readers are smart
  engineers who know the field and weren't in the room. "we don't want to assume
  they have context that we have not shared" (2026-09-10).
- **Bring them in from what they feel.** Open with what hurts or what changes
  for them, in their terms, then why, then the idea. A page that opens with the
  inside of the machine has lost them by its second sentence.
- **He reads as a systems thinker.** "your way of describing this still
  sucks... I'm a systems thinker" (2026-09-29): show how the parts connect and
  what moves what, not a list of features.
- **Humans and agents are equal readers.** The same facts are on the page for
  people and beside it for agents, and words written for an agent (its
  numbering, its codes) never reach a person.
- **A page sent for Joel to read is written for him** (2026-11-11: "anything
  that is sent for me to look at needs to be written for me"): what it means
  for him first, an example from his own machine, no citations or codes in
  the running text, and the agents' record one link away. docs/design.md's
  Written for Joel holds every session to it.

## Short and Plain, Together

Clear doesn't mean long. A request for clearer words once turned a site's
titles into sentences and its lede into paragraphs: "fuck, the site went into
novel mode and lost its pithy title. this is not what i meant" (2026-11-06).

- **The title is pithy**: three to six words, sharp, never a sentence without
  its full stop.
- **The lede is one or two short sentences, under 40 words**, saying what hurts
  and the idea. A reader who stops after the first screen has the whole idea.
- **Each section opens with a short headline that says its point**, then two or
  three short sentences. More goes in a fold for those who want it. Reading the
  headlines alone gives the argument.
- **One idea a sentence.** Colons, semicolons and stacked clauses that pack
  three facts into a line get split.
- **Cut, don't add.** A rewrite longer than what it replaced is probably wrong.
  Figures carry the explanation; words caption them.
- **Every term is explained in plain words where it first appears, or isn't
  used.** A domain word with character may stay when its meaning is a hover,
  focus or tap away.

## Say What's True Now

- **The present, not the history.** The page says where things stand, never
  how the page or the work got there: "I don't want the site to be a history
  lesson in making the site" (2026-09-03); "i don't want it to be a chronicle
  of historic debate btw, but a picture in what we currently think"
  (2026-11-06). Superseded ideas leave the page; a changelog holds the past.
- **Every number has its meaning first, then the number, then what it's
  compared with.** "Almost no two writes at once touch the same thing, so
  nothing makes them wait" before "96% of writes...". A number with no
  comparison is noise; a headline number links to what it counts.
- **Honest about what's unmeasured, early or broken.** Say what it costs and
  what it won't fix. A section called "What It Costs" earns more trust than
  any claim.
- **Every part agrees with every other.** One file of facts feeds every
  section, so a status changed in one place changes everywhere: "parts of it
  are in different states of agreement with the rest" (2026-11-06) is a
  fault in the facts, not the prose.
- **Accuracy is part of the design.** A stale date, a wrong status or an
  unchecked claim undoes the rest. Check each against its source before every
  publish.

## A Proposal States Its Case

- **"clearly articulate the desired outcome, summarize the recommendations ...
  build a case"** (2026-09-30). It's "completely self contained" and "a
  proposal, not a ask" (2026-09-15).
- **Never pose the owner's decisions as questions.** Say what was decided and
  why; leave as open only what is open.
- **Data supports the story; it doesn't lead it.** "this needs to be readable,
  not a fact dump. we need supporting data, but the data is not the story
  itself" (2026-09-15).
- **Who thinks what, quoted fairly and short.** A quotation that's trimmed
  shows the cut with an ellipsis or brackets.

## Nothing the Reader Can't Open

- **No private references**: no internal ids, task or lane names, file names on
  someone's disk, scratch paths or unpushed branches. "the site naming my
  quests and files on my disk will mean nothing to a reader, we need to have the
  site communicate with readers not confuse them" (2026-11-06).
- **Say what the evidence is, in words**: "production metrics, 4 November", "a
  load test on a full-size copy", "a formal model, 80 cases checked". Sources go
  in a quiet line under the part, never in the sentence.
- **Link only where a reader can follow.** For deeper detail, publish a clean
  write-up on the site and link that.
- **People by name, agents never.** Credit real people for their work, without
  timestamps. When the site speaks for Joel, it says "I": "you and me are I,
  not we" (2026-11-07).
- **No agent tics**: no numbered stream labels, letter suffixes ("6a"), shouted
  codes, invented jargon or the agents' own metaphors. They earn the verdict
  "it seems written by AI that doesn't like humans to be able to understand
  shit. it doesn't use the proper domain language" (2026-10-01).
- **A phrase readers misread is retired**, and a guard that fails the publish
  keeps it from coming back.

## Words in an Interface

- **Every label, empty state, error and prompt is written as carefully as the
  page's prose**, and so is every word an AI writes into it: "human and AI agent
  empathy needs to be a critical tenet" (2026-11-07).
- **Actions and headings are Title Case; messages are sentences with their
  full stop.**
- **One name for each thing everywhere**, the same word in the list, the
  drawer, the palette and the docs. No synonyms.
- **An error says what happened and what to do**, in words people will paste
  into a search or a chat.
- **Nothing nags.** No prompt that waits on a person to visit, no "don't
  forget", no badge pleading for attention.

## Notices and Change Lines

- **A change line is one short human sentence of what we now think**, never
  what an agent did.
- **Readers are told only when the picture moved**, not for a typo.
- **The changelog is by day, newest first**: a terse overview of each day, its
  entries folded.

## Checking the Words

Before each publish, give each changed section to three cold readers who
weren't there: one who knows the field, one who doesn't, and one skimming in a
hurry. Each says in one sentence what the section means and marks every place
they stumbled. Rewrite until all three get it right on first read. Readers find
words; they don't find space, rhythm or craft, so this never replaces looking.
