use defuse_nep245::MtBurnEvent;

use crate::contract::events::PostponedMtBurnEvents;

#[derive(Debug, Default)]
pub struct Runtime {
    postponed_burns: PostponedMtBurnEvents,
}

impl Runtime {
    pub fn mt_burn(&mut self, event: MtBurnEvent<'static>) {
        self.postponed_burns.mt_burn(event);
    }
}
