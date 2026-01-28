use std::mem;

use defuse_core::Result;
use defuse_near_utils::UnwrapOrPanicError;
use defuse_nep245::{MtBurnEvent, MtEvent};

#[derive(Debug, Default)]
pub struct PostponedMtBurnEvents(Vec<MtBurnEvent<'static>>);

impl PostponedMtBurnEvents {
    pub fn mt_burn(&mut self, event: MtBurnEvent<'static>) {
        self.0.push(event);
    }

    pub fn flush(&mut self) -> Result<()> {
        let events = mem::take(&mut self.0);
        if events.is_empty() {
            return Ok(());
        }
        MtEvent::MtBurn(events.into()).check_refund()?.emit();
        Ok(())
    }
}

impl Drop for PostponedMtBurnEvents {
    fn drop(&mut self) {
        /// NOTE: it will only fail when the refund event for withrawal would exceed
        /// maximum event log size, this is to prevent panic in withdrawal resolution
        self.flush().unwrap_or_panic_display();
    }
}
