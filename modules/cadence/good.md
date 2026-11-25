# The Definition of Good

What good means for each measure the hour card judges: its target, the line of Joel's it comes
from, and what being over it costs him. On 14 November he asked for "something constantly
evaluating how we can ship more and faster while also delivering insane quality, both
functionally, and by maintaining clean zones", and on 11 November that "nothing can be out of
spec for this". A target with no reason behind it can push work he doesn't want or pass what he
does, so each one says why.

The page is built into cadence, and every hour holds the card to it. A measure on the card
that no section names, or whose target differs from the one written here, is counted in
`measures-undefined`, and its keeper is asked to write its line. Every target is a most: lower
is better for each.

Each section names its measures' keys, a key ending in `*` naming every key that opens the
same way. A target that opens with a number is the one the hour must hold the measure to; one
that opens with "none" means the measure is shown and never judged; one that opens with "set
where it's read" is taken from the measure's own source. Any other target is counted as none.
An item's later lines are indented under it with no blank line between, each field is said once,
and a key holds no space; any other line or item in a section, a field said again, or a key
holding a space, as one broken across two lines does, is counted too.

## Apps Under Load

- Measures: `apps:*`
- Target: set where it's read: each figure's own, as apps-under-load writes it in its hour.
- Joel, 19 Nov: "have everything be very quick to respond ... run insanely high quality video
  and audio while the machine is totally under all the pressures"
- Over costs him: a call that freezes, or a Slack, Chrome or launcher that lags under his
  hands while agents build. He ranked this above everything else.

## Apps Under Load Unread

- Measures: `apps-under-load`
- Target: 6 hours with no hour read.
- Joel, 19 Nov: "have everything be very quick to respond ... run insanely high quality video
  and audio while the machine is totally under all the pressures"
- Over costs him: nothing watches his apps, so a call or a window going slow under load goes
  unseen until he finds it himself.

## Wi-Fi Holding a Call's Video

- Measures: `wifi-video-held`
- Target: 2 minutes in the hour.
- Joel, 19 Nov: "run insanely high quality video and audio while the machine is totally under
  all the pressures"
- Over costs him: his call's video stutters or freezes over Wi-Fi while the people on it watch.

## Sessions Here

- Measures: `sessions-here`
- Target: none, shown and never judged.
- Joel, 23 Nov 17:52: "i want to distribute load until we have the cloud ready"
- Over costs him: nothing judged. The count shows him how much runs on the machine he works
  at, which slows everything he uses while the cloud isn't ready.

## Ask to First Slice

- Measures: `ask-to-first-slice`
- Target: 4 hours, the median from his ask to a first piece he can use.
- Joel, 11 Nov 16:4x: "i need for you to make faster progress. i don't understand why so much
  has not been completed."
- Over costs him: half a working day or more before anything he asked for reaches his hands.

## Ask to First Slice, Set by the List

- Measures: `ask-to-first-slice-derived`
- Target: none, shown beside his own asks and never judged.
- Joel, 11 Nov 16:4x: "i need for you to make faster progress. i don't understand why so much
  has not been completed."
- Over costs him: nothing judged. It shows whether the list's own work moves as fast as his.

## Merge to On

- Measures: `merge-to-on`
- Target: set where it's read: the median the tick's card names for it.
- Joel, 14 Nov: "ship more and faster"
- Over costs him: a fix merged but not yet on his machine, so what hurt still hurts.

## Commit to Merge

- Measures: `commit-to-merge`
- Target: set where it's read: the median the tick's card names for it.
- Joel, 14 Nov: "ship more and faster"
- Over costs him: finished work sits waiting on its gate instead of reaching him.

## Mycelium to Lock

- Measures: `mycelium-to-lock`
- Target: 60 minutes for the longest-waiting mycelium merge outside core's lock.
- Joel, 14 Nov: "ship more and faster"
- Over costs him: a mycelium fix merged that no machine runs, since none gets it until core's
  lock carries it.

## Landed

- Measures: `landed`
- Target: none, since an hour's count swings with the clock.
- Joel, 14 Nov: "ship more and faster"
- Over costs him: nothing judged. It shows how much reached the three mains in the hour.

## Council Rounds per Merge

- Measures: `rounds-per-merge`
- Target: 3, the median rounds the changes merged in the last day took.
- Joel, 23 Nov 11:15: "NO! fucking land this and fix forward, don't make perfect the enemy of
  the good."
- Over costs him: what he waits on is held for another review round, each one paid for.

## Rounds Past Five That Fixed Nothing

- Measures: `rounds-past-five-fixed-nothing:*`
- Target: 5 rounds on a change whose rounds after the fifth caught nothing.
- Joel, 23 Nov 11:15: "NO! fucking land this and fix forward, don't make perfect the enemy of
  the good."
- Over costs him: money and hours spent on rounds that changed nothing he would see.

## Balance Across Machines

- Measures: `balance`
- Target: none, shown and never judged.
- Joel, 22 Nov 15:2x: "WE NEED TO FOCUS ON SPREADING LOAD NOW! WE NEED TO QUICKEN MOVING
  STUFF INTO THE CLOUD!"
- Over costs him: nothing judged. It shows how each machine shared the hour's work.

## Crowded Machines

- Measures: `crowded`
- Target: 0 machines over their room while another has room.
- Joel, 21 Nov: "I need more machines ... i need for sessions to take less resources and i need
  more things to run them on"
- Over costs him: the machine under his hands slows while another sits with room to spare.

## A World Git Turn

- Measures: `queue:world-walks`
- Target: 60 seconds for the day's p90 wait.
- Joel, 20 Nov: "what is better than a queue is when work just works without one. when a queue
  is still needed, the best queue is self regulating and allows maximal flow."
- Over costs him: sessions held waiting on each other instead of working.

## A Build on This Machine

- Measures: `queue:builds`
- Target: 120 seconds for the day's p90 wait.
- Joel, 20 Nov: "what is better than a queue is when work just works without one. when a queue
  is still needed, the best queue is self regulating and allows maximal flow."
- Over costs him: every change waits for cores before it can be checked.

## A VM Run's Builder

- Measures: `queue:sends`
- Target: 120 seconds for the day's p90 wait.
- Joel, 20 Nov: "what is better than a queue is when work just works without one. when a queue
  is still needed, the best queue is self regulating and allows maximal flow."
- Over costs him: a change's VM tests wait for a lent builder rather than running.

## Core's Merge Train

- Measures: `queue:trains`
- Target: 1200 seconds for the day's p90 wait, from boarding to landed.
- Joel, 20 Nov: "what is better than a queue is when work just works without one. when a queue
  is still needed, the best queue is self regulating and allows maximal flow."
- Over costs him: a change ready to land rides for over 20 minutes before it does.

## Misses

- Measures: `misses`
- Target: 0 misses he found himself with no check yet running here to catch the next.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: what he ran into can come back, with nothing to say it has.

## Misses Naming No Test

- Measures: `misses-untested`
- Target: 0 misses naming no test that runs here.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: a check written as a sentence, which no machine can run, so nothing would
  catch the miss coming back.

## Misses Said None

- Measures: `misses-said-none`
- Target: none, since a pain heard where there was none is the listener's, and each shows a day
  with its owner's reason so the word that closed it is read.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: nothing judged. A real miss closed on a word nobody reads would go quiet, so
  each reason stays on the card a day.

## Pains Open

- Measures: `pains-open`
- Target: 0 pains he said that nobody has taken up.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: he said it hurt, and nobody is fixing it.

## Work-System Findings

- Measures: `symbiosis-findings`
- Target: 0 findings open past a day with no fix running.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: a fault in his agents' system that someone found and nobody fixed.

## Slips

- Measures: `slips`, `slips-derived`
- Target: 0 dates that passed in the last day without landing, or passed since with no landing,
  meeting or move from their owner, nor the ask put down or parked on a ruling.
- Joel, 11 Nov 16:4x: "i need for you to make faster progress. i don't understand why so much
  has not been completed."
- Over costs him: a date he was given passed, and what was promised isn't there.

## Slips Parked

- Measures: `slips-parked`, `slips-parked-derived`
- Target: none, since an ask parked on a ruling waits on that ruling, not its owner, and each is
  named by the ruling's message id so the parking itself is read.
- Joel, 22 Nov 15:2x: "WE NEED TO FOCUS ON SPREADING LOAD NOW! WE NEED TO QUICKEN MOVING STUFF
  INTO THE CLOUD!"
- Over costs him: nothing judged. An ask parked and forgotten would never slip, so the count
  by ruling stays on the card.

## Dates Told Late

- Measures: `dates-told-late`, `dates-told-late-derived`
- Target: 0 dates moved later in the last day with 15 minutes or less left before them, unless
  landed or met before the move, or parked on a ruling or put down. A date that passes before
  it moves is a slip, counted there.
- Joel, 22 Nov 18:10: "don't wait till monday to ask on shit."
- Over costs him: whoever waits on a date learns it moved only once it was too late to plan
  around.

## Broken on Main

- Measures: `broken`
- Target: 0 checks a merge broke in the last day that still fail.
- Joel, 14 Nov: "delivering insane quality, both functionally, and by maintaining clean zones"
- Over costs him: every change after waits on a red main or lands without the check.

## Reverts

- Measures: `reverts`
- Target: 0 merges the net reverted or quarantined a test for in the last day.
- Joel, 14 Nov: "delivering insane quality, both functionally, and by maintaining clean zones"
- Over costs him: work that landed broken and came out again, done twice.

## Quarantine Undated

- Measures: `quarantine-undated`
- Target: 0 quarantined tests with no date to come back.
- Joel, 14 Nov: "delivering insane quality, both functionally, and by maintaining clean zones"
- Over costs him: a test switched off for good without anyone saying so.

## Holds Undated

- Measures: `holds-undated`
- Target: 0 live holds on the mesh with no ask dating the fix that ends them.
- Joel, 24 Nov: "I need my agents to be able to get work done, they can't be neutered."
- Over costs him: agents stopped from working with nobody saying when they start again.

## Budgets Billed Under Logged

- Measures: `budgets-billed-under-logged`
- Target: 0 budgets billed under half what their sessions logged.
- Joel, 6 Nov 20:5x: "we need to treat token budgets as first class citizens ... this can't be
  a dirty hack, it needs to be clean and integrated and work for all the things"
- Over costs him: a team's work billed to his own key.

## Budgets Left Unresolved

- Measures: `budgets-left-unresolved`
- Target: 0 budgets sessions asked for and couldn't draw on.
- Joel, 6 Nov 20:5x: "we need to treat token budgets as first class citizens ... this can't be
  a dirty hack, it needs to be clean and integrated and work for all the things"
- Over costs him: a team's work going on his default key from the hour it starts.

## Spent Landing Nothing

- Measures: `spent-landing-nothing`
- Target: none, shown and never filed.
- Joel, 7 Nov 20:3x: "we need to constantly refine and improve it. along effectiveness, cost,
  and its ability to get shit done for me and remove frictions."
- Over costs him: nothing judged. It shows which quests spent today and landed nothing.

## Cells Left Standing

- Measures: `cells-idle`, `cells-parked-done`, `quests-without-cells`, `quests-cells-ended`,
  `quests-parked-past-start`
- Target: 0 sessions idle with work they could do, or parked on finished quests, and 0 moving
  quests with no cell, with their cells ended, or parked past a step's start.
- Joel, 23 Oct: "i need for them to all run autonomously ... make as much progress as possible
  ... without stopping unless they literally have nothing left to do"
- Over costs him: work he expects done overnight that nobody is doing.

## Cells the Map Draws Wrongly

- Measures: `map-cells`
- Target: 0 cells the map draws alone, on an unnamed network or out of reach.
- Joel, 24 Nov 13:15: "no way for me to go from there to the agent needing me ... no feedback
  at all"
- Over costs him: he can't find, from the map, the agent that needs him.

## Reports Nobody Put to Him

- Measures: `reports-unasked`
- Target: 0 finished reports and reviews no look on his list asks him about.
- Joel, 22 Nov 18:10: "don't wait till monday to ask on shit."
- Over costs him: work finished for him that he never hears about.

## Clean Zones

- Measures: `clean:*`
- Target: set where it's read: each measure's own, as code-health writes it in its hour.
- Joel, 14 Nov: "delivering insane quality, both functionally, and by maintaining clean zones"
- Over costs him: code that gets harder to change, so every later fix is slower to land.

## Measures Undefined

- Measures: `measures-undefined`
- Target: 0 measures on the card with no section here, or a target here they aren't held to.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: a figure with no reason behind it, pushing work he doesn't want or passing
  what he does.
