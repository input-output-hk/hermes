//! Host - Cron implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    hermes::cron::api::{CronEventTag, CronSched, CronTagged, CronTime, Host},
    wasi::clocks::monotonic_clock::Instant,
    HermesState, NewState,
};

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {
    #[doc = " # Schedule Recurrent CRON event"]
    #[doc = " "]
    #[doc = " Cron events will be delivered to the `on-cron` event handler."]
    #[doc = " "]
    #[doc = " ## Parameters"]
    #[doc = " "]
    #[doc = " - `entry`: The crontab entry to add."]
    #[doc = " - `when`: When the event triggers.  Standard crontab format."]
    #[doc = " - `tag`: A tag which will accompany the triggered event."]
    #[doc = " - `retrigger`:"]
    #[doc = " - `true`: The event will re-trigger every time the crontab entry matches until cancelled."]
    #[doc = " - `false`: The event will automatically cancel after it is generated once."]
    #[doc = " "]
    #[doc = " ## Returns"]
    #[doc = " "]
    #[doc = " - `true`: Crontab added successfully.  (Or the crontab event already exists)"]
    #[doc = " - `false`: Crontab failed to be added."]
    #[doc = " "]
    #[doc = " ## Note:"]
    #[doc = " "]
    #[doc = " If the crontab entry already exists, the retrigger flag can be changed by calling"]
    #[doc = " this function.  This could be useful where a retriggering crontab event is desired"]
    #[doc = " to be stopped, but ONLY after it has triggered once more."]
    fn add(&mut self, entry: CronTagged, retrigger: bool) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " # Schedule A Single cron event after a fixed delay."]
    #[doc = " "]
    #[doc = " Allows for easy timed wait events to be delivered without"]
    #[doc = " requiring datetime calculations or formatting cron entries."]
    #[doc = " "]
    #[doc = " ## Parameters"]
    #[doc = " "]
    #[doc = " - `duration`: How many nanoseconds to delay.  The delay will be AT LEAST this long."]
    #[doc = " - `tag`: A tag which will accompany the triggered event."]
    #[doc = " "]
    #[doc = " ## Returns"]
    #[doc = " "]
    #[doc = " - `true`: Crontab added successfully."]
    #[doc = " - `false`: Crontab failed to be added."]
    #[doc = " "]
    #[doc = " ## Note:"]
    #[doc = " "]
    #[doc = " This is a convenience function which will automatically calculate the crontab"]
    #[doc = " entry needed to trigger the event after the requested `duration`."]
    #[doc = " It is added as a non-retriggering event."]
    #[doc = " Listing the crontabs after this call will list the delay in addition to all other"]
    #[doc = " crontab entries."]
    fn delay(&mut self, duration: Instant, tag: CronEventTag) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " # List currently active cron schedule."]
    #[doc = " "]
    #[doc = " Allows for management of scheduled cron events."]
    #[doc = " "]
    #[doc = " ## Parameters"]
    #[doc = " "]
    #[doc = " - `tag`: Optional, the tag to limit the list to.  If `none` then all crons listed."]
    #[doc = " "]
    #[doc = " ## Returns"]
    #[doc = " "]
    #[doc = " - A list of tuples containing the scheduled crontabs and their tags, along with the current retrigger flag."]
    #[doc = " The list is sorted from most crontab that will trigger soonest to latest."]
    #[doc = " Crontabs are only listed once, in the case where a crontab may be scheduled"]
    #[doc = " may times before a later one."]
    #[doc = " - `0` - `cron-tagged` - The Tagged crontab event."]
    #[doc = " - `1` - `bool` - The state of the retrigger flag."]
    fn ls(&mut self, tag: Option<CronEventTag>) -> wasmtime::Result<Vec<(CronTagged, bool)>> {
        todo!()
    }

    #[doc = " # Remove the requested crontab."]
    #[doc = " "]
    #[doc = " Allows for management of scheduled cron events."]
    #[doc = " "]
    #[doc = " ## Parameters"]
    #[doc = " "]
    #[doc = " - `when`: The crontab entry to add.  Standard crontab format."]
    #[doc = " - `tag`: A tag which will accompany the triggered event."]
    #[doc = " "]
    #[doc = " ## Returns"]
    #[doc = " "]
    #[doc = " - `true`: The requested crontab was deleted and will not trigger."]
    #[doc = " - `false`: The requested crontab does not exist."]
    fn rm(&mut self, entry: CronTagged) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " # Make a crontab entry from individual time values."]
    #[doc = " "]
    #[doc = " Crates the properly formatted cron entry"]
    #[doc = " from numeric cron time components."]
    #[doc = " Convenience function to make building cron strings simpler when they are"]
    #[doc = " calculated from data."]
    #[doc = " "]
    #[doc = " ## Parameters"]
    #[doc = " "]
    #[doc = " - `dow` - DayOfWeek (0-7, 0 or 7 = Sunday)"]
    #[doc = " - `month` - Month of the year (1-12, 1 = January)"]
    #[doc = " - `day` - Day in the month (1-31)"]
    #[doc = " - `hour` - Hour in the day (0-23)"]
    #[doc = " - `minute` - Minute in the hour (0-59)"]
    #[doc = " "]
    #[doc = " ## Returns"]
    #[doc = " "]
    #[doc = " - A matching `cron-sched` ready for use in the cron functions above."]
    #[doc = " "]
    #[doc = " ## Note:"]
    #[doc = " No checking is done to determine if the requested date is valid."]
    #[doc = " If a particular component is out of its allowable range it will be silently"]
    #[doc = " clamped within the allowable range of each parameter."]
    #[doc = " Redundant entries will be removed."]
    #[doc = " - For example specifying a `month` as `3` and `2-4` will"]
    #[doc = " remove the individual month and only produce the range."]
    fn mkcron(
        &mut self, dow: CronTime, month: CronTime, day: CronTime, hour: CronTime, minute: CronTime,
    ) -> wasmtime::Result<CronSched> {
        todo!()
    }
}
