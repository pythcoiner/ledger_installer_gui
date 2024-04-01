mod client;
mod gui;
mod ledger;

use iced::{Application, Settings};

use crate::client::ClientFn;
use crate::gui::{Flags, LedgerInstaller};
use crate::ledger::LedgerClient;

#[tokio::main]
async fn main() {
    let (ledger_sender, gui_ledger_receiver) = async_channel::unbounded();
    let (gui_ledger_sender, ledger_receiver) = async_channel::unbounded();

    let flags = Flags {
        ledger_sender: gui_ledger_sender.clone(),
        ledger_receiver: gui_ledger_receiver,
    };

    let ledger = LedgerClient::new(ledger_sender, ledger_receiver, gui_ledger_sender);
    ledger.start();

    let settings = Settings::with_flags(flags);
    LedgerInstaller::run(settings).expect("")
}
