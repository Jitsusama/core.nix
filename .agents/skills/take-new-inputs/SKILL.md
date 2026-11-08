---
name: take-new-inputs
description: >
  Take every flake input to its newest, once a week or at once for a
  security fix: update the lock, fix what the new nixpkgs and home-manager
  break, check the inputs pinned by commit and the kernel's point release,
  run everything on the builders, and merge. Use when asked to update the
  inputs, bump nixpkgs, take new pins, or when the lock is a week old.
---

# Take New Inputs

Software that isn't updated is the easiest way into a machine: a fix that
upstream shipped weeks ago does nothing for one still running the old
release. So core.nix takes every input to its newest each weekend, and at
once when a fix for something its machines run is announced. A bump is a
change every check reads, so it runs everything on the builders before it
merges ([docs/testing.md][testing]), and the machine repositories take it
afterwards with `nix flake update core` ([docs/machines.md][machines]).

The lock rarely updates cleanly: the 7 November bump needed a second
commit, since nixpkgs had dropped the scripted initrd and changed
OpenVINO's build. Fixing what breaks is most of the job, and it needs
judgement, which is why this is a skill and not a script.

## Steps

1. **Cut a tree** at a branch named for the day, such as
   `inputs-to-11-14`, from origin's main.

2. **Update the lock.** `nix flake update` moves every input that follows
   a branch. Inputs pinned by commit or tag in `flake.nix` stay put; step 4
   is for them. Commit the lock alone, as
   `chore(flake): take every input to <date>`, with the date each input
   moved to in the body, from `git diff flake.lock` (`lastModified`).

3. **Fix what breaks.** Evaluate before building anything:
   `changed-checks --name-only --no-vm` fails at once on a removed option, a
   renamed package or an assertion. For each break:
   - Read why upstream changed it, in the nixpkgs or home-manager commit
     that did, and meet the change in the module that breaks. Never pin an
     input back to dodge one, and never paper over it with `mkForce`.
   - A check that only read a setting upstream removed stops reading it.
   - A module that worked around a bug upstream has now fixed drops its
     workaround, and its comment with it.
   Commit the fixes as `fix: meet <month>'s nixpkgs and home-manager`,
   whose body says what each upstream change was and how the module meets
   it.

4. **Look at what's pinned.** Each input pinned by commit or tag has a
   comment saying why, and what it waits for. For each, look for a newer
   release, and whether it carries what the comment waits for; if it does,
   move the pin there, back to a tag where the comment says so. lanzaboote
   and disko decide whether a machine boots, so a move of either runs the
   boot VM tests `merge-net needs` names.

5. **Look at the kernel.** The kernel's point release is named in `stable`
   in `modules/kernel/package.nix`, not taken from nixpkgs, so the lock
   never moves it. Compare it with the newest release of its series in
   kernel.org's `releases.json`. A newer one carries that week's security
   fixes: take it as [docs/kernel.md][kernel] says, under Changing It. The
   kernel has an owner: send them what you found if they aren't you.

6. **Run everything, on the builders.** Commit first, since a builder
   fetches only a commit. Then `merge-net needs`, which names every VM test
   a lock change moves, and `changed-checks --build`, which builds every
   check the change moved, VM tests included, on the builders. A bump
   moves nearly every check, so this is the whole run: start it in the
   background and do step 7 while it goes. A check that fails is read and
   fixed as in step 3, never retried unread.

7. **Build the machines.** Each machine repository builds its machines
   with core overridden to the tree, as [docs/testing.md][testing] says
   in Checking a Machine Repository Against a Change. A machine that fails
   to build here would fail to switch.

8. **Merge, and say what moved.** The pull request's description names the
   date each input moved to, every fix and why, and what a person might
   notice. Anything that needs a restart to take effect, such as the
   kernel, the boot loader or the initrd, is named, since it waits for its
   person to restart. Then the machine repositories take the new core.

## When It's Urgent

A security fix announced for something the machines run (the kernel,
OpenSSH, systemd, a browser, the Nix daemon) doesn't wait for the weekend.
Take only that input, or only the kernel's point release, and if
`merge-net needs` names no VM test, `changed-checks` alone gates it
([docs/delivery.md][delivery]).

[delivery]: ../../../docs/delivery.md
[kernel]: ../../../docs/kernel.md
[machines]: ../../../docs/machines.md
[testing]: ../../../docs/testing.md
