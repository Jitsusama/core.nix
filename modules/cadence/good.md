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

## Dates at Risk

- Measures: `dates-at-risk`, `dates-at-risk-derived`
- Target: 0 dates still to come that wait on a merge whose change shares files with what main
  has changed since the two parted, unless the change landed or its piece or ask was put down.
  Only core.nix changes and the zone's branches are judged. A date whose change can't be read
  is counted apart as unread, and with none at risk beside it the hour goes unjudged.
- Joel, 14 Nov: "something constantly evaluating how we can ship more and faster"
- Over costs him: a date is said to stand while its change will be refused or checked again,
  so it slips with no warning.

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

## The Sessions' Failed Model Calls

- Measures: `calls:failed`
- Target: 100 model calls the sessions made in the hour that stopped on an error.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: each one is a turn of his sessions that stalls and retries, whatever route it
  took, as a third of them did on the evening of 26 November.

## The Sessions' Failed Call Share

- Measures: `calls:failed-share`
- Target: 2 per cent of the sessions' model calls in the hour stopped on an error.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: a quiet hour hides failures in its count, so the share says when the way out
  is failing whatever the load.

## The Path to Each Origin

- Measures: `path:tls:*`
- Target: 1 second, the hour's median TLS setup to the origin.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: a slow path stalls every call and push his sessions make through it, and
  names where the fault is when the calls fail.

## The Model Relay's Origin Resets

- Measures: `relay:origin-resets`
- Target: 400 requests the origin reset in the hour.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: his sessions' model requests are cut and sent again, slowing every turn, and
  each reset past a retry reaches him as an error.

## The Model Relay Sending Pi Direct

- Measures: `relay:goes-direct`
- Target: 150 requests the relay gave back for pi to make itself in the hour.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: each one leaves the relay's shared connections, so his sessions open their
  own and meet "other side closed" when the origin drops them.

## The Model Relay's Reset Share

- Measures: `relay:reset-share`
- Target: 2 per cent of the requests asked of the relay in the hour reset by the origin.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: a busy hour hides resets in its count, so the share says when the origin is
  failing whatever the load.

## The Model Relay's 502s

- Measures: `relay:answered-502`
- Target: 50 requests the relay answered pi with a 502 in the hour.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: "502" on his own screen mid-turn, as he saw at 16:38 on 26 November before
  anything else did.

## The Node's Site Coming Live

- Measures: `site:live`
- Target: 2 seconds, the hour's median wait for every watch the site's page opens to answer.
- Joel, 26 Nov 16:42: "we need some set of cells maintaining the health of mycelium at all
  timees"
- Over costs him: the page he reads the mycelium by shows nothing new until its slowest watch
  answers, as at 22:40 on 26 November, when the message watch took 3.5 s.

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

## Lines Citing No Owner

- Measures: `lines-uncited`
- Target: 0 lines to him in the last day giving a live or switch time, or asking him to act or
  decide, with no message from a cell other than the writer cited before them, and 0 items the
  morning note puts to him naming no such message. A time counts as cited only when the message
  cited says it, and a switch time only from the cell running the switches. A message sent more
  than two days before backs nothing. A log, citation or message that can't be read leaves the
  hour unjudged.
- Joel, 25 Nov 09:04: "2 - you should not be presenting me with this, a cell concerned with
  this should."
- Over costs him: a time nobody running the work gave, or a decision nobody who owns it put to
  him, which he acts on as if they had.

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

## Causes Recurring

- Measures: `causes-recurring`
- Target: 0 classes of cause found on 3 or more changes in the last week.
- Joel, 26 Nov 11:01: "we need to not have so many rounds ... tackle things at root causes ...
  detect patterns of issues and just nip them in the bud."
- Over costs him: the same fault comes back change after change, each paid for with another
  review round, with nobody fixing its cause.

## Measures Undefined

- Measures: `measures-undefined`
- Target: 0 measures on the card with no section here, or a target here they aren't held to.
- Joel, 11 Nov 18:4x: "we need to learn from all pain points with all the things and be in a
  constant improvement loop. nothing can be out of spec for this."
- Over costs him: a figure with no reason behind it, pushing work he doesn't want or passing
  what he does.
