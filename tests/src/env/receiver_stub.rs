use multi_token_receiver_stub::MTReceiverMode;

#[derive(Debug, Clone)]
pub struct TransferCallExpectation {
    pub mode: MTReceiverMode,
    pub transfer_amount: Option<u128>,
    pub expected_sender_balance: u128,
    pub expected_receiver_balance: u128,
}
