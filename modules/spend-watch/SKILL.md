---
name: spend-watch
description: >
  What AI use has cost, from spend-watch's history: the bill where a feed
  gives one, pi's logs at list prices where not. Use when asked what
  something cost, where the money went, how spend compares with usual,
  how close a cap is, what's coming this week, which sessions, quests,
  models or agents cost most, or how much went on rebuilding context.
---

# spend-watch

spend-watch keeps a history of every dollar the account's AI use cost, by
hour, session, quest, model, agent and repo. Its figures are the bill where a
feed gives one (a provider's warehouse, then its live meter), an estimate from
pi's logs times each model's learned ratio of bill to list where the bill
hasn't come ("about"), and pi's list prices where there's no bill at all.
Every report says which.

Prefer it to pricing pi's logs yourself: list prices run well above a
negotiated bill, and spend-watch has the bill.

## Asking

Reports answer from the history, whether the daemon runs or not. Add
`--format json` for a machine to read, `markdown` to quote.

```bash
spend-watch report                         # the week at a glance
spend-watch report last week by quest      # a period, cut one way
spend-watch report today by session
spend-watch report this month by model
spend-watch report 30 days by agent
spend-watch report waste                   # context rebuilt, by cause
spend-watch report caps                    # caps and their changes
spend-watch report quests                  # spend beside status and PRs
spend-watch report outliers                # sessions costly per turn
spend-watch report forecast                # the week ahead, with ranges
spend-watch findings                       # what holds now
```

Periods: today, yesterday, this week, last week, this month, last month, 30
days. Cuts: quest, model, agent, repo, session, tool, day, source.

## Reading the Answers

- A figure reading "about" is estimated; a plain one is billed or at list
  prices, as the report's first line says.
- A forecast's range is its 80% one, checked against the history by
  `spend-watch backtest`.
- A finding marked expected (`spend-watch findings expect <key>`) stays in
  the collection but never asks for the person again. Mark one only when
  the person says it was expected.
