//! When Xmip work runs: the estate's one home for scheduling (ADR-0058,
//! amendment 2026-09-22: *the scheduling code if any done shall be in this
//! repo*).
//!
//! It decides when, and waits for nothing and runs nothing: the runtime
//! asks it, then waits on its node's stop or wake and does the work.
//!
//! - [`pace`]: how long a Receive Location rests before it asks its
//!   transport again — its poll, its sweep, its backoff — and how long it
//!   drains once its node stops (`runtime-model.md` section 5).
//! - [`due`]: when a Send Port queue's earliest retry is due, and how many
//!   wait for one, kept per queue (`runtime-model.md` section 10).
//!
//! Each default is the node's `[tuning]` default; a cluster and a node set
//! their own in the Cluster TOML.

pub mod due;
pub mod pace;
