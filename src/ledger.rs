use crate::client::ClientFn;
use crate::gui::Message;
use crate::gui::Message::LedgerClientMsg;
use std::time::Duration;

use crate::listener;

listener!(LedgerListener, LedgerMessage, Message, LedgerClientMsg);

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum LedgerMessage {
    UpdateMain,
    InstallMain,
    UpdateTest,
    InstallTest,
    TryConnect,
    
    Connected(Option<String>, Option<String>),
    MainAppVersion(String),
    MainAppNextVersion(String),
    TestAppVersion(String),
    TestAppNextVersion(String),
}

#[allow(unused)]
pub struct LedgerClient {
    sender: Sender<LedgerMessage>,
    receiver: Receiver<LedgerMessage>,
    loopback: Sender<LedgerMessage>,
    processing: bool,
}

impl LedgerClient {
    pub fn start(mut self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }
    
    #[allow(unused)]
    fn send_to_gui(&self, msg: LedgerMessage) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            if sender.send(msg).await.is_err() {
                log::debug!("Fail to send Message")
            };
        });
    }
    
    fn poll_later(&self) {
        let loopback = self.loopback.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if loopback.send(LedgerMessage::TryConnect).await.is_err() {
                log::debug!("Fail to send Message")
            };
        });
    }
    
    fn handle_message(&mut self, msg: LedgerMessage) {
        match msg {
            LedgerMessage::TryConnect => {
                self.poll_later();
                if !self.processing {self.try_connect();}
            }
            LedgerMessage::UpdateMain => {self.update_main()}
            LedgerMessage::InstallMain => {self.install_main()}
            LedgerMessage::UpdateTest => {self.update_test()}
            LedgerMessage::InstallTest => {self.install_test()}
            _ => {}
        }
    }
    
    fn try_connect(&self) {
        // TODO: try connect the ledger
        println!("Try to connect");
    }
    
    fn update_main(&self) {
        // TODO 
    }

    fn install_main(&self) {
        // TODO 
    }

    fn update_test(&self) {
        // TODO 
    }

    fn install_test(&self) {
        // TODO 
    }
}

impl ClientFn<LedgerMessage, Sender<LedgerMessage>> for LedgerClient {
    fn new(sender: Sender<LedgerMessage>, receiver: Receiver<LedgerMessage>, loopback: Sender<LedgerMessage>) -> Self {
        LedgerClient { sender, receiver, loopback, processing: false }
    }

    async fn run(&mut self) {
        if self.loopback.send(LedgerMessage::TryConnect).await.is_err() {
            log::debug!("Cannot start connect poller");
        }
        loop {
            if let Ok(msg) = self.receiver.try_recv() {
                self.handle_message(msg);
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
