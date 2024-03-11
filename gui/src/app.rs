use std::sync::mpsc::{Receiver, Sender};

use egui::{Color32, Label, Sense};

use log::{error, info};
use plox::{detect_game, rules::EWarningRule, update_new_load_order};

use crate::{init_parser, AppData, ETheme};

#[derive(PartialEq)]
pub enum EModListView {
    NewOrder,
    LoadOrder,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    app_data: Option<AppData>,

    // filters
    show_notes: bool,
    show_conflicts: bool,
    show_requires: bool,
    show_patches: bool,
    #[serde(skip)]
    mod_list_view: EModListView,
    #[serde(skip)]
    text_filter: String,
    #[serde(skip)]
    plugin_filter: String,
    #[serde(skip)]
    plugin_hover_filter: Vec<String>,

    // ui
    theme: Option<ETheme>,
    #[serde(skip)]
    async_log: String,
    // Sender/Receiver for async notifications.
    #[serde(skip)]
    tx: Sender<String>,
    #[serde(skip)]
    rx: Receiver<String>,
    // TODO UI refactor to use one channel
    #[serde(skip)]
    tx2: Sender<Option<AppData>>,
    #[serde(skip)]
    rx2: Receiver<Option<AppData>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();

        Self {
            app_data: None,
            show_notes: true,
            show_conflicts: true,
            show_requires: true,
            show_patches: true,
            theme: None,
            mod_list_view: EModListView::NewOrder,
            text_filter: String::new(),
            plugin_filter: String::new(),
            plugin_hover_filter: vec![],
            async_log: String::new(),
            rx,
            tx,
            rx2,
            tx2,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_zoom_factor(1.3);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let app: TemplateApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        // do all the logic here
        // init parser
        // Execute the runtime in its own thread.
        // The future doesn't have to do anything. In this example, it just sleeps forever.
        let tx = app.tx.clone();
        let tx2 = app.tx2.clone();

        std::thread::spawn(move || {
            let result = pollster::block_on(async {
                init_parser(detect_game().expect("no game"), tx.clone())
            });
            // send result to app
            let _ = tx.send("App initialized".to_string());
            let _ = tx2.send(result);
        });

        app
    }

    /// Dark/light mode switch
    fn global_dark_light_mode_buttons(&mut self, ui: &mut egui::Ui) {
        let mut visuals = ui.ctx().style().visuals.clone();
        visuals.light_dark_radio_buttons(ui);
        ui.ctx().set_visuals(visuals);
        match ui.ctx().style().visuals.clone().dark_mode {
            true => self.theme = Some(ETheme::Dark),
            false => self.theme = Some(ETheme::Light),
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // set dark mode by default
        if self.theme.is_none() {
            ctx.set_visuals(egui::Visuals::light())
        } else if let Some(theme) = &self.theme {
            match theme {
                crate::ETheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
                crate::ETheme::Light => ctx.set_visuals(egui::Visuals::light()),
            }
        }

        // Update the counter with the async response.
        if self.app_data.is_none() {
            if let Ok(result) = self.rx.try_recv() {
                self.async_log += format!("{}\n", result).as_str();
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Loading...");
                ui.separator();
                ui.label(&self.async_log);
            });

            if let Ok(result) = self.rx2.try_recv() {
                self.app_data = result;
            }

            // pump ui events while in thread
            ctx.request_repaint();

            return;
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
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

                self.global_dark_light_mode_buttons(ui);
            });
        });

        let Some(data) = self.app_data.as_ref() else {
            return;
        };

        // side panel
        egui::SidePanel::left("side_panel")
            .min_width(200_f32)
            .show(ctx, |ui| {
                ui.heading("Load Order");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.mod_list_view, EModListView::LoadOrder, "Old");
                    ui.radio_value(&mut self.mod_list_view, EModListView::NewOrder, "New");
                });
                ui.separator();

                // mod list
                let order = match self.mod_list_view {
                    EModListView::NewOrder => &data.new_order,
                    EModListView::LoadOrder => &data.old_order,
                };
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for mod_name in order {
                        let notes: Vec<_> = data
                            .plugin_warning_map
                            .iter()
                            .filter(|(p, _)| p.to_lowercase() == *mod_name.to_lowercase())
                            .collect();

                        // get color for background
                        let mut bg_color = if !notes.is_empty() {
                            let i = notes[0].1;
                            let background_color = get_color_for_rule(&data.warnings[i].rule);
                            // make it more transparent
                            background_color.gamma_multiply(0.5)
                        } else {
                            Color32::TRANSPARENT
                        };
                        // override background color if mod is in plugin_filter with light blue
                        if !self.plugin_filter.is_empty()
                            && mod_name.to_lowercase() == self.plugin_filter.to_lowercase()
                        {
                            bg_color = Color32::LIGHT_BLUE;
                            if ctx.style().visuals.dark_mode {
                                bg_color = Color32::DARK_BLUE;
                            }
                        };
                        // override the background color if mod is hovered
                        if self.plugin_hover_filter.contains(&mod_name.to_lowercase()) {
                            bg_color = Color32::LIGHT_BLUE;
                            if ctx.style().visuals.dark_mode {
                                bg_color = Color32::DARK_BLUE;
                            }
                        }

                        // item view
                        egui::Frame::none().fill(bg_color).show(ui, |ui| {
                            let label = Label::new(mod_name).sense(Sense::click());
                            // TODO UI Left align the text
                            let r = ui.add_sized([ui.available_width(), 0_f32], label);
                            if r.clicked() {
                                // unselect if clicked again
                                if self.plugin_filter == mod_name.clone() {
                                    self.plugin_filter = String::new();
                                } else {
                                    // add notes to filter
                                    self.plugin_filter = mod_name.clone();
                                }
                            }
                        });
                    }
                });

                // add spacing until bottom
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // accept button
                    ui.add_space(4_f32);

                    let r =
                        ui.add_sized([ui.available_width(), 0_f32], egui::Button::new("Accept"));
                    if r.clicked() {
                        // apply sorting
                        match update_new_load_order(data.game, &data.new_order) {
                            Ok(_) => {
                                info!("Update successful");
                            }
                            Err(e) => {
                                error!("Could not updae load order: {}", e);
                            }
                        }

                        // exit the app
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.separator();
                });
            });

        // main panel
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, w) in data.warnings.iter().enumerate() {
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

                        // update hover filter
                        self.plugin_hover_filter = w.get_plugins().clone();
                    } else {
                        self.plugin_hover_filter = vec![];
                    }
                    frame.paint(ui);
                }
            });
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
        ui.hyperlink_to("plox", "https://github.com/rfuzzo/plox");
        ui.label(" powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(".");
    });
}
