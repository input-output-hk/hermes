//! Cron State.
/// The crontabs hash map.
use std::hash::{Hash, Hasher};

use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
};

use super::{
    event::OnCronEvent,
    queue::{cron_queue_task, CronEventQueue, CronJob},
};
use crate::runtime_extensions::{
    bindings::hermes::cron::api::{CronEventTag, CronTagged, Instant},
    hermes::cron::mkdelay_crontab,
};

/// Cron Internal State
pub(crate) static CRON_INTERNAL_STATE: Lazy<InternalState> = Lazy::new(|| {
    let sender = if let Ok(runtime) = Builder::new_current_thread().enable_all().build() {
        let (sender, receiver) = mpsc::channel(1);

        let _handle = std::thread::spawn(move || {
            runtime.block_on(async move {
                let h = tokio::spawn(cron_queue_task(receiver));
                drop(tokio::join!(h));
            });
            std::process::exit(0);
        });
        Some(sender)
    } else {
        // TODO(saibatizoku): log error
        // Failed to start the queue task
        None
    };

    InternalState::new(sender)
});

/// Name of the Application that owns the `OnCronEvent`s.
pub type AppName = String;

/// Internal State.
pub(crate) struct InternalState {
    /// The send events to the crontab queue.
    pub(crate) cron_queue: CronEventQueue,
}

impl InternalState {
    /// Create a new `InternalState`.
    pub(crate) fn new(sender: Option<mpsc::Sender<CronJob>>) -> Self {
        Self {
            cron_queue: CronEventQueue::new(sender),
        }
    }

    /// Add a new crontab entry.
    ///
    /// Allows for management of scheduled cron events queue.
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    ///
    /// ## Parameters
    ///
    /// - `app_name`:  `AppName`. The name of the application that owns the crontab.
    /// - `entry`:  `CronTagged`. The crontab entry to add.
    /// - `retrigger`:  `bool`. If `true`, the event will re-trigger every time the
    ///   crontab entry matches until cancelled.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.
    /// - `false`: Crontab failed to be added.
    pub(crate) fn add_crontab(&self, app_name: &str, entry: CronTagged, retrigger: bool) -> bool {
        let crontab = OnCronEvent {
            tag: entry,
            last: retrigger,
        };
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Add(app_name.into(), crontab, cmd_tx)),
        );
        // TODO(saibatizoku): deal with errors
        if let Ok(resp) = cmd_rx.blocking_recv() {
            resp
        } else {
            false
        }
    }

    /// Schedule a single cron event after a fixed delay.
    ///
    /// Allows for easy timed wait events to be delivered without
    /// requiring datetime calculations or formatting cron entries.
    ///
    /// ## Parameters
    ///
    /// - `app_name`:  `AppName`. The name of the application that owns the crontab.
    /// - `duration`: `Instant`. How many nanoseconds to delay.  The delay will be AT
    ///   LEAST this long.
    /// - `tag`:  `CronEventTag`. A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `Ok(true)`: Crontab added successfully.
    /// - `Ok(false)`: Crontab failed to be added.
    /// - `Err`: Returns error if the duration is invalid for generating a crontab entry.
    pub(crate) fn delay_crontab(
        &self, app_name: &str, duration: Instant, tag: CronEventTag,
    ) -> wasmtime::Result<bool> {
        let cron_delay = mkdelay_crontab(duration, tag)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Delay(app_name.into(), cron_delay, cmd_tx)),
        );
        // TODO(saibatizoku): deal with errors
        if let Ok(resp) = cmd_rx.blocking_recv() {
            Ok(resp)
        } else {
            Ok(false)
        }
    }

    /// List crontabs for an application.
    ///
    /// Allows for management of scheduled cron events queue.
    /// If `tag` is `none` then all crontabs are listed.
    /// Otherwise, only the crontabs with the specified tag are listed.
    ///
    /// ## Parameters
    ///
    /// - `tag`: Optional, the tag to limit the list to.  If `none` then all crons listed.
    ///
    /// ## Returns
    ///
    /// - A list of tuples containing the scheduled crontabs and their tags, along with
    ///   the current retrigger flag.  `Vec<(CronEventTag, bool)>`
    /// The list is sorted from most crontab that will trigger soonest to latest.
    /// Crontabs are only listed once, in the case where a crontab may be scheduled
    /// may times before a later one.
    pub(crate) fn ls_crontabs(
        &self, app_name: &str, tag: Option<CronEventTag>,
    ) -> Vec<(CronTagged, bool)> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::List(app_name.into(), tag, cmd_tx)),
        );
        // TODO(saibatizoku): deal with errors
        if let Ok(resp) = cmd_rx.blocking_recv() {
            resp
        } else {
            vec![]
        }
    }

    /// Remove the requested crontab.
    ///
    /// Allows for management of scheduled cron events.
    ///
    /// ## Parameters
    ///
    /// - `when`: The crontab entry to add.  Standard crontab format.
    /// - `tag`: A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `true`: The requested crontab was deleted and will not trigger.
    /// - `false`: The requested crontab does not exist.
    pub(crate) fn rm_crontab(&self, app_name: &str, entry: CronTagged) -> bool {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Remove(app_name.into(), entry, cmd_tx)),
        );
        // TODO(saibatizoku): deal with errors
        if let Ok(resp) = cmd_rx.blocking_recv() {
            resp
        } else {
            false
        }
    }
}

impl Hash for CronTagged {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.when.hash(state);
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;

    const APP_NAME: &str = "test";
    const EVERY_MINUTE_WHEN: &str = "* * * * *";
    const EVERY_DAY_WHEN: &str = "0 0 * * *";
    const EVERY_MONTH_WHEN: &str = "0 0 1 * *";
    const EXAMPLE_TAG: &str = "ExampleTag";
    const OTHER_TAG: &str = "OtherTag";

    // triggers every minute
    fn crontab_example_1() -> CronTagged {
        CronTagged {
            when: EVERY_MINUTE_WHEN.into(),
            tag: EXAMPLE_TAG.into(),
        }
    }
    // triggers every minute
    fn crontab_example_2() -> CronTagged {
        CronTagged {
            when: EVERY_MONTH_WHEN.into(),
            tag: EXAMPLE_TAG.into(),
        }
    }
    // triggers every minute
    fn crontab_example_3() -> CronTagged {
        CronTagged {
            when: EVERY_DAY_WHEN.into(),
            tag: EXAMPLE_TAG.into(),
        }
    }
    // triggers every minute
    fn crontab_other_1() -> CronTagged {
        CronTagged {
            when: EVERY_MINUTE_WHEN.into(),
            tag: OTHER_TAG.into(),
        }
    }
    const RETRIGGER_YES: bool = true;
    const RETRIGGER_NO: bool = false;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_internal_state_with_no_tokio_task() {
        // start the state without a cron queue task thread
        let state = InternalState::new(None);

        // Add returns false
        assert!(!state.add_crontab(APP_NAME, crontab_example_1(), RETRIGGER_YES));
        // List returns empty vec.
        assert!(state.ls_crontabs(APP_NAME, None).is_empty());
        // Delay returns false
        assert!(!state
            .delay_crontab(APP_NAME, 0, "test".to_string())
            .unwrap());
        // Remove returns false
        assert!(!state.rm_crontab(APP_NAME, CronTagged {
            when: "*".to_string(),
            tag: "test".to_string()
        }));
    }

    #[test]
    fn test_cron_state() {
        let state = &CRON_INTERNAL_STATE;
        // Initial state for any AppName is always empty
        assert!(state.ls_crontabs(APP_NAME, None).is_empty());
    }

    #[test]
    fn test_cron_state_add_and_list_crontabs() {
        // Initial state for any AppName is always empty

        let state = &CRON_INTERNAL_STATE;
        assert!(state.ls_crontabs(APP_NAME, None).is_empty());

        // inserting returns true
        assert!(state.add_crontab(APP_NAME, crontab_example_1(), RETRIGGER_YES));
        // re-inserting returns true
        assert!(state.add_crontab(APP_NAME, crontab_example_1(), RETRIGGER_NO));

        assert_eq!(state.ls_crontabs(APP_NAME, None), vec![
            (crontab_example_1(), RETRIGGER_YES),
            (crontab_example_1(), RETRIGGER_NO),
        ]);

        assert!(state.add_crontab(APP_NAME, crontab_example_2(), RETRIGGER_YES));

        assert!(state.add_crontab(APP_NAME, crontab_example_2(), RETRIGGER_YES));

        assert_eq!(state.ls_crontabs(APP_NAME, None), vec![
            (crontab_example_1(), RETRIGGER_YES),
            (crontab_example_1(), RETRIGGER_NO),
            (crontab_example_2(), RETRIGGER_YES),
            (crontab_example_2(), RETRIGGER_YES),
        ]);

        assert!(state.add_crontab(APP_NAME, crontab_other_1(), RETRIGGER_YES));

        assert_eq!(state.ls_crontabs(APP_NAME, None), vec![
            (crontab_example_1(), RETRIGGER_YES),
            (crontab_example_1(), RETRIGGER_NO),
            (crontab_other_1(), RETRIGGER_YES),
            (crontab_example_2(), RETRIGGER_YES),
            (crontab_example_2(), RETRIGGER_YES),
        ]);
    }
}
