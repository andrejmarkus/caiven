# First user experiment — Quick Remix

Recorded 2026-09-25, revised 2026-09-28 after the readiness audit. The loop
in [quick-remix.md](quick-remix.md) is built. This test asks one question
before more of it gets built:

> When someone plays a tiny Caiven game, do they actually want to change it?

Behaviour is the evidence. Compliments aren't. The activation event: an
outside person, unexplained, plays a cart, notices Remix, changes something,
runs the changed version, and understands their edit caused the result.

The run itself (frozen build, starter IDs, smoke test, email checks) is
recorded in [experiment-001.md](experiment-001.md).

## Setup

| | |
| --- | --- |
| Audience | Two groups, roughly half each: (1) people with some coding curiosity or programming experience; (2) curious people who don't make games. No mass audience yet. |
| Sample | 3–5 observed pilot sessions, then 10–30 external people. Nobody who has seen Caiven before. |
| Entry | A direct `/play/<starter-id>` link from the **experiment set**, not the homepage. Rotate Juggle, Meteor, Hop, Chain in order. |
| Device | Their own device, network and browser, logged out. Not the observer's network: anonymous visitors on one IP are one viewer. |
| Observer | Watches (screen share or in person) and takes notes. The observer's own checking happens logged in as the admin account, so it stays out of the numbers. |
| Build | One frozen SHA for the whole phase. No redesign between subjects. |

### No coaching

Send the link with one line: "Here's a tiny game, have a go." Don't explain
Remix, the code, or publishing. Help only when the person is completely
stuck and the session would otherwise end. When you do, write down the step
and the words you used. A step that needs help counts as failed for that
person.

## Phases

**Before.** All gates in [experiment-001.md](experiment-001.md#gates): CI green
on the frozen SHA, deployed build = that SHA, smoke test passed, readout
matches the smoke session, email checked in real inboxes, fresh experiment
starter set published, freeze record filled in.

**Pilot (3–5 observed).** Same link, same one line. Fix only defects that
break the experiment (lost edits, a publish that can't finish, a wedged
page, wrong numbers). Any fix means a new SHA, a new experiment starter set
and a new `since`; pilot numbers are not pooled with the main phase.

**Main (10–30).** Same build and starter set from the first to the last
person. Collect notes, read the numbers at the end, then decide.

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
9. Made a second change without being prompted? (Observed only: nothing
   records it.)
10. Attempted Publish?
11. Blocked by sign-up or email confirmation? Where?
12. Published?
13. Wanted to share the result (sent the link, or asked how)?
14. Would open someone else's remix? (Show one and watch.)
15. Would come back for another challenge? (Ask once, at the end.)

Don't ask "Do you like Caiven?" or anything else that invites a polite
answer. Ask "what did you expect to happen?" at a moment of confusion.

## Reading the numbers

```sh
CAIVEN_PORT_URL=https://port.example CAIVEN_PORT_API_KEY=<admin token> \
  scripts/experiment/readout.sh <since> <starter-id> ...
```

It prints `GET /api/v2/admin/metrics/remix-funnel?since=…`, starters first,
each remix under them. Staff (admin) activity is excluded.

| Column | Event | Recorded | Unit |
| --- | --- | --- | --- |
| play started | `play_events` | server, when the Play page's game boots | unique viewer keys |
| qualified play | `qualified_play` | Play page, see below | unique viewer keys |
| remix opened | `remix_opened` | Remix page loaded with the code | unique viewer keys |
| changed run ok | `remix_ran` | a changed source loaded and ran 60 frames without a fault (or was running cleanly when Publish was pressed) | unique viewer keys |
| publish pressed | `publish_started` | Publish pressed, before any sign-up | unique viewer keys |
| remixes published | `carts.parent_cart_id` | derived on the server | carts |
| …played by another | a child's `qualified_play` from a key that isn't its owner's account | derived | carts |
| …remixed again | a child that has its own child | derived | carts |

Not recorded at all: the first edit before a run, a second edit, publish
failures, shares. Those come from the notes.

**Viewer key.** SHA-256 of `user:<id>` when logged in, `ip:<address>` when
not. One row per (cart, event, viewer key), kept forever; the row's time is
the first occurrence, and `since` filters on it. A refresh or a repeat
visit adds nothing.

**Qualified play.** 20 s (1,200 frames) of the game running on `/play`,
counted from the first button press (key, touch or gamepad), only while the
tab is visible, never after a crash. Leaving early, crashing first, or
clicking Remix before 20 s records nothing; time on the Remix page doesn't
count. So `remix_opened` can exceed `qualified_plays`, and an eager person
may never have a qualified play.

### Known biases

- **Not a cohort.** Every column counts keys independently. The readout's
  ratios are stage-count ratios, not "x % of people went on to…", and can
  exceed 1.
- **Sign-up splits a person.** Before sign-up they are `ip:…`, after it
  `user:…`. The Remix page returning from the account wall doesn't report
  its steps again, but anything they do afterwards (play another starter,
  remix again) counts as a new viewer.
- **Shared IP merges people.** Same wifi, same office, some mobile carrier
  NAT: one key. Participants on the observer's or the smoke tester's network
  are merged with them, and a step they already reached is never recorded
  again (dedup is forever).
- **Old events hide new ones.** A viewer who reached a step on a cart before
  `since` is invisible for that step in the window. The experiment set is
  published fresh after the smoke test so no participant meets that.
- **Remix counts are visible.** "N remixes so far" on the Play page includes
  earlier participants. Later participants see a busier cart; note the order.
- A creator opening their own remix while logged out, or on their phone,
  counts as an external play. Cross-check with the notes.
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
restarts on A. A test plays each **try N** alone for 10 s against the shipped
game: 22 of 24 change the picture. Hop `SPEED_UP` only shows after a pipe is
passed and Meteor `SHAKE` only on a crash, so a person who tries those first
may see nothing yet. Watch for that in the notes.
