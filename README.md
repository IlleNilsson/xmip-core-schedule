# xmip-core-schedule

Decides when Xmip work runs, and nothing else: it waits for nothing and runs
nothing. The runtime asks it, then waits on its node's stop or wake and does
the work. It is the estate's one home for scheduling: ADR-0058, amendment
2026-09-22, *the scheduling code if any done shall be in this repo*; its
first code moved here from `xmip-core-runtime` on 2026-11-13 (ADR-0058,
amendment of that date).

## The surface

- `pace`: how long a Receive Location rests before it asks its transport
  again. `Pace::rest` says, from how the receive ended (`Ended`), how the
  transport is asked (`Asking`: its receive waits by itself, it is polled,
  or it is polled and its far end tells of a change) and how long the
  receive took: nothing after a receive that found something or one that
  waits by itself, its poll or its sweep after one that found nothing, its
  backoff after one that failed in a way asking again may mend.
  `RECEIVE_POLL`, `RECEIVE_SWEEP`, `RECEIVE_BACKOFF` and `RECEIVE_DRAIN` are
  the `[tuning]` defaults (`doc/architecture/runtime-model.md` section 5).
- `due`: when a Send Port queue's earliest retry is due and how many wait
  for one, kept per queue however many wait (`Due`), counted by the newest
  read, a due time a hand-on noted kept across a read under way, and the
  `earliest` the send step wakes for (`doc/architecture/runtime-model.md`
  section 10, open problem 43), each moment a reading of the node's
  monotonic Clock in nanoseconds, so a test steps past a due time with no
  sleep (ADR-0068, amendment 2026-11-15).

Every default is a default only: a cluster and a node set their own in the
Cluster TOML's `[tuning]`.

## What stays in the runtime

The waiting and the work: a rest waited on the node's stop, the drain once
it stops, the send step's lock over its queues' `Due`, the reads of Xmip
Storage and the wake (`xmip-core-runtime`'s `receiving` and `send_step`).
The Work Process step's dispatch stays there too: it decides with the
runtime's pool, pickup and admission, and moving it here would make this
repository depend on the runtime.

## Build and test

```powershell
cargo test
```

No dependency: what it decides is read from a clock and durations alone.
