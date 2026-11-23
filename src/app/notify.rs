//! Which events deserve a Windows notification (no egui or Win32 here, so it is unit-tested).
//! Failures always produce a note; a finished queue produces one summary, and successful items stay silent.

use crate::engine::queue::State;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    Failed { title: String, code: Option<String> },
    QueueDone { done: u64, failed: u64 },
}

#[derive(Debug, Default)]
pub struct Tracker {
    running: bool,
    done: u64,
    failed: u64,
}

impl Tracker {
    /// Summary on running → stopped, but only if something finished during that run.
    pub fn on_queue_running(&mut self, running: bool) -> Option<Note> {
        if running == self.running {
            return None;
        }
        self.running = running;
        if running {
            self.done = 0;
            self.failed = 0;
            return None;
        }
        let note = (self.done + self.failed > 0).then_some(Note::QueueDone {
            done: self.done,
            failed: self.failed,
        });
        self.done = 0;
        self.failed = 0;
        note
    }

    /// `prev` is the item's last known state (None for a new item).
    pub fn on_item_state(
        &mut self,
        prev: Option<State>,
        new: State,
        title: &str,
        code: Option<&str>,
    ) -> Option<Note> {
        // prev None = first sight (e.g. queue loaded at start): an old failure is not news
        let prev = prev?;
        if prev == new {
            return None;
        }
        match new {
            State::Done => {
                if self.running {
                    self.done += 1;
                }
                None
            }
            State::Failed => {
                if self.running {
                    self.failed += 1;
                }
                Some(Note::Failed {
                    title: title.to_string(),
                    code: code.map(str::to_string),
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_note_once_per_transition() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        let n = t.on_item_state(
            Some(State::Installing),
            State::Failed,
            "LEGO",
            Some("RPI-04"),
        );
        assert_eq!(
            n,
            Some(Note::Failed {
                title: "LEGO".into(),
                code: Some("RPI-04".into())
            })
        );
        assert_eq!(
            t.on_item_state(Some(State::Failed), State::Failed, "LEGO", Some("RPI-04")),
            None
        );
    }

    #[test]
    fn no_note_for_items_loaded_already_failed() {
        let mut t = Tracker::default();
        assert_eq!(
            t.on_item_state(None, State::Failed, "LEGO", Some("RPI-04")),
            None
        );
    }

    #[test]
    fn no_note_for_done_cancelled_or_skipped() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        for s in [State::Done, State::Cancelled, State::Skipped] {
            assert_eq!(t.on_item_state(Some(State::Installing), s, "x", None), None);
        }
    }

    #[test]
    fn queue_summary_counts() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        t.on_item_state(Some(State::Installing), State::Done, "a", None);
        t.on_item_state(Some(State::Installing), State::Done, "b", None);
        t.on_item_state(Some(State::Installing), State::Failed, "c", Some("RPI-01"));
        assert_eq!(
            t.on_queue_running(false),
            Some(Note::QueueDone { done: 2, failed: 1 })
        );
    }

    #[test]
    fn no_summary_when_nothing_finished() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        t.on_item_state(Some(State::Installing), State::Cancelled, "a", None);
        assert_eq!(t.on_queue_running(false), None);
    }

    #[test]
    fn counts_reset_between_runs() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        t.on_item_state(Some(State::Installing), State::Done, "a", None);
        assert!(t.on_queue_running(false).is_some());
        t.on_queue_running(true);
        t.on_item_state(Some(State::Installing), State::Done, "b", None);
        assert_eq!(
            t.on_queue_running(false),
            Some(Note::QueueDone { done: 1, failed: 0 })
        );
    }

    #[test]
    fn items_finishing_while_not_running_are_not_counted() {
        let mut t = Tracker::default();
        assert_eq!(
            t.on_item_state(Some(State::Installing), State::Done, "a", None),
            None
        );
        t.on_queue_running(true);
        assert_eq!(t.on_queue_running(false), None);
        // a failure is still announced when no run is active, but not counted in a summary
        let n = t.on_item_state(Some(State::Checking), State::Failed, "b", None);
        assert_eq!(
            n,
            Some(Note::Failed {
                title: "b".into(),
                code: None
            })
        );
        assert_eq!(t.on_queue_running(false), None);
    }

    #[test]
    fn repeated_running_flag_does_not_reset_counts() {
        let mut t = Tracker::default();
        t.on_queue_running(true);
        t.on_item_state(Some(State::Installing), State::Done, "a", None);
        assert_eq!(t.on_queue_running(true), None);
        assert_eq!(
            t.on_queue_running(false),
            Some(Note::QueueDone { done: 1, failed: 0 })
        );
    }
}
