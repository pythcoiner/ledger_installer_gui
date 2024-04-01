mod client;
mod gui;
mod ledger;

use iced::{Application, Settings, Size, window::icon};
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

    const ICON: &[u8] = include_bytes!("sardine.png");
    let icon = icon::from_file_data(ICON, None).unwrap();

    let mut settings = Settings::with_flags(flags);
    settings.window.size = Size::new(500.0, 200.0);
    settings.window.resizable = false;
    settings.window.icon = Some(icon);
    
    LedgerInstaller::run(settings).expect("")
}
