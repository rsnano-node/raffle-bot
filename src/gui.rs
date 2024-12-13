use crate::{chat_messages::ChatMessage, logic::RaffleLogic};
use eframe::{
    egui::{self, CentralPanel, IconData, SidePanel, TopBottomPanel, ViewportBuilder},
    NativeOptions,
};
use rsnano_nullable_clock::SteadyClock;
use std::sync::{Arc, Mutex};

pub(crate) fn run_gui(logic: Arc<Mutex<RaffleLogic>>, clock: Arc<SteadyClock>) -> eframe::Result {
    let icon_data = load_icon();

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        "RsNano Spinner",
        options,
        Box::new(|_| {
            Ok(Box::new(AdminGui {
                logic,
                clock,
                message: String::new(),
                user: String::new(),
            }))
        }),
    )
}

fn load_icon() -> IconData {
    let icon_image = image::open("assets/icon-256.png").expect("Unable to open icon PNG file");

    let width = icon_image.width();
    let height = icon_image.height();
    let icon_rgba8 = icon_image.into_rgba8().to_vec();

    IconData {
        rgba: icon_rgba8,
        width,
        height,
    }
}

#[derive(Default)]
struct AdminGui {
    clock: Arc<SteadyClock>,
    logic: Arc<Mutex<RaffleLogic>>,
    message: String,
    user: String,
}

impl eframe::App for AdminGui {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after_secs(0.5);
        let mut logic = self.logic.lock().unwrap();
        let now = self.clock.now();

        SidePanel::left("controls")
            .exact_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                if logic.running() {
                    if ui.button("stop").clicked() {
                        logic.stop();
                    }
                } else if ui.button("start").clicked() {
                    logic.start();
                }

                ui.label("User:");
                ui.text_edit_singleline(&mut self.user);
                ui.label("Message:");
                ui.text_edit_singleline(&mut self.message);
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    logic.handle_chat_message(ChatMessage {
                        message: self.message.clone(),
                        author_name: Some(self.user.clone()),
                        author_channel_id: self.user.clone(),
                    });
                    self.message = String::new();
                }

                if ui.button("add test users").clicked() {
                    for i in 1..5 {
                        for name in ["Alice", "Bob", "John", "Jane", "Tom"] {
                            let user = format!("{}{}", name, i);
                            logic.handle_chat_message(ChatMessage {
                            author_channel_id: user.clone(),
                            author_name: Some(user.clone()),
                            message:
                                "nano_1iawmcfwmmdyr7xmnordt71gpnhnao8rsk4nywq5khtmedocaj6bafk4fb8h"
                                    .to_owned(),
                        });
                        }
                    }
                }
                if ui.button("run raffle now").clicked() {
                    logic.run_raffle_now(now);
                }
                let connected = if logic.spinner_connected(now) {
                    "ONLINE"
                } else {
                    "OFFLINE"
                };
                ui.label(format!("Spinner {}", connected));
                if let Some(win) = logic.current_win() {
                    ui.label(format!("CURRENT WINNER: {}", win.winner));
                }
            });

        SidePanel::right("winners-panel")
            .exact_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                let winners = logic.winners();
                ui.heading(format!("Winners ({})", winners.len()));
                for winner in winners {
                    ui.label(winner);
                }
            });

        SidePanel::right("viewers-panel")
            .exact_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                let participants = logic.participants();
                ui.heading(format!("Participants ({})", participants.len()));
                for participant in participants {
                    ui.label(participant.name);
                }
            });

        TopBottomPanel::top("timer-panel").show(ctx, |ui| {
            ui.heading(format!("{}s until raffle", logic.countdown(now).as_secs()));
        });

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Chat");
            for msg in logic.latest_messages() {
                ui.label(format!(
                    "{}: {}",
                    msg.author_name.as_ref().map_or("no name", |i| i.as_str()),
                    msg.message
                ));
            }
        });
    }
}
