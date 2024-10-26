#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0]) // wide enough for the drag-drop overlay text
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Native file dialogs and drag-and-drop files",
        options,
        Box::new(|_cc| Ok(Box::<MsgPackDifferApp>::default())),
    )
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct Crc32 {
    result: u32,
}

impl std::fmt::Display for Crc32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CRC32: {:08x}", self.result)
    }
}

impl Crc32 {
    pub fn calculate_hash_of(data: &[u8]) -> Self {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&data);
        let result = hasher.finalize();
        Self { result }
    }
}

struct MsgPackFile {
    path: PathBuf,
    data: Vec<u8>,
    crc32: Crc32,
    parsed: Option<rmpv::Value>,
}
impl MsgPackFile {
    fn load_from(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        let crc32 = Crc32::calculate_hash_of(&data);

        let parsed = rmpv::decode::read_value(&mut data.as_slice())?;
        Ok(Self {
            path: path.clone(),
            data,
            crc32,
            parsed: Some(parsed),
        })
    }
}

#[derive(Default)]
struct MsgPackDifferApp {
    path_a: Option<MsgPackFile>,
    path_b: Option<MsgPackFile>,
}

impl eframe::App for MsgPackDifferApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left").show(ctx, |ui| {
            Self::render_msg_pack_file(&mut self.path_a, ui);
        });
        egui::SidePanel::right("right").show(ctx, |ui| {
            Self::render_msg_pack_file(&mut self.path_b, ui);
        });
    }
}

impl MsgPackDifferApp {
    fn render_msg_pack_file(current_file: &mut Option<MsgPackFile>, ui: &mut egui::Ui) {
        if ui.button("Open file…").clicked() {
            if let Some(picked_path) = rfd::FileDialog::new()
                .add_filter("*.msgpack files", &["msgpack"])
                .pick_file()
            {
                match MsgPackFile::load_from(&picked_path) {
                    Ok(loaded_file) => {
                        *current_file = Some(loaded_file);
                    }
                    Err(err) => {
                        ui.label(format!("Error loading file: {}", err));
                    }
                }
            }
        }

        if let Some(picked_path) = current_file {
            ui.heading(picked_path.path.file_name().unwrap().to_string_lossy())
                .on_hover_text(picked_path.path.to_string_lossy());
            ui.label(format!("{} bytes", picked_path.data.len()));
            ui.label(format!("{}", picked_path.crc32));

            ui.horizontal(|ui| {});
        }
    }
}
