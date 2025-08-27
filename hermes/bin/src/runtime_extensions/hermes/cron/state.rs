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
    queue::{CronEventQueue, CronJob, CronJobDelay},
};
use crate::{
    app::ApplicationName,
    event::{queue::send, HermesEvent, TargetApp, TargetModule},
    runtime_extensions::{
        bindings::hermes::cron::api::{CronEventTag, CronTagged, Instant},
        hermes::cron::mkdelay_crontab,
    },
};

/// Cron Internal State
static CRON_INTERNAL_STATE: Lazy<InternalState> = Lazy::new(|| {
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
        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
        // Failed to start the queue task
        None
    };

    InternalState::new(sender)
});

/// Internal State.
struct InternalState {
    /// The send events to the crontab queue.
    cron_queue: CronEventQueue,
}

impl InternalState {
    /// Create a new `InternalState`.
    fn new(sender: Option<mpsc::Sender<CronJob>>) -> Self {
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
    /// - `app_name`:  `HermesAppName`. The name of the application that owns the crontab.
    /// - `entry`:  `CronTagged`. The crontab entry to add.
    /// - `retrigger`:  `bool`. If `true`, the event will re-trigger every time the
    ///   crontab entry matches until cancelled.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.
    /// - `false`: Crontab failed to be added.
    fn add_crontab(
        &self,
        app_name: &ApplicationName,
        entry: CronTagged,
        retrigger: bool,
    ) -> bool {
        let crontab = OnCronEvent {
            tag: entry,
            last: !retrigger,
        };
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Add(app_name.clone(), crontab, cmd_tx)),
        );
        cmd_rx.blocking_recv().unwrap_or(false)
    }

    /// Schedule a single cron event after a fixed delay.
    ///
    /// Allows for easy timed wait events to be delivered without
    /// requiring datetime calculations or formatting cron entries.
    ///
    /// ## Parameters
    ///
    /// - `app_name`:  `HermesAppName`. The name of the application that owns the crontab.
    /// - `duration`: `Instant`. How many nanoseconds to delay.  The delay will be AT
    ///   LEAST this long.
    /// - `tag`:  `CronEventTag`. A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `Ok(true)`: Crontab added successfully.
    /// - `Ok(false)`: Crontab failed to be added.
    /// - `Err`: Returns error if the duration is invalid for generating a crontab entry.
    fn delay_crontab(
        &self,
        app_name: &ApplicationName,
        duration: Instant,
        tag: CronEventTag,
    ) -> wasmtime::Result<bool> {
        let cron_delay = mkdelay_crontab(duration, tag)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Delay(app_name.clone(), cron_delay, cmd_tx)),
        );
        if let Ok(resp) = cmd_rx.blocking_recv() {
            Ok(resp)
        } else {
            // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
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
    ///   the current retrigger flag.  `Vec<(CronEventTag, bool)>` The list is sorted from
    ///   most crontab that will trigger soonest to latest. Crontabs are only listed once,
    ///   in the case where a crontab may be scheduled may times before a later one.
    fn ls_crontabs(
        &self,
        app_name: &ApplicationName,
        tag: Option<CronEventTag>,
    ) -> Vec<(CronTagged, bool)> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::List(app_name.clone(), tag, cmd_tx)),
        );
        // TODO (@@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
        cmd_rx.blocking_recv().unwrap_or_default()
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
    fn rm_crontab(
        &self,
        app_name: &ApplicationName,
        entry: CronTagged,
    ) -> bool {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        drop(
            self.cron_queue
                .spawn_cron_job(CronJob::Remove(app_name.clone(), entry, cmd_tx)),
        );
        cmd_rx.blocking_recv().unwrap_or(false)
    }
}

impl Hash for CronTagged {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.tag.hash(state);
        self.when.hash(state);
    }
}

/// Add a crontab to the cron queue.
pub(crate) fn cron_queue_add(
    app_name: &ApplicationName,
    entry: CronTagged,
    retrigger: bool,
) -> bool {
    CRON_INTERNAL_STATE.add_crontab(app_name, entry, retrigger)
}

/// List crontabs from the cron queue.
pub(crate) fn cron_queue_ls(
    app_name: &ApplicationName,
    tag: Option<CronEventTag>,
) -> Vec<(CronTagged, bool)> {
    CRON_INTERNAL_STATE.ls_crontabs(app_name, tag)
}

/// Delay a crontab in the cron queue.
pub(crate) fn cron_queue_delay(
    app_name: &ApplicationName,
    duration: Instant,
    tag: CronEventTag,
) -> wasmtime::Result<bool> {
    CRON_INTERNAL_STATE.delay_crontab(app_name, duration, tag)
}

/// Remove a crontab from the cron queue.
pub(crate) fn cron_queue_rm(
    app_name: &ApplicationName,
    entry: CronTagged,
) -> bool {
    CRON_INTERNAL_STATE.rm_crontab(app_name, entry)
}

/// Trigger the cron queue events dispatch.
pub(crate) fn cron_queue_trigger() -> anyhow::Result<()> {
    CRON_INTERNAL_STATE.cron_queue.trigger()
}

/// Send event to the Hermes Event Queue.
pub(crate) fn send_hermes_on_cron_event(
    app_name: &ApplicationName,
    on_cron_event: OnCronEvent,
) -> anyhow::Result<()> {
    //
    let event = HermesEvent::new(
        on_cron_event,
        TargetApp::List(vec![app_name.clone()]),
        TargetModule::All,
    );
    send(event)
}

/// The crontab queue task runs in the background.
async fn cron_queue_task(mut queue_rx: mpsc::Receiver<CronJob>) {
    while let Some(cron_job) = queue_rx.recv().await {
        match cron_job {
            CronJob::Add(app_name, on_cron_event, response_tx) => {
                handle_add_cron_job(app_name, on_cron_event, response_tx);
                // Trigger the cron queue
                if let Err(_err) = cron_queue_trigger() {
                    // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
                }
            },
            CronJob::List(app_name, tag, response_tx) => {
                handle_ls_cron_job(&app_name, tag.as_ref(), response_tx);
            },
            CronJob::Delay(app_name, cron_job_delay, response_tx) => {
                handle_delay_cron_job(app_name, cron_job_delay, response_tx);
                // Trigger the cron queue
                if let Err(_err) = cron_queue_trigger() {
                    // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
                }
            },
            CronJob::Remove(app_name, cron_tagged, response_tx) => {
                handle_rm_cron_job(&app_name, &cron_tagged, response_tx);
                // Trigger the cron queue
                if let Err(_err) = cron_queue_trigger() {
                    // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
                }
            },
        }
    }
}

/// Handle the `CronJob::Remove` command.
fn handle_rm_cron_job(
    app_name: &ApplicationName,
    cron_tagged: &CronTagged,
    response_tx: oneshot::Sender<bool>,
) {
    let response = CRON_INTERNAL_STATE
        .cron_queue
        .rm_event(app_name, cron_tagged);
    if let Err(_err) = response_tx.send(response) {
        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
    }
}

/// Handle the `CronJob::Add` command.
fn handle_add_cron_job(
    app_name: ApplicationName,
    on_cron_event: OnCronEvent,
    response_tx: oneshot::Sender<bool>,
) {
    // Check if the event will trigger by getting the next immediate timestamp.
    let response = if let Some(timestamp) = on_cron_event.tick_from(None) {
        // add the event to the queue
        CRON_INTERNAL_STATE
            .cron_queue
            .add_event(app_name, timestamp, on_cron_event);
        true
    } else {
        // The event will not trigger any timestamp. This can happen when setting
        // impossible combinations such as `day = 31` and `month = 2`, since it is not possible
        // for February to have 31 days.

        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
        // debug!("Event will not trigger: {on_cron_event:?}");
        false
    };
    if let Err(_err) = response_tx.send(response) {
        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
        // error!("Error sending response to `handle_add_cron_job`: {_err:?}");
    }
}

/// Handle the `CronJob::List` command.
fn handle_ls_cron_job(
    app_name: &ApplicationName,
    cron_tagged: Option<&CronEventTag>,
    response_tx: oneshot::Sender<Vec<(CronTagged, bool)>>,
) {
    let response = CRON_INTERNAL_STATE
        .cron_queue
        .ls_events(app_name, cron_tagged);
    if let Err(_err) = response_tx.send(response) {
        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
    }
}

/// Handle the `CronJob::Delay` command.
fn handle_delay_cron_job(
    app_name: ApplicationName,
    CronJobDelay { timestamp, event }: CronJobDelay,
    response_tx: oneshot::Sender<bool>,
) {
    CRON_INTERNAL_STATE
        .cron_queue
        .add_event(app_name, timestamp, event);
    let response = true;
    if let Err(_err) = response_tx.send(response) {
        // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use chrono::Datelike;

    use super::*;
    use crate::runtime_extensions::hermes::cron::tests::hermes_app_name;

    const APP_NAME: &str = "test";

    // triggers every minute, three days from now
    fn crontab_future_dow(
        tag: &str,
        days_from_now: i64,
    ) -> CronTagged {
        #[allow(clippy::arithmetic_side_effects)] // Ok in tests.
        let dow = (chrono::Utc::now() + chrono::TimeDelta::try_days(days_from_now).unwrap())
            .weekday()
            .number_from_monday();
        CronTagged {
            when: format!("* * * * {dow}"),
            tag: tag.into(),
        }
    }
    // triggers every minute, three days from now
    fn crontab_example_1() -> CronTagged {
        crontab_future_dow("Example1", 3)
    }
    // triggers every minute, four days from now
    fn crontab_example_2() -> CronTagged {
        crontab_future_dow("Example2", 4)
    }
    // triggers every minute, two days from now
    fn crontab_example_3() -> CronTagged {
        crontab_future_dow("Example3", 2)
    }
    // triggers every minute, two days from now
    fn crontab_other_1() -> CronTagged {
        crontab_future_dow("Other1", 2)
    }
    const RETRIGGER_YES: bool = true;
    const RETRIGGER_NO: bool = false;
    const IS_LAST: bool = true;
    const IS_NOT_LAST: bool = false;

    #[test]
    fn test_internal_state_with_no_tokio_task() {
        // start the state without a cron queue task thread
        let state = InternalState::new(None);
        let hermes_app = hermes_app_name(APP_NAME);

        // Add returns false
        assert!(!state.add_crontab(&hermes_app, crontab_example_1(), RETRIGGER_YES));
        // List returns empty vec.
        assert!(state.ls_crontabs(&hermes_app, None).is_empty());
        // Delay returns false
        assert!(!state
            .delay_crontab(&hermes_app, 0, "test".to_string())
            .unwrap());
        // Remove returns false
        assert!(!state.rm_crontab(&hermes_app, CronTagged {
            when: "*".to_string(),
            tag: "test".to_string()
        }));
    }

    #[test]
    fn test_cron_state() {
        let state = &CRON_INTERNAL_STATE;
        let hermes_app = hermes_app_name(APP_NAME);
        // Initial state for any `HermesAppName` is always empty
        assert!(state.ls_crontabs(&hermes_app, None).is_empty());
    }

    #[test]
    // **NOTE**: in order to test the `cron_queue_*` functions,
    // custom `CronTagged`s are used, by setting the `dow` to be at least 2 days from now
    // and never more than 6 days ahead. This way, the events won't be dispatched for the
    // duration of the test.
    fn test_cron_state_multi_thread_add_and_list_and_delay_and_remove_crontabs_without_triggering()
    {
        let app_name = hermes_app_name(APP_NAME);
        // Initial state for any `HermesAppName` is always empty
        assert!(cron_queue_ls(&app_name, None).is_empty());

        // inserting returns true
        assert!(cron_queue_add(
            &hermes_app_name(APP_NAME),
            crontab_example_1(),
            RETRIGGER_YES
        ));

        // inserting separate thread
        let h = std::thread::spawn(move || {
            let app_name = hermes_app_name(APP_NAME);
            cron_queue_add(&app_name, crontab_example_1(), RETRIGGER_NO)
        });
        assert!(h.join().unwrap());

        let queue_ls = cron_queue_ls(&app_name, None);
        assert!(queue_ls.contains(&(crontab_example_1(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_example_1(), IS_LAST)));

        assert!(cron_queue_add(
            &app_name,
            crontab_example_2(),
            RETRIGGER_YES
        ));

        let h = std::thread::spawn(move || {
            let app_name = hermes_app_name(APP_NAME);
            cron_queue_add(&app_name.clone(), crontab_example_2(), RETRIGGER_YES)
        });
        assert!(h.join().unwrap());

        let h = std::thread::spawn(move || {
            let app_name = hermes_app_name(APP_NAME);
            cron_queue_ls(&app_name.clone(), None)
        });
        let queue_ls = h.join().unwrap();
        assert!(queue_ls.contains(&(crontab_example_1(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_example_1(), IS_LAST)));
        assert!(queue_ls.contains(&(crontab_example_2(), IS_NOT_LAST)));

        assert!(cron_queue_add(
            &app_name,
            crontab_example_3(),
            RETRIGGER_YES
        ));
        assert!(cron_queue_add(&app_name, crontab_other_1(), RETRIGGER_YES));

        // List
        let queue_ls = cron_queue_ls(&app_name, None);
        assert!(queue_ls.contains(&(crontab_example_1(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_example_3(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_other_1(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_example_1(), IS_NOT_LAST)));
        assert!(queue_ls.contains(&(crontab_example_1(), IS_LAST)));
        assert!(queue_ls.contains(&(crontab_example_2(), IS_NOT_LAST)));

        // Adding a delayed crontab from another thread
        let h = std::thread::spawn(move || {
            let duration = 3_600_000_000_000;
            let delayed_tag = "Delayed1".to_string();
            let CronJobDelay {
                timestamp: _,
                event,
            } = mkdelay_crontab(duration, delayed_tag.clone()).unwrap();
            assert!(
                cron_queue_delay(&hermes_app_name(APP_NAME), duration, delayed_tag.clone())
                    .unwrap()
            );

            CronTagged {
                when: event.tag.when.clone(),
                tag: delayed_tag,
            }
        });
        let expected_crontagged = h.join().unwrap();

        assert!(cron_queue_ls(&app_name, None).contains(&(expected_crontagged.clone(), IS_LAST)));

        // Start clearing the queue in current thread
        assert!(cron_queue_rm(&app_name, expected_crontagged));

        // Remove everything else from another thread
        let h = std::thread::spawn(move || {
            let app_name = hermes_app_name(APP_NAME);
            assert!(cron_queue_rm(&app_name, crontab_example_1()));
            assert!(cron_queue_rm(&app_name, crontab_example_2()));
            assert!(cron_queue_rm(&app_name, crontab_example_3()));
            assert!(cron_queue_rm(&app_name, crontab_other_1()));
        });
        h.join().unwrap();

        // Run final test in another thread
        let h = std::thread::spawn(move || {
            let app_name = hermes_app_name(APP_NAME);
            assert!(cron_queue_ls(&app_name, None).is_empty());
        });
        h.join().unwrap();
    }
}
