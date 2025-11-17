use near_sdk::Gas;

use crate::{Error, Params, Result};

use super::tokens::Sendable;

impl Params {
    pub(super) fn validate_gas(&self) -> Result<()> {
        // mt_on_transfer() with p256 signature validation
        const MAX_FILL_GAS: Gas = Gas::from_tgas(300 - 30 - 10);

        self.required_gas_to_fill()
            .is_some_and(|total| total <= MAX_FILL_GAS)
            .then_some(())
            .ok_or(Error::ExcessiveGas)
    }

    fn required_gas_to_fill(&self) -> Option<Gas> {
        const FILL_GAS: Gas = Gas::from_tgas(10);

        FILL_GAS
            .checked_add(self.dst_token.transfer_gas(
                self.receive_dst_to.min_gas,
                self.receive_dst_to.msg.is_some(),
            ))?
            .checked_add(
                self.src_token
                    .transfer_gas(self.refund_src_to.min_gas, self.refund_src_to.msg.is_some()),
            )?
            .checked_add(
                self.dst_token.transfer_gas(None, false).checked_mul(
                    self.integrator_fees
                        .values()
                        .copied()
                        .chain(self.protocol_fees.as_ref().map(|p| p.fee + p.surplus))
                        .filter(|fee| !fee.is_zero())
                        .count()
                        .try_into()
                        .unwrap_or_else(|_| unreachable!()),
                )?,
            )
    }
}
