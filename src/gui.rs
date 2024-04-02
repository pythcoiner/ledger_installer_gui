use async_channel::{Receiver, Sender};
use iced::alignment::Horizontal;
use iced::widget::{container, Button, Column, Row, Space, Text};
use iced::{executor, Application, Element, Length, Theme};
use iced_runtime::futures::Subscription;
use iced_runtime::Command;

use crate::ledger::{LedgerListener, LedgerMessage, Version};

#[allow(unused)]
#[derive(Debug)]
pub struct Flags {
    pub(crate) ledger_sender: Sender<LedgerMessage>,
    pub(crate) ledger_receiver: Receiver<LedgerMessage>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Message {
    LedgerClientMsg(LedgerMessage),

    UpdateMain,
    InstallMain,
    UpdateTest,
    InstallTest,
    Connect,
}

#[allow(unused)]
pub struct LedgerInstaller {
    ledger_sender: Sender<LedgerMessage>,
    ledger_receiver: Receiver<LedgerMessage>,
    ledger_model: Option<String>,
    ledger_version: Option<String>,
    main_app_version: Version,
    main_next_version: Version,
    test_app_version: Version,
    test_next_version: Version,
}

impl LedgerInstaller {
    #[allow(unused)]
    pub fn send_ledger_msg(&self, msg: LedgerMessage) {
        let sender = self.ledger_sender.clone();
        tokio::spawn(async move { sender.send(msg).await });
    }
}

impl Application for LedgerInstaller {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(args: Self::Flags) -> (Self, Command<Self::Message>) {
        let escrow = LedgerInstaller {
            ledger_sender: args.ledger_sender,
            ledger_receiver: args.ledger_receiver,
            ledger_model: None,
            ledger_version: None,
            main_app_version: Version::None,
            main_next_version: Version::None,
            test_app_version: Version::None,
            test_next_version: Version::None,
        };

        (escrow, Command::none())
    }

    fn title(&self) -> String {
        "Ledger Installer".to_string()
    }

    fn update(&mut self, event: Message) -> Command<Message> {
        log::debug!("Gui receive: {:?}", event.clone());
        match event {
            Message::LedgerClientMsg(ledger) => match &ledger {
                LedgerMessage::Connected(model, version) => {
                    if model.is_none() {
                        self.main_app_version = Version::None;
                        self.main_next_version = Version::None;
                        self.test_app_version = Version::None;
                        self.test_next_version = Version::None;
                    }
                    self.ledger_model = model.clone();
                    self.ledger_version = version.clone();
                }
                LedgerMessage::MainAppVersion(version) => self.main_app_version = version.clone(),
                LedgerMessage::MainAppNextVersion(version) => {
                    self.main_next_version = version.clone()
                }
                LedgerMessage::TestAppVersion(version) => self.test_app_version = version.clone(),
                LedgerMessage::TestAppNextVersion(version) => {
                    self.test_next_version = version.clone()
                }
                LedgerMessage::DisplayMessage(s, _) => {log::info!("{}", s)}
                _ => {
                    log::debug!("Unhandled message from ledger: {:?}!", ledger)
                }
            },
            Message::UpdateMain => self.send_ledger_msg(LedgerMessage::UpdateMain),
            Message::InstallMain => self.send_ledger_msg(LedgerMessage::InstallMain),
            Message::UpdateTest => self.send_ledger_msg(LedgerMessage::UpdateTest),
            Message::InstallTest => self.send_ledger_msg(LedgerMessage::InstallMain),
            _ => {
                log::debug!("Unhandled message!")
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let model = match (&self.ledger_model, &self.ledger_version) {
            (Some(model), Some(version)) => {
                Text::new(format!("Model: {}            Version: {}", model, version))
            }
            (Some(model), None) => Text::new(format!("Model: {}  Version: unknown ", model)),
            _ => Text::new("Please connect a device..."),
        }
        .horizontal_alignment(Horizontal::Center);

        let main_app = app_row(
            "Bitcoin app",
            &self.main_app_version,
            Message::InstallMain,
            Message::UpdateMain,
        );

        let _connect: Row<Message> = Row::new()
            .push(Space::with_width(Length::Fill))
            .push({
                let mut connect = Button::new("Connect");
                // TODO: grey out button when processing update or install
                connect = connect.on_press(Message::Connect);
                connect
            })
            .push(Space::with_width(Length::Fill));

        let test_app = app_row(
            "Testnet app",
            &self.test_app_version,
            Message::InstallTest,
            Message::UpdateTest,
        );

        container(
            Column::new()
                .push(Space::with_height(Length::Fill))
                .push(
                    Row::new()
                        .push(Space::with_width(Length::Fill))
                        .push(model)
                        .push(Space::with_width(Length::Fill)),
                )
                .push(Space::with_height(10))
                .push_maybe(if self.ledger_model.is_some() {
                    Some(main_app)
                } else {
                    None
                })
                .push(Space::with_height(10))
                .push_maybe(if self.ledger_model.is_some() {
                    Some(test_app)
                } else {
                    None
                })
                .push(Space::with_height(Length::Fill)),
        )
        .into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::from_recipe(LedgerListener {
            receiver: self.ledger_receiver.clone(),
        })
    }
}

fn app_row<'a>(
    app_name: &'a str,
    version: &Version,
    update_msg: Message,
    install_msg: Message,
) -> Row<'a, Message> {
    
    let button_text = match version {
        Version::Installed(_) => {"Update".to_string()}
        Version::NotInstalled => {"Install".to_string()}
        Version::None => {"".to_string()}
    };
    let mut button = Button::new(
        Text::new(button_text)
            .size(11)
            .width(80)
            .horizontal_alignment(Horizontal::Center)
        );
    
    match version {
        Version::Installed(_) => {
            button = button.on_press(update_msg);
        }
        Version::NotInstalled => {
            button = button.on_press(install_msg);
        }
        Version::None => {}
    }
    
    Row::new()
        .push(Space::with_width(Length::Fill))
        .push(
            container(
                Row::new()
                    .push(Text::new(app_name))
                    .push(Space::with_width(Length::Fill))
                    .push(Text::new(version.to_string())),
            )
            .width(220),
        )
        .push(Space::with_width(15))
        .push(button)
        .push(Space::with_width(Length::Fill))
}
