use crate::State;

impl State {
    pub(crate) const fn should_cleanup(&self) -> bool {
        self.closed
            && self.maker_src_remaining == 0
            && self.maker_dst_lost == 0
            && self.callbacks_in_flight == 0
    }

    // const fn start_cleanup(&mut self) -> bool {
    //     !mem::replace(&mut self.cleanup_in_progress, true)
    // }
}
