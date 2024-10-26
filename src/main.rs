#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::path::PathBuf;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("MsgPack Differ"),
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
    parsed: rmpv::Value,
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
            parsed,
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
        let width = ctx.available_rect().width();
        egui::SidePanel::left("path_a")
            .min_width(width / 4.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    Self::render_msg_pack_file(&mut self.path_a, "A", ui);
                });
            });
        egui::SidePanel::right("path_b")
            .min_width(width / 4.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    Self::render_msg_pack_file(&mut self.path_b, "B", ui);
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                self.render_msg_pack_diff(ui);
            });
        });
    }
}

impl MsgPackDifferApp {
    fn render_msg_pack_file(
        current_file: &mut Option<MsgPackFile>,
        label: &str,
        ui: &mut egui::Ui,
    ) {
        if let Some(file) = current_file {
            ui.horizontal(|ui| {
                ui.heading(file.path.file_name().unwrap().to_string_lossy())
                    .on_hover_text(file.path.to_string_lossy());
                ui.label(format!(
                    "({} bytes, crc={:08x})",
                    file.data.len(),
                    file.crc32.result
                ));
            });
        } else {
            ui.heading(format!("File {label}"));
        }

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

        if let Some(file) = current_file {
            render_rmpv(ui, &file.parsed);
        }
    }

    fn render_msg_pack_diff(&mut self, ui: &mut egui::Ui) {
        ui.heading("Diff");
        let prompt = if self.path_a.is_none() && self.path_b.is_none() {
            Some("Select files A and B to compare them")
        } else if self.path_a.is_none() {
            Some("Select file A to compare with file B")
        } else if self.path_b.is_none() {
            Some("Select file B to compare with file A")
        } else {
            None
        };
        if let Some(prompt) = prompt {
            ui.label(prompt);
            return;
        }
    }
}

fn render_rmpv(ui: &mut egui::Ui, value: &rmpv::Value) {
    match value {
        rmpv::Value::Nil => {
            ui.label("nil");
        }
        rmpv::Value::Boolean(b) => {
            ui.label(if *b { "true" } else { "false" });
        }
        rmpv::Value::Integer(i) => {
            ui.label(i.to_string());
        }
        rmpv::Value::F32(f) => {
            ui.label(f.to_string());
        }
        rmpv::Value::F64(f) => {
            ui.label(f.to_string());
        }
        rmpv::Value::String(s) => {
            ui.label(s.as_str().expect("should be valid string"));
        }
        rmpv::Value::Binary(b) => {
            ui.label(format!("{} bytes", b.len()));
        }
        rmpv::Value::Array(a) => {
            ui.vertical(|ui| {
                for (i, array_item) in a.iter().enumerate() {
                    ui.horizontal(|ui| {
                        // ui.label(format!("[{i}]"));
                        ui.push_id(i, |ui| {
                            egui::CollapsingHeader::new(format!("array[{i}]"))
                                .default_open(false)
                                .show(ui, |ui| {
                                    render_rmpv(ui, array_item);
                                });
                        });
                    });
                }
            });
        }
        rmpv::Value::Map(m) => {
            ui.vertical(|ui| {
                for (key, value) in m.iter() {
                    ui.push_id(HashableValue(key), |ui| {
                        egui::CollapsingHeader::new(format!("map[{}]", key))
                            .default_open(true)
                            .show(ui, |ui| {
                                if matches!(
                                    key,
                                    rmpv::Value::Nil
                                        | rmpv::Value::String(_)
                                        | rmpv::Value::Integer(_)
                                        | rmpv::Value::F64(_)
                                        | rmpv::Value::F32(_)
                                ) {
                                    // already rendered the key in the map[{}] header so skip it
                                    render_rmpv(ui, value);
                                } else {
                                    ui.label(format!("[FYI! {}]", type_name_of(key)));
                                    ui.horizontal(|ui| {
                                        render_rmpv(ui, key);
                                        ui.label("->");
                                        render_rmpv(ui, value);
                                    });
                                }
                            })
                            .header_response
                            .on_hover_text_at_pointer(type_name_of(key));
                    });
                }
            });
        }
        rmpv::Value::Ext(i8, bytes) => {
            ui.label(format!("External {}, {} bytes", i8, bytes.len()));
        }
    }
}

fn type_name_of(value: &rmpv::Value) -> &'static str {
    match value {
        rmpv::Value::Nil => "Key type: Nil",
        rmpv::Value::Boolean(_) => "Key type: Boolean",
        rmpv::Value::Integer(_) => "Key type: Integer",
        rmpv::Value::F32(_) => "Key type: F32",
        rmpv::Value::F64(_) => "Key type: F64",
        rmpv::Value::String(_) => "Key type: String",
        rmpv::Value::Binary(_) => "Key type: Binary",
        rmpv::Value::Array(_) => "Key type: Array",
        rmpv::Value::Map(_) => "Key type: Map",
        rmpv::Value::Ext(_, _) => "Key type: Ext",
    }
}

struct HashableValue<'a>(&'a rmpv::Value);
impl<'a> std::hash::Hash for HashableValue<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.0 {
            rmpv::Value::Nil => {
                state.write_u8(0);
            }
            rmpv::Value::Boolean(b) => {
                (*b).hash(state);
            }
            rmpv::Value::Integer(i) => {
                if let Some(i) = i.as_i64() {
                    i.hash(state);
                } else if let Some(i) = i.as_u64() {
                    i.hash(state);
                } else if let Some(i) = i.as_f64() {
                    i.to_bits().hash(state);
                } else {
                    panic!("unsupported rmpv integer type");
                }
            }
            rmpv::Value::F32(f) => {
                f.to_bits().hash(state);
            }
            rmpv::Value::F64(f) => {
                f.to_bits().hash(state);
            }
            rmpv::Value::String(s) => {
                s.as_bytes().hash(state);
            }
            rmpv::Value::Binary(b) => {
                state.write_u8(6);
                state.write_u64(b.len() as u64);
                state.write(b);
            }
            rmpv::Value::Array(a) => {
                state.write_u8(7);
                state.write_u64(a.len() as u64);
                for item in a {
                    HashableValue(item).hash(state);
                }
            }
            rmpv::Value::Map(m) => {
                state.write_u8(8);
                state.write_u64(m.len() as u64);
                for (key, value) in m {
                    HashableValue(key).hash(state);
                    HashableValue(value).hash(state);
                }
            }
            rmpv::Value::Ext(i8, bytes) => {
                state.write_u8(9);
                state.write_i8(*i8);
                state.write_u64(bytes.len() as u64);
                state.write(bytes);
            }
        }
    }
}
impl<'a> PartialEq for HashableValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(other.0)
    }
}
