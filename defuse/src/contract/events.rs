use std::mem;

use defuse_nep245::{MtBurnEvent, MtEvent};

#[derive(Debug, Default)]
pub struct PostponedMtBurnEvents(Vec<MtBurnEvent<'static>>);

impl PostponedMtBurnEvents {
    pub fn mt_burn(&mut self, event: MtBurnEvent<'static>) {
        self.0.push(event);
    }

    pub fn flush(&mut self) {
        let events = mem::take(&mut self.0);
        if events.is_empty() {
            return;
        }
        MtEvent::MtBurn(events.into()).emit();
    }
}

impl Drop for PostponedMtBurnEvents {
    fn drop(&mut self) {
        self.flush();
    }
}
