//! Cron host implementation for WASM runtime.

use dashmap::DashMap;

use crate::{
    runtime_extensions::{
        bindings::{
            hermes::cron::api::{CronEventTag, CronSched, CronTagged, CronTime, Host},
            wasi::clocks::monotonic_clock::Instant,
        },
        hermes::cron::{mkcron_impl, mkdelay_crontab, CronTab},
    },
    state::HermesState,
};

impl Host for HermesState {
    /// # Schedule Recurrent CRON event
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    ///
    /// ## Parameters
    ///
    /// - `entry`: The crontab entry to add.
    /// - `when`: When the event triggers.  Standard crontab format.
    /// - `tag`: A tag which will accompany the triggered event.
    /// - `retrigger`:
    /// - `true`: The event will re-trigger every time the crontab entry matches until
    ///   cancelled.
    /// - `false`: The event will automatically cancel after it is generated once.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.  (Or the crontab event already exists)
    /// - `false`: Crontab failed to be added.
    ///
    /// ## Note:
    ///
    /// If the crontab entry already exists, the retrigger flag can be changed by calling
    /// this function.  This could be useful where a retriggering crontab event is desired
    /// to be stopped, but ONLY after it has triggered once more.
    fn add(&mut self, entry: CronTagged, retrigger: bool) -> wasmtime::Result<bool> {
        let app_name = self.ctx.app_name();
        let tag = entry.tag.clone();
        let crontab = CronTab { entry, retrigger };
        if self.hermes.cron.crontabs.contains_key(app_name) {
            if let Some(app_cron) = self.hermes.cron.crontabs.get_mut(app_name) {
                app_cron.insert(tag, crontab);
            } else {
                let app_cron: DashMap<CronEventTag, CronTab> = DashMap::new();
                app_cron.insert(tag, crontab);
                self.hermes
                    .cron
                    .crontabs
                    .insert(app_name.to_string(), app_cron);
            }
        }

        Ok(true)
    }

    /// # Schedule A Single cron event after a fixed delay.
    ///
    /// Allows for easy timed wait events to be delivered without
    /// requiring datetime calculations or formatting cron entries.
    ///
    /// ## Parameters
    ///
    /// - `duration`: How many nanoseconds to delay.  The delay will be AT LEAST this
    ///   long.
    /// - `tag`: A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.
    /// - `false`: Crontab failed to be added.
    ///
    /// ## Note:
    ///
    /// This is a convenience function which will automatically calculate the crontab
    /// entry needed to trigger the event after the requested `duration`.
    /// It is added as a non-retriggering event.
    /// Listing the crontabs after this call will list the delay in addition to all other
    /// crontab entries.
    fn delay(&mut self, duration: Instant, tag: CronEventTag) -> wasmtime::Result<bool> {
        let crontab = mkdelay_crontab(duration, tag)?;
        self.add(crontab, false)?;
        Ok(true)
    }

    /// # List currently active cron schedule.
    ///
    /// Allows for management of scheduled cron events.
    ///
    /// ## Parameters
    ///
    /// - `tag`: Optional, the tag to limit the list to.  If `none` then all crons listed.
    ///
    /// ## Returns
    ///
    /// - A list of tuples containing the scheduled crontabs and their tags, along with
    ///   the current retrigger flag.
    /// The list is sorted from most crontab that will trigger soonest to latest.
    /// Crontabs are only listed once, in the case where a crontab may be scheduled
    /// may times before a later one.
    /// - `0` - `cron-tagged` - The Tagged crontab event.
    /// - `1` - `bool` - The state of the retrigger flag.
    fn ls(&mut self, tag: Option<CronEventTag>) -> wasmtime::Result<Vec<(CronTagged, bool)>> {
        let app_name = self.ctx.app_name();
        if self.hermes.cron.crontabs.contains_key(app_name) {
            if let Some(app_cron) = self.hermes.cron.crontabs.get_mut(app_name) {
                //
                if let Some(tag) = tag {
                    match app_cron.get(&tag) {
                        Some(cron) => Ok(vec![(cron.entry.clone(), cron.retrigger)]),
                        None => Ok(vec![]),
                    }
                } else {
                    Ok(app_cron
                        .iter()
                        .map(|cron| (cron.entry.clone(), cron.retrigger))
                        .collect())
                }
            } else {
                //
                Ok(vec![])
            }
        } else {
            //
            Ok(vec![])
        }
    }

    /// # Remove the requested crontab.
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
    fn rm(&mut self, entry: CronTagged) -> wasmtime::Result<bool> {
        let app_name = self.ctx.app_name();
        if self.hermes.cron.crontabs.contains_key(app_name) {
            if let Some(app_cron) = self.hermes.cron.crontabs.get_mut(app_name) {
                match app_cron.remove(&entry.tag) {
                    Some(_) => Ok(true),
                    None => Ok(false),
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// # Make a crontab entry from individual time values.
    ///
    /// Creates the properly formatted cron entry
    /// from numeric cron time components.
    /// Convenience function to make building cron strings simpler when they are
    /// calculated from data.
    ///
    /// ## Parameters
    ///
    /// - `dow` - `DayOfWeek` (0-7, 0 or 7 = Sunday)
    /// - `month` - Month of the year (1-12, 1 = January)
    /// - `day` - Day in the month (1-31)
    /// - `hour` - Hour in the day (0-23)
    /// - `minute` - Minute in the hour (0-59)
    ///
    /// ## Returns
    ///
    /// - A matching `cron-sched` ready for use in the cron functions above.
    ///
    /// ## Note:
    /// No checking is done to determine if the requested date is valid.
    /// If a particular component is out of its allowable range it will be silently
    /// clamped within the allowable range of each parameter.
    /// Redundant entries will be removed.
    /// - For example specifying a `month` as `3` and `2-4` will
    /// remove the individual month and only produce the range.
    fn mkcron(
        &mut self, dow: CronTime, month: CronTime, day: CronTime, hour: CronTime, minute: CronTime,
    ) -> wasmtime::Result<CronSched> {
        Ok(mkcron_impl(&dow, &month, &day, &hour, &minute))
    }
}
