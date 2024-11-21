use std::sync::{Arc, Mutex};

use eframe::{
    egui::{self, CentralPanel, SidePanel, TopBottomPanel, ViewportBuilder},
    NativeOptions,
};
use rsnano_nullable_clock::SteadyClock;

use crate::{chat::ChatMessage, logic::RaffleLogic};

pub(crate) fn run_gui(logic: Arc<Mutex<RaffleLogic>>, clock: Arc<SteadyClock>) -> eframe::Result {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RsNano Spinner",
        options,
        Box::new(|_| {
            Ok(Box::new(SpinnerGui {
                logic,
                clock,
                input: String::new(),
            }))
        }),
    )
}

#[derive(Default)]
struct SpinnerGui {
    clock: Arc<SteadyClock>,
    logic: Arc<Mutex<RaffleLogic>>,
    input: String,
}

impl eframe::App for SpinnerGui {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after_secs(0.5);
        let mut logic = self.logic.lock().unwrap();

        SidePanel::right("viewers-panel")
            .exact_width(300.0)
            .resizable(false)
            .show(ctx, |ui| {
                let viewers = logic.registered_viewers();
                ui.heading(format!("Participants ({})", viewers.len()));
                for viewer in viewers {
                    ui.label(viewer.name);
                }
            });

        TopBottomPanel::top("timer-panel").show(ctx, |ui| {
            ui.heading(format!(
                "{}s until raffle",
                logic.countdown(self.clock.now()).as_secs()
            ));
        });

        TopBottomPanel::bottom("chat-input").show(ctx, |ui| {
            ui.text_edit_singleline(&mut self.input);
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                logic.handle_chat_message(ChatMessage {
                    message: self.input.clone(),
                    author_name: Some("APP USER".to_owned()),
                    author_channel_id: "APP".to_owned(),
                });
                self.input = String::new();
            }
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
