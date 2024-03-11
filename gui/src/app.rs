use std::sync::mpsc::{Receiver, Sender};

use egui::{Color32, Label, RichText, Sense};

use log::{debug, error};
use plox::{
    detect_game,
    parser::{Parser, Warning},
    rules::EWarningRule,
};

use crate::{init_parser, ParserResult};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    parser: Option<Parser>,

    //#[serde(skip)]
    //sorter: Sorter,

    // plugins
    //#[serde(skip)]
    //mods: Vec<String>,
    #[serde(skip)]
    new_order: Vec<String>,

    // notes
    #[serde(skip)]
    warnings: Vec<Warning>,

    #[serde(skip)]
    plugin_warning_map: Vec<(String, usize)>,

    // filters
    show_notes: bool,
    show_conflicts: bool,
    show_requires: bool,
    show_patches: bool,
    text_filter: String,
    plugin_filter: String,

    // ui
    #[serde(skip)]
    warning: String,
    // Sender/Receiver for async notifications.
    #[serde(skip)]
    tx: Sender<Option<ParserResult>>,
    #[serde(skip)]
    rx: Receiver<Option<ParserResult>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            parser: None,
            //sorter: new_stable_sorter(),
            //mods: vec![],
            new_order: vec![],
            warnings: vec![],
            plugin_warning_map: vec![],
            show_notes: true,
            show_conflicts: true,
            show_requires: true,
            show_patches: true,
            text_filter: String::new(),
            plugin_filter: String::new(),
            warning: String::new(),
            tx,
            rx,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        // TODO do all the logic here
        let app: TemplateApp = Default::default();

        // init parser
        // Execute the runtime in its own thread.
        // The future doesn't have to do anything. In this example, it just sleeps forever.
        let tx = app.tx.clone();
        std::thread::spawn(move || {
            pollster::block_on(async {
                init_parser(detect_game().expect("no game"), tx.clone());
                debug!("Parser init done");
            });
        });

        app
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // TODO make this async in the backgorund
        // Update the counter with the async response.
        if let Ok(result) = self.rx.try_recv() {
            // todo
            if let Some((parser, new_order, plugin_warning_map)) = result {
                self.parser = Some(parser);
                self.new_order = new_order;
                self.plugin_warning_map = plugin_warning_map;
                self.warnings = self.parser.as_ref().unwrap().warnings.clone();
            }
        } else if self.parser.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Loading...");
            });
            return;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        // warning only UI
        if !self.warning.is_empty() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("WARNING");
                ui.label(self.warning.clone());

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    powered_by_egui_and_eframe(ui);
                    egui::warn_if_debug_build(ui);
                });
            });

            return;
        }

        // TODO main UI
        egui::SidePanel::left("side_panel")
            .min_width(200_f32)
            .show(ctx, |ui| {
                ui.heading("New Order");
                for m in &self.new_order {
                    let notes: Vec<_> = self
                        .plugin_warning_map
                        .iter()
                        .filter(|(p, _)| p.to_lowercase() == *m.to_lowercase())
                        .collect();

                    let mut text = if !notes.is_empty() {
                        let i = notes[0].1;
                        let background_color = get_color_for_rule(&self.warnings[i].rule);
                        // make it more transparent

                        RichText::new(m).background_color(background_color.gamma_multiply(0.5))
                    } else {
                        RichText::new(m)
                    };

                    // override background color if mod is in plugin_filter with light blue
                    if !self.plugin_filter.is_empty()
                        && m.to_lowercase() == self.plugin_filter.to_lowercase()
                    {
                        text = text.background_color(Color32::LIGHT_BLUE);
                    };

                    ui.horizontal(|ui| {
                        if ui.add(Label::new(text).sense(Sense::click())).clicked() {
                            // add notes to filter
                            self.plugin_filter = m.clone();
                        }
                    });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("PLOX");

            // filters
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.show_notes, "Notes");
                ui.toggle_value(&mut self.show_conflicts, "Conflicts");
                ui.toggle_value(&mut self.show_requires, "Requires");
                ui.toggle_value(&mut self.show_patches, "Patches");

                ui.separator();
                //filter text
                ui.add(egui::TextEdit::singleline(&mut self.text_filter).hint_text("Filter"));
            });

            // display warnings
            for (i, w) in self.warnings.iter().enumerate() {
                //filters
                if !self.show_notes && matches!(w.rule, EWarningRule::Note(_)) {
                    continue;
                }
                if !self.show_conflicts && matches!(w.rule, EWarningRule::Conflict(_)) {
                    continue;
                }
                if !self.show_requires && matches!(w.rule, EWarningRule::Requires(_)) {
                    continue;
                }
                if !self.show_patches && matches!(w.rule, EWarningRule::Patch(_)) {
                    continue;
                }

                if !self.text_filter.is_empty()
                    && !w
                        .get_rule_name()
                        .to_lowercase()
                        .contains(&self.text_filter.to_lowercase())
                    && !w
                        .get_comment()
                        .to_lowercase()
                        .contains(&self.text_filter.to_lowercase())
                {
                    continue;
                }

                // plugin filter
                if !self.plugin_filter.is_empty() {
                    let mut found = false;
                    for p in &w.get_plugins() {
                        if p.to_lowercase() == self.plugin_filter.to_lowercase() {
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        continue;
                    }
                }

                // item view
                let mut frame = egui::Frame::default().inner_margin(4.0).begin(ui);
                {
                    // create itemview
                    let color = get_color_for_rule(&w.rule);
                    frame.content_ui.colored_label(color, w.get_rule_name());

                    frame.content_ui.label(w.get_comment());

                    frame.content_ui.push_id(i, |ui| {
                        ui.collapsing("Plugins Affected", |ui| {
                            for plugin in &w.get_plugins() {
                                ui.label(plugin);
                            }
                        });
                    });
                }
                let response = frame.allocate_space(ui);
                if response.hovered() {
                    let mut bg_color = egui::Color32::LIGHT_GRAY;
                    // if theme is dark, make it darker
                    if ctx.style().visuals.dark_mode {
                        bg_color = Color32::DARK_GRAY;
                    }

                    frame.frame.fill = bg_color;
                }
                frame.paint(ui);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn get_color_for_rule(rule: &EWarningRule) -> Color32 {
    match rule {
        EWarningRule::Note(_) => Color32::DARK_GREEN,
        EWarningRule::Conflict(_) => Color32::RED,
        EWarningRule::Requires(_) => Color32::YELLOW,
        EWarningRule::Patch(_) => Color32::BLUE,
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
