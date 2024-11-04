#[derive(serde::Deserialize, Debug, Clone, Copy, PartialOrd, PartialEq)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub transaction_id: u32,
    #[serde(rename = "amount")]
    pub amount: Option<f32>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}
