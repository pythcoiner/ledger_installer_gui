mod client;
mod gui;
mod ledger;
mod ledger_lib;
mod ledger_manager;

use crate::client::ClientFn;
use crate::gui::{Flags, LedgerInstaller};
use crate::ledger::LedgerClient;
use iced::{window::icon, Application, Settings, Size};
use chrono::Local;
use colored::Colorize;

#[tokio::main]
async fn main() {

    let verbose_log = true;
    fern::Dispatch::new()
        .format(move |out, message, record| {
            let color = match record.level() {
                log::Level::Error => "red",
                log::Level::Warn => "yellow",
                log::Level::Info => "green",
                log::Level::Debug => "blue",
                log::Level::Trace => "magenta",
            };

            let file = record.file();
            let line = record.line();
            let mut file_line = "".to_string();

            if let Some(f) = file {
                file_line = format!(":{}", f);
                if let Some(l) = line {
                    file_line = format!("{}:{}", file_line, l);
                }
            }
            let formatted = if verbose_log {
                format!(
                    "[{}][{}][{}] {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.target(),

                    record.level(),
                    message
                )
            } else {
                format!(
                    "[{}] {}",
                    record.level(),
                    message
                )
            };
            out.finish(format_args!("{}", formatted.color(color)))
        })
        .level(log::LevelFilter::Info)
        .level_for("bacca", log::LevelFilter::Debug)
        .level_for("ledger_transport_hidapi", log::LevelFilter::Error)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
    
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
