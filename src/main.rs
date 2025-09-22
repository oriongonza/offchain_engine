pub mod input;

use crate::input::{ClientId, Money, Tx, TxId, TxType, get_transactions};

use std::{
    collections::HashMap,
    env::args_os,
    fs::{self, File},
    io::{self, Write},
    mem,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Instant,
};

use anyhow::{Context, bail};
use serde::Serialize;

type AccountMap = HashMap<ClientId, Account>;

fn write_output(map: AccountMap, w: impl Write) {
    let mut w = csv::Writer::from_writer(w);
    for (client_id, account) in map {
        let account = FinalAccount::from_account(client_id, account);
        if let Err(e) = w.serialize(account) {
            // At this point let's not stop the execution.
            eprintln!("Error writing record to output: {e:?}");
        }
    }
}

#[derive(Default)]
struct Account {
    available: Money,
    held: Money,
    locked: bool,
    txs: HashMap<TxId, Tx>,
}

#[derive(Serialize)]
struct FinalAccount {
    client: ClientId,
    available: f32,
    held: f32,
    total: f32,
    locked: bool,
}

impl FinalAccount {
    fn from_account(
        client_id: ClientId,
        Account {
            available,
            held,
            locked,
            ..
        }: Account,
    ) -> Self {
        Self {
            client: client_id,
            available: available as f32 / 1000.0,
            held: held as f32 / 1000.0,
            total: (available + held) as f32 / 1000.0,
            locked,
        }
    }
}

impl Account {
    fn process_tx(&mut self, tx: Tx) -> anyhow::Result<()> {
        if self.locked {
            bail!("Attempted transaction for locked self.")
        }

        match tx.tx_type {
            TxType::Deposit(money) => {
                self.available += money;
                self.txs.insert(tx.tx_id, tx);
            }
            TxType::Withdrawal(money) => {
                let new_balance = self.available - money;
                if new_balance < 0 {
                    bail!(
                        "Attempted to withdraw more ({money}) than current balance {balance}, to a result of {new_balance}",
                        balance = self.available
                    );
                }
                self.available = new_balance;
                self.txs.insert(tx.tx_id, tx);
            }
            TxType::Dispute => {
                let old_tx = self.past_tx(&tx)?;
                if old_tx.disputed {
                    bail!("tx already disputed");
                }
                let TxType::Withdrawal(money) = old_tx.tx_type else {
                    bail!("invalid tx type: {:?}", old_tx.tx_type)
                };

                old_tx.disputed = true;
                self.held += money;
                // Assuming we can go into the negative in this case
                self.available -= money;
            }
            TxType::Resolve => {
                let old_tx = self.past_tx(&tx)?;
                if !old_tx.disputed {
                    bail!("tx {} not disputed", old_tx.tx_id);
                }
                let TxType::Withdrawal(money) = old_tx.tx_type else {
                    bail!("invalid tx type: {:?}", old_tx.tx_type)
                };

                self.held -= money;
                self.available += money;
            }
            TxType::Chargeback => {
                let old_tx = self.past_tx(&tx)?;
                if !old_tx.disputed {
                    bail!("tx {} not disputed", old_tx.tx_id);
                }
                let TxType::Withdrawal(money) = old_tx.tx_type else {
                    bail!("invalid tx type: {:?}", old_tx.tx_type)
                };
                self.held -= money;
                self.locked = true;
            }
        };

        Ok(())
    }

    fn past_tx(&mut self, tx: &Tx) -> anyhow::Result<&mut Tx> {
        self.txs
            .get_mut(&tx.tx_id)
            .with_context(|| format!("Transaction {} does not exist.", tx.tx_id))
    }
}

fn make_thread() -> (
    mpsc::Sender<Vec<Tx>>,
    JoinHandle<HashMap<ClientId, Account>>,
) {
    let (sender, recv) = mpsc::channel::<Vec<Tx>>();
    let h = thread::spawn(move || {
        let mut clients: AccountMap = HashMap::new();
        while let Ok(batch) = recv.recv() {
            for tx in batch {
                let account = clients.entry(tx.client_id).or_default();
                if let Err(e) = account.process_tx(tx) {
                    eprintln!("{e:?}");
                }
            }
        }

        clients
    });
    (sender, h)
}

fn process_all(file: File) -> anyhow::Result<AccountMap> {
    const BUF_SIZE: usize = 256; // Better throughput for larger datasets

    let t0 = Instant::now();
    let (sender, h) = make_thread();

    let mut vec = Vec::with_capacity(BUF_SIZE);
    for tx in get_transactions(file) {
        // We exit the whole operation on a read/deserializing error.
        // We could skip the record but that's bad, we don't want to skip operations.
        // It's better to fail and exit than to do process incomplete data.
        let tx = tx?;
        vec.push(tx);

        // PERF: This does not ever block because the other thread implementation is efficient.
        // The bottleneck is in reading + deserializing.
        if vec.len() == BUF_SIZE {
            // Unwrap safety: The channel cannot close until we call join.
            sender.send(mem::take(&mut vec)).unwrap();
            eprintln!("Sent transactions, t: {:?}", t0.elapsed());
        }
    }

    sender.send(vec).unwrap();

    eprintln!("Done deserializing after {:?}", t0.elapsed());

    drop(sender);

    // Unwrap safety:
    // I panic here to extend the panic.
    // It's not like the thread could panic, but it's still good practice to not swallow panics.
    let map = h.join().unwrap();

    eprintln!("finished processing after {:?}", t0.elapsed());

    Ok(map)
}

fn main() -> anyhow::Result<()> {
    let path = args_os()
        .nth(1)
        .context("Usage: `cargo run -- input.csv`")?;
    let file = fs::File::open(path)?;

    let map = process_all(file)?;
    let stdout = io::stdout().lock();
    write_output(map, stdout);

    Ok(())
}
