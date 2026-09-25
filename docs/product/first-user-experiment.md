# First user experiment — Quick Remix

Recorded 2026-09-25. The loop in [quick-remix.md](quick-remix.md) is
built. This test asks one question before more of it gets built:

> When someone plays a tiny Caiven game, do they actually want to change it?

Behaviour is the evidence. Compliments aren't.

## Setup

| | |
| --- | --- |
| Audience | Two groups, roughly half each: (1) people with some coding curiosity or programming experience; (2) curious people who don't make games. No mass audience yet. |
| Sample | 10–30 external people. Nobody who has seen Caiven before. |
| Entry | A direct `/play/<starter-id>` link, not the homepage. The test is the core loop, not navigation. Rotate the four starters (Juggle, Meteor, Hop, Chain) through participants in order, so each gets a few. |
| Device | Their own device, network and browser, logged out. A shared office network makes anonymous visitors one IP, which merges their funnel rows. |
| Observer | Watches (screen share or in person) and takes notes. The observer's own checking happens logged in as the admin account, so it stays out of the numbers. |
| Window | Note the time of the first session. Every readout uses `since=<that time>`. |

### No coaching

Send the link with one line: "Here's a tiny game, have a go." Don't explain
Remix, the code, or publishing. Help only when the person is completely
stuck and the session would otherwise end. When you do, write down the step
and the words you used. A step that needs help counts as failed for that
person.

## Per-session sheet

Mark yes / no / helped for each, and add a line of what they said or did.

1. Started playing (pressed a button within ~10 s)?
2. Noticed Remix?
3. Clicked it without prompting?
4. Understood what to edit?
5. Used a suggested constant (a chip or **try N**)?
6. Edited code by hand?
7. Ran a change successfully?
8. Understood that their change caused what changed in the game?
9. Tried a second change?
10. Attempted Publish?
11. Blocked by sign-up or email confirmation? Where?
12. Wanted to share the result (sent the link, or asked how)?
13. Would open someone else's remix? (Show one and watch.)
14. Would come back for another challenge? (Ask once, at the end.)

Don't ask "Do you like Caiven?" or anything else that invites a polite
answer. Ask "what did you expect to happen?" at a moment of confusion.

## Reading the numbers

```text
GET /api/v2/admin/metrics/remix-funnel?since=2026-10-01T09:00:00Z
X-Api-Key: <admin full-scope token>
```

`by_cart` has one row per cart. Starter rows answer questions 1–6 of the
experiment (plays, qualified plays, remix opened, remix ran, publish started,
remixes published). A published remix appears as its own row. Its
`qualified_plays` tells you whether anyone else played it, and the parent's
`remixes_with_external_play` / `remixes_remixed` count the second generation.
`conversion` gives each step-to-step ratio, or `null` where the earlier step
is zero.

| Field | Means |
| --- | --- |
| `plays` | Distinct viewers whose game booted |
| `qualified_plays` | Viewers who played 20 s after their first button press, without a crash |
| `remix_opened` | Viewers who loaded the remix page |
| `remix_ran` | Viewers whose changed code ran cleanly at least once |
| `publish_started` | Viewers who pressed Publish (before any sign-up) |
| `remixes_published` | Remixes of this cart published in the window |
| `remixes_with_external_play` | …of which someone other than the remixer played one |
| `remixes_remixed` | …of which someone remixed again |

With 10–30 people, the numbers point to a step for the session notes to
explain. They are not rates to benchmark. Staff (admin) activity is excluded;
add `include_staff=true` to see everything.

### Known biases

- Anonymous viewers on one IP (shared wifi, one office) count as one. A
  visitor who signs up counts as two keys for the steps after sign-up; the
  remix page doesn't re-report the steps it already counted.
- The observer counts as a participant if logged out. Stay logged in as admin.
- A creator opening their own remix while logged out, or on their phone,
  counts as an external play. Cross-check with the notes.
- Dedup is forever per cart and viewer: a participant who comes back
  tomorrow is not a second person, and a repeat visit doesn't show up.
- Link-preview bots don't run the game, so they never count as plays.
- The funnel can't see why someone stopped. That's what the notes are for.

## What each drop-off means

Decided before the sessions, so the results don't get argued into a plan.

| Pattern | Likely cause | Next work |
| --- | --- | --- |
| Many qualified plays, few remix opens | Weak remix desire, weak invitation, or the seed doesn't make people curious | PLAY → REMIX. Not editor features. |
| Many remix opens, few successful runs | Editor looks intimidating, suggestions unclear, feedback weak, errors blocking | REMIX → CHANGE → RUN |
| Many runs, few publish starts | Little sense of ownership, unclear why to publish, private tinkering is enough | RUN → OWNERSHIP → PUBLISH |
| Many publish starts, few publishes | Sign-up, email confirmation, form friction, publish failures | The publish flow |
| Many published remixes, few external plays | Weak sharing or preview, people don't want to send what they made | PUBLISH → SHARE → PLAY |
| Remixes get played but nobody remixes a remix | Remix works as onboarding to creation, not as a social loop | Decide which of those Caiven is before building social features |

That last distinction matters most. A loop that onboards creators and a
loop that spreads between people need different next steps.

## Starters, as tested

| Starter | First seconds | Goal / controls | Wildest first change |
| --- | --- | --- | --- |
| Juggle | Balls already falling | Keep them up; ← → | `PADDLE_WIDTH` try 190, `BALLS` try 30 |
| Meteor | Rocks already falling | Dodge; ← → | `ROCK_SIZE` try 25, `ROCKS_PER_SECOND` try 20 |
| Hop | "PRESS A TO FLAP" | Flap through gaps; A (Z/Space) or ↑ | `GRAVITY` try 0.05, `GAP` try 20 |
| Chain | Aim cursor over drifting dots | One shot; arrows + A | `DOTS` try 200, `BLAST_SIZE` try 30 |

Each has six constants at the top with `-- try N` hints, surfaced as
**try N** buttons. A wild value is undone with **Start over**, and each game
restarts on A.

## Before the first session

1. Deploy with the checklist in
   [../development/port-operations.md](../development/port-operations.md).
   `CAIVEN_IP_HEADER`, `CAIVEN_BASE_URL`, `CAIVEN_SECURE_COOKIES=true` and
   complete SMTP are required for this test.
2. Register the admin account first. The first account on a fresh Port
   becomes admin.
3. Publish the starters: `scripts/remix-seeds/publish.sh` with an admin
   full-scope token. Check that Home shows **Start here**.
4. Dry run on a phone and a laptop, logged out, on a network other than the
   server's: play → remix → **try N** → Publish → register → confirm the email
   → publish → open the share link in a private window. Check that the
   readout shows each step once. The dry run's anonymous steps count, so
   the experiment's `since` goes after it.
5. Note the start time, then send the first link.
