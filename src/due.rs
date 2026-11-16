//! When a queue's earliest retry is due, kept per queue rather than per
//! Message waiting (the owner, 2026-10-16; open problem 43): what the send
//! step wakes for, and how many wait, as each queue's last read showed and
//! as each hand-on since that read began noted (`runtime-model.md` section
//! 10). The runtime's send step keeps one [`Due`] per queue under its lock,
//! reads Xmip Storage for it and wakes at [`earliest`].
//!
//! **Counted by the newest read** (corrected 2026-11-12). How many wait is
//! what the latest begun of two reads said: a read of the whole queue, and
//! the read of its retries a hand-on that set a Message waiting makes once
//! Xmip Storage took it, which begins after that hand-on and so counts it,
//! once. A read begun before the one that set the count says nothing of
//! how many wait, however late it ends. Until 2026-11-12 a read's count
//! replaced the count and each hand-on added one to it, so a read begun
//! before a hand-on landed lost it and one that ended between a hand-on and
//! its note counted it twice; that day's first correction kept the count no
//! lower than the hand-ons noted since the read began, which said three
//! where three were found and a fourth noted.

/// When a queue's earliest retry is due, on this node's clock — Xmip
/// Storage's due time read as a duration from when the read was asked, or
/// a hand-on's backoff from when Xmip Storage took it — and how many wait
/// for one, as its last read showed: what the node wakes for, one per
/// queue however many wait. Every moment is a reading of the monotonic
/// clock the send step was handed, in nanoseconds
/// (`xcore::Clock::monotonic_nanos`; ADR-0068, amendment 2026-11-15), so a
/// test steps it past a due time with no sleep.
#[derive(Clone, Debug)]
pub struct Due {
    /// The Send Port whose queue it is.
    pub port: String,
    /// When the node wakes for the queue's earliest retry.
    pub at: Option<i128>,
    /// How many wait for a retry.
    pub waiting: u64,
    /// The earliest due time a hand-on noted since the queue's last read
    /// began: a read of the whole queue may have passed its place before
    /// Xmip Storage took that hand-on, so the read's answer does not drop
    /// it. Until 2026-10-16 it did, and a short backoff waited for the
    /// next `[tuning] send_scan`, a second by default, after it was due.
    noted: Option<i128>,
    /// When the read that set `waiting` began.
    counted: Option<i128>,
}

impl Due {
    /// None waiting yet, in the queue of the Send Port `port`.
    #[must_use]
    pub fn of(port: &str) -> Self {
        Self {
            port: port.to_string(),
            at: None,
            waiting: 0,
            noted: None,
            counted: None,
        }
    }

    /// Woken for at `at`, the earliest kept: a retry the queue's read found
    /// not yet due, which the read counts by Xmip Storage's count of them.
    #[inline]
    pub fn wakes_at(&mut self, at: i128) {
        self.at = Some(earlier(self.at, at));
    }

    /// `waiting` wait, as a read begun at `began` found them: kept where no
    /// read begun later counted them.
    #[inline]
    pub fn count(&mut self, began: i128, waiting: u64) {
        if self.counted.is_none_or(|counted| began >= counted) {
            self.waiting = waiting;
            self.counted = Some(began);
        }
    }

    /// Woken for at `at`, kept across the read under way, and not counted:
    /// a retry a scan's claim found not yet due, or a hand-on that set a
    /// Message waiting, which the queue's read counts (`runtime-model.md`
    /// section 10, corrected 2026-10-17).
    #[inline]
    pub fn wakes(&mut self, at: i128) {
        self.at = Some(earlier(self.at, at));
        self.noted = Some(earlier(self.noted, at));
    }

    /// A read of the queue begins: what a hand-on noted before now, the
    /// read finds in Xmip Storage.
    #[inline]
    pub const fn reading(&mut self) {
        self.noted = None;
    }

    /// A hand-on noted where Xmip Storage did not answer how many wait:
    /// one more than were counted.
    #[inline]
    pub const fn unanswered(&mut self) {
        self.waiting += 1;
    }

    /// What a scan begun at `began` found waiting, `waits`: the queue's due
    /// time as it found it where it read `whole` the queue — with every due
    /// time a hand-on noted since the read began — and how many wait where
    /// no read begun later counted them; else the earlier of its due time
    /// and what it found, and the more waiting.
    pub fn found(&mut self, waits: &Self, began: i128, whole: bool) {
        if whole {
            let noted = self.noted;
            self.at = waits.at;
            self.noted = None;
            self.count(began, waits.waiting);
            if let Some(at) = noted {
                self.at = Some(earlier(self.at, at));
                self.noted = noted;
            }
        } else if let Some(at) = waits.at {
            self.at = Some(earlier(self.at, at));
            self.waiting = self.waiting.max(waits.waiting);
        }
    }
}

/// When the send step wakes with nothing handed or asked: the earliest
/// retry due in any of `queues`, or `until`, its next scan.
#[inline]
#[must_use]
pub fn earliest<'a>(queues: impl IntoIterator<Item = &'a Due>, until: i128) -> i128 {
    queues
        .into_iter()
        .filter_map(|due| due.at)
        .fold(until, i128::min)
}

/// The earlier of `was`, where there is one, and `at`.
#[inline]
fn earlier(was: Option<i128>, at: i128) -> i128 {
    was.map_or(at, |was| was.min(at))
}

#[cfg(test)]
mod tests {
    use super::{Due, earliest};

    /// A millisecond and a second of the clock's readings, in nanoseconds.
    const MILLISECOND: i128 = 1_000_000;
    const SECOND: i128 = 1_000 * MILLISECOND;

    /// A reading of the clock, some way past its own moment.
    const NOW: i128 = 7 * SECOND;

    /// What a read of the whole queue finds: `waiting` waiting.
    fn found(waiting: u64) -> Due {
        let mut found = Due::of("Out");
        found.waiting = waiting;
        found
    }

    /// A hand-on noted during a read of the whole queue that found nothing
    /// waiting: its due time is kept, not dropped by the read.
    #[test]
    fn a_due_time_noted_during_a_read_outlives_the_read() {
        let mut due = Due::of("Out");
        let began = NOW;
        due.reading();
        let at = began + 20 * MILLISECOND;
        due.wakes(at);
        due.found(&Due::of("Out"), began, true);
        assert_eq!(due.at, Some(at));
    }

    /// What was noted before a read began, the read answers for.
    #[test]
    fn a_read_begun_after_a_note_answers_for_it() {
        let mut due = Due::of("Out");
        due.wakes(NOW + 3600 * SECOND);
        due.reading();
        due.found(&Due::of("Out"), NOW + MILLISECOND, true);
        assert_eq!(due.at, None);
        assert_eq!(due.waiting, 0);
    }

    /// A read begun before the count, however late it ends, says nothing
    /// of how many wait; one begun after it does.
    #[test]
    fn the_newest_read_counts() {
        let mut due = Due::of("Out");
        let stale = NOW;
        let fresh = stale + MILLISECOND;
        due.count(fresh, 2);
        due.found(&found(1), stale, true);
        assert_eq!(due.waiting, 2, "the newer count kept");
        due.found(&found(5), fresh + MILLISECOND, true);
        assert_eq!(due.waiting, 5, "a newer read's count");
    }

    /// A read of part of the queue keeps the earlier due time and the more
    /// waiting, and counts nothing.
    #[test]
    fn a_partial_read_keeps_the_earlier_and_the_more() {
        let now = NOW;
        let mut due = Due::of("Out");
        due.wakes_at(now + 2 * SECOND);
        due.count(now, 3);
        let mut part = found(1);
        part.wakes_at(now + SECOND);
        due.found(&part, now + MILLISECOND, false);
        assert_eq!(due.at, Some(now + SECOND));
        assert_eq!(due.waiting, 3);
        due.unanswered();
        assert_eq!(due.waiting, 4, "one more where Storage did not answer");
    }

    /// The send step wakes for the earliest queue's due time, or its scan.
    #[test]
    fn the_earliest_due_or_the_scan() {
        let now = NOW;
        let scan = now + 10 * SECOND;
        let (mut one, mut two) = (Due::of("A"), Due::of("B"));
        assert_eq!(earliest([&one, &two], scan), scan, "none due");
        one.wakes_at(now + 4 * SECOND);
        two.wakes_at(now + 3 * SECOND);
        assert_eq!(earliest([&one, &two], scan), now + 3 * SECOND);
    }
}
