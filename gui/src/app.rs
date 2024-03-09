use std::env;

use log::error;
use plox::{
    detect_game, download_latest_rules, gather_mods, get_default_rules_dir,
    parser::{self, Parser},
    sorter::{new_stable_sorter, Sorter},
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    parser: Option<Parser>,

    #[serde(skip)]
    sorter: Sorter,

    #[serde(skip)]
    mods: Vec<String>,

    #[serde(skip)]
    new_order: Vec<String>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            parser: None,
            sorter: new_stable_sorter(),
            mods: vec![],
            new_order: vec![],
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        // TODO do all the logic here
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // first setup
        let mut render_warning_only = false;
        if self.parser.is_none() {
            // init parser
            if let Some(game) = detect_game() {
                // TODO this blocks UI and sorts everything
                // TODO run a terminal?
                let root = env::current_dir().expect("No current working dir");

                // rules
                let rules_dir = get_default_rules_dir(game);
                download_latest_rules(game, &rules_dir);

                // mods
                self.mods = gather_mods(&root, game);

                // parser
                let mut parser = parser::get_parser(game);
                if let Err(e) = parser.init(rules_dir) {
                    error!("Parser init failed: {}", e);
                    // TODO do something, render some warning only
                    render_warning_only = true;
                }

                // evaluate
                parser.evaluate_plugins(&self.mods);

                // sort
                match self.sorter.topo_sort(&self.mods, &parser.order_rules) {
                    Ok(new) => {
                        self.new_order = new;
                    }
                    Err(e) => {
                        error!("error sorting: {e:?}");
                        // TODO do something, render some warning only
                        render_warning_only = true;
                    }
                }

                self.parser = Some(parser);
            } else {
                error!("No game detected");
                // TODO do something, render some warning only
                render_warning_only = true;
            }
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("PLOX");

            if render_warning_only {
                // TODO some warning
                ui.label("WARNING");

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    powered_by_egui_and_eframe(ui);
                    egui::warn_if_debug_build(ui);
                });

                return;
            }

            // TODO main UI

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
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
