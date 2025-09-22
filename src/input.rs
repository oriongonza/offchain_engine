use std::io::Read;

use anyhow::Context;
use serde::Deserialize;

// These Raw* types are useful temporarily, we massage them a bit more for type safety.

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawTxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// This is 1_000x larger to work with fixed point arithmetic.
pub type Money = i32;
// I don't know the dataset well enough, but it's possibly a good idea to use NonZeroU* instead.
pub type ClientId = u16;
pub type TxId = u32;

#[derive(Deserialize)]
struct RawTx {
    r#type: RawTxType,
    client: ClientId,
    tx: TxId,
    amount: Option<f32>,
}

// Parsed structs

#[derive(Debug, Clone, Copy)]
pub enum TxType {
    Deposit(Money),
    Withdrawal(Money),
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Copy)]
pub struct Tx {
    pub client_id: ClientId,
    pub tx_id: TxId,
    pub tx_type: TxType,
    pub disputed: bool,
}

impl TryFrom<RawTx> for Tx {
    type Error = anyhow::Error;

    fn try_from(
        RawTx {
            r#type,
            client,
            tx,
            amount,
        }: RawTx,
    ) -> Result<Self, Self::Error> {
        let tx_type = match r#type {
            RawTxType::Deposit => {
                let money = amount.context("deposit had no amount")?;
                TxType::Deposit((money * 1000.0) as i32)
            }
            RawTxType::Withdrawal => {
                let money = amount.context("deposit had no amount")?;
                TxType::Withdrawal((money * 1000.0) as i32)
            }
            RawTxType::Dispute => TxType::Dispute,
            RawTxType::Resolve => TxType::Resolve,
            RawTxType::Chargeback => TxType::Chargeback,
        };

        Ok(Self {
            client_id: client,
            tx_id: tx,
            tx_type,
            disputed: false,
        })
    }
}

pub fn get_transactions(file: impl Read) -> impl Iterator<Item = anyhow::Result<Tx>> {
    let reader = csv::ReaderBuilder::new()
        .flexible(false)
        .trim(csv::Trim::All)
        .from_reader(file);
    let records = reader.into_deserialize::<RawTx>();
    records.map(|x| x.context("CSV error").and_then(Tx::try_from))
}
