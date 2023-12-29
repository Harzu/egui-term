use std::sync::mpsc::{sync_channel, Receiver};
use egui::{Vec2, Sense, Rect};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::time::sleep;

use crate::terminal;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
pub struct TemplateApp {
    term: terminal::Terminal,
    rx: Receiver<Vec<u8>>
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut term = terminal::Terminal::new("/bin/bash".to_string());
        let (tx,rx) = sync_channel(100);

        let mut reader = File::from(term.new_reader());
        let ctx = cc.egui_ctx.clone();

        tokio::spawn(async move {
            loop {
                let mut buf = [0; 4096];
                if let Ok(_) = reader.read(&mut buf).await {
                    println!("{:?}", buf);

                    tx.send(buf.to_vec()).unwrap();
                    ctx.request_repaint();
                };
                sleep(std::time::Duration::from_millis(1)).await;
            }
        });

        Self {
            term,
            rx,
        }
    }
}

impl eframe::App for TemplateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let font_witdth = 13.0;
        let font_height = 20.0;

        egui::CentralPanel::default().show(ctx, |ui| {
            let row_count = ((ui.available_height() as f32) / font_height).round() as u16 - 1;
            let col_count = ((ui.available_width() as f32) / font_witdth).round() as u16;
            self.term.resize(row_count, col_count);

            ui.input(|i| {
                if i.key_down(egui::Key::Enter) {
                    self.term.write_to_pty('\n');
                } else {
                    for e in &i.events {
                        if let egui::Event::Text(char) = e {
                            self.term.write_to_pty(char.as_bytes()[0] as char);
                        }
                    }
                }
            });

            if let Ok(data) = self.rx.try_recv() {
                self.term.update(data);
            }

            let (_, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
            let cells = self.term.cells();
            for cell in cells {
                let x = (cell.column as f32 + 1.0) * font_witdth;
                let cell_line = cell.line + cell.display_offset as i32;
                let y = (cell_line as f32 + 1.0) * font_height;

                let rect_pos = egui::pos2(x, y);
                let rect = Rect::from_min_size(rect_pos, Vec2::new(font_witdth, font_height));
                painter.rect_filled(rect, 0.0, cell.bg);

                let font_id = egui::FontId::default();
                let pos = egui::pos2(x, y); // Position of the text on the canvas
                painter.text(pos, egui::Align2::LEFT_TOP, cell.content, font_id, cell.fg);
            }
        });
    }
}
