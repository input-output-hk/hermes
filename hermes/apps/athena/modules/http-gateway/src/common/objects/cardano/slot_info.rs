//! Defines API schemas of Cardano Slot info types.

use derive_more::{From, Into};

use crate::common::{
    objects::cardano::hash::Hash256,
    types::{cardano::slot_no::SlotNo, generic::date_time::DateTime},
};

/// Cardano block's slot data.
#[allow(clippy::struct_field_names)]
pub(crate) struct Slot {
    /// Slot number.
    pub(crate) slot_number: SlotNo,

    /// Block hash.
    pub(crate) block_hash: Hash256,

    /// Block time.
    pub(crate) block_time: DateTime,
}

/// Previous slot info.
#[derive(From, Into)]
pub(crate) struct PreviousSlot(Slot);

/// Current slot info.
#[derive(From, Into)]
pub(crate) struct CurrentSlot(Slot);

/// Next slot info.
#[derive(From, Into)]
pub(crate) struct NextSlot(Slot);

/// Cardano follower's slot info.
pub(crate) struct SlotInfo {
    /// Previous slot info.
    pub(crate) previous: Option<PreviousSlot>,

    /// Current slot info.
    pub(crate) current: Option<CurrentSlot>,

    /// Next slot info.
    pub(crate) next: Option<NextSlot>,
}
