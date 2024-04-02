use std::fmt::{Display, Formatter, write};
use crate::client::ClientFn;
use crate::gui::Message;
use crate::gui::Message::LedgerClientMsg;
use std::time::Duration;
use ledger_transport_hidapi::TransportNativeHID;
use crate::ledger_lib::{bitcoin_app, BitcoinAppV2, DeviceInfo, InstalledApp, list_installed_apps};
use crate::ledger_manager::{device_info, install_app, ledger_api};


use crate::listener;

listener!(LedgerListener, LedgerMessage, Message, LedgerClientMsg);

#[derive(Debug, Clone)]
pub enum Version{
    Installed(String),
    NotInstalled,
    None,
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Installed(version) => { write!(f, "{}", version)}
            Version::NotInstalled => {write!(f, "Not installed!")}
            Version::None => {write!(f, "???")}
        }
    }
}

#[derive(Debug, Clone)]
pub enum Model{
    NanoS,
    NanoSP,
    NanoX,
    Unknown,
}

impl Display for Model {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::NanoS => { write!(f, "Nano S") }
            Model::NanoSP => { write!(f, "Nano S+") }
            Model::NanoX => { write!(f, "Nano X") }
            _ => {write!(f, "")}
        }
    }
}
    
#[allow(unused)]
#[derive(Debug, Clone)]
pub enum LedgerMessage {
    UpdateMain,
    InstallMain,
    UpdateTest,
    InstallTest,
    TryConnect,

    Connected(Option<String>, Option<String>),
    MainAppVersion(Version),
    MainAppNextVersion(Version),
    TestAppVersion(Version),
    TestAppNextVersion(Version),
    DisplayMessage(String, bool),
}

#[allow(unused)]
pub struct LedgerClient {
    sender: Sender<LedgerMessage>,
    receiver: Receiver<LedgerMessage>,
    loopback: Sender<LedgerMessage>,
    device_version: Option<String>,
    mainnet_version: Version,
    testnet_version: Version,
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

    fn handle_message(&mut self, msg: LedgerMessage) {
        match msg {
            LedgerMessage::TryConnect => {
                self.poll_later();
                self.poll();
            }
            LedgerMessage::UpdateMain => self.update_main(),
            LedgerMessage::InstallMain => self.install_main(),
            LedgerMessage::UpdateTest => self.update_test(),
            LedgerMessage::InstallTest => self.install_test(),
            _ => {}
        }
    }

    fn poll_later(&self) {
        let loopback = self.loopback.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if loopback.send(LedgerMessage::TryConnect).await.is_err() {
                log::debug!("Fail to send Message")
            };
        });
    }

    fn poll(&mut self) {
        log::info!("Try to poll device...");
        if let Some(transport) = self.connect() {
            let mut device_version: Option<String> = None;

            let info = match device_info(&transport) {
                Ok(info) => {
                    log::info!("Device connected");
                    log::debug!("Device version: {}", info.version.clone());
                    if self.device_version.is_none() {
                        self.send_to_gui(LedgerMessage::Connected(
                            Some("Ledger".to_string()),
                            Some(info.version.clone())
                        ));
                    }
                    device_version = Some(info.version.clone());
                    Some(info)
                }
                Err(e) => {
                    log::debug!("Failed connect device: {}", &e);
                    self.display_message(&e, true);
                    None
                }
            };

            if let Some(info) = info {
                // if it's our first connection, we check the if apps are installed & version
                if self.device_version.is_none() && device_version.is_some() {
                    if let Ok((main_installed, test_installed)) = self.check_apps_installed(&transport) {
                        // get the mainnet app version name
                        let (main_model, main_version) = if main_installed {
                            match self.get_app_version(&info, true) {
                                Ok((model, version)) => { (model, version)}
                                Err(e) => {
                                    self.display_message(&e, false);
                                    (Model::Unknown, Version::None)
                                }
                            }
                        } else {
                            log::debug!("Mainnet app not installed!");
                            (Model::Unknown, Version::NotInstalled)
                        };

                        // get the testnet app version name
                        let (test_model, test_version) = if test_installed {
                            match self.get_app_version(&info, true) {
                                Ok((model, version)) => { (model, version)}
                                Err(e) => {
                                    self.display_message(&e, false);
                                    (Model::Unknown, Version::None)
                                }
                            }
                        } else {
                            log::debug!("Testnet app not installed!");
                            (Model::Unknown, Version::NotInstalled)
                        };

                        let model = match (&main_model, &test_model) {
                            (Model::Unknown, _) => {test_model}
                            _ => {main_model}
                        };
                        self.send_to_gui(LedgerMessage::Connected(Some(model.to_string()), device_version.clone()));
                        self.mainnet_version = main_version;
                        self.testnet_version = test_version;
                        self.update_apps_version();
                    }
                }
                self.device_version = device_version;
            }

        } else {
            self.send_to_gui(LedgerMessage::Connected(None, None));
            log::debug!("No transport");
        }

    }

    fn connect(&self) -> Option<TransportNativeHID> {
        if let Some(api) = &ledger_api().ok() {
            TransportNativeHID::new(api).ok()
        } else { None }
    }

    fn check_apps_installed(&mut self, transport: &TransportNativeHID) -> Result<(bool, bool), ()> {
        self.display_message("Querying installed applications from your Ledger. You might have to confirm on your device.", false);
        let mut mainnet = false;
        let mut testnet = false;
        match list_installed_apps(transport) {
            Ok(apps) => {
                log::debug!("List installed apps:");
                for app in apps {
                    log::debug!("  [{}]", &app.name);
                    if app.name == "Bitcoin" {
                        log::debug!("Mainnet installed");
                        mainnet = true
                    }
                    if app.name == "Bitcoin Test" {
                        log::debug!("Testnet installed");
                        testnet = true
                    }
                }
            }
            Err(e) => {
                log::debug!("Error listing installed applications: {}.", e);
                self.send_to_gui(LedgerMessage::DisplayMessage(
                    format!("Error listing installed applications: {}.", e),
                    true
                ));
                return Err(());
            }
        }
        if mainnet {
            log::debug!("Mainnet App installed");
        }
        if testnet {
            log::debug!("Testnet App installed");
        }

        Ok((mainnet, testnet))
    }

    fn get_app_version(&mut self, info: &DeviceInfo, testnet: bool) -> Result<(Model, Version), String> {
        match bitcoin_app(info, testnet) {
            Ok(r) => {
                // example for nano s
                // BitcoinAppV2 { version_name: "Bitcoin Test", perso: "perso_11", delete_key: "nanos/2.1.0/bitcoin_testnet/app_2.2.1_del_key", firmware: "nanos/2.1.0/bitcoin_testnet/app_2.2.1", firmware_key: "nanos/2.1.0/bitcoin_testnet/app_2.2.1_key", hash: "7f07efc20d96faaf8c93bd179133c88d1350113169da914f88e52beb35fcdd1e" }
                // example for nano s+
                // BitcoinAppV2 { version_name: "Bitcoin Test", perso: "perso_11", delete_key: "nanos+/1.1.0/bitcoin_testnet/app_2.2.0-beta_del_key", firmware: "nanos+/1.1.0/bitcoin_testnet/app_2.2.0-beta", firmware_key: "nanos+/1.1.0/bitcoin_testnet/app_2.2.0-beta_key", hash: "3c6d6ebebb085da948c0211434b90bc4504a04a133b8d0621aa0ee91fd3a0b4f" }
                if let Some(app) = r {
                    let chunks: Vec<&str> = app.firmware
                        .split('/')
                        .collect();
                    let model = chunks.first().map(|m| m.to_string());
                    let version = chunks.last().map(|m| m.to_string());
                    if let (Some(model), Some(version)) = (model, version) {
                        let model = if model == "nanos" {Model::NanoS} 
                        else if model == "nanos+" {Model::NanoSP}
                        else if model == "nanox" {Model::NanoX}
                        else {Model::Unknown};
                        
                        let version = if version.contains("app_") {
                            version.replace("app_", "")
                        } else {
                            version
                        };
                        
                        let version = Version::Installed(version);
                        if testnet {
                            log::debug!("Testnet Model{}, Version{}", model.clone(), version.clone());
                        }else {
                            log::debug!("Mainnet Model{}, Version{}", model.clone(), version.clone());
                        }
                        Ok((model, version))
                    } else {
                        Err(format!("Failed to parse  model/version in {:?}", chunks))
                    }
                    
                } else {
                    log::debug!("Fail to get version info");
                    Err("Fail to get version info".to_string())
                }
            }
            Err(e) => {
                log::debug!("Fail to get version info: {}",e);
                Err(format!("Fail to get version info: {}",e))
            }
        }
    }

    fn update_apps_version(&self) {
        match &self.mainnet_version {
            Version::None => {}
            _ => {self.send_to_gui(LedgerMessage::MainAppVersion(self.mainnet_version.clone()));}
        }
        match &self.testnet_version {
            Version::None => {}
            _ => {self.send_to_gui(LedgerMessage::TestAppVersion(self.testnet_version.clone()));}
        }

    }

    fn install(&mut self, testnet: bool) {
        self.send_to_gui(LedgerMessage::MainAppVersion(Version::None));
        self.send_to_gui(LedgerMessage::TestAppVersion(Version::None));
        
        if let Some(api) = self.connect() {
            install_app(&api, testnet);
        }
        
        self.device_version = None;
        self.poll();
        
    }

    fn install_main(&mut self) {
        self.install(true);
    }

    fn update_main(&mut self) {
        self.install_main()
    }

    fn install_test(&mut self) {
        self.install(false);
    }

    fn update_test(&mut self) {
        self.install_test()
    }

    fn display_message(&mut self, msg: &str, reset_button: bool) {
        self.send_to_gui(LedgerMessage::DisplayMessage(msg.to_string(), reset_button));
    }

}

impl ClientFn<LedgerMessage, Sender<LedgerMessage>> for LedgerClient {
    fn new(
        sender: Sender<LedgerMessage>,
        receiver: Receiver<LedgerMessage>,
        loopback: Sender<LedgerMessage>,
    ) -> Self {
        LedgerClient {
            sender,
            receiver,
            loopback,
            device_version: None,
            mainnet_version: Version::None,
            testnet_version: Version::None,
        }
    }

    async fn run(&mut self) {
        self.poll();
        self.poll_later();
        loop {
            if let Ok(msg) = self.receiver.try_recv() {
                self.handle_message(msg);
            }

            tokio::time::sleep(Duration::from_nanos(1)).await;
        }
    }
}
