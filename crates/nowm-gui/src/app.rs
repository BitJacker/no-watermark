//! Application state and the whole of the user interface.

use std::time::{Duration, Instant};

use egui::{Align, Layout, RichText, TextStyle};
use nowm_core::{analyze, visualize, Action, Preset, Profile, Report};
use nowm_i18n::{self as i18n, Lang};

use crate::theme;

/// How often the clipboard is polled while watching is enabled.
const CLIPBOARD_POLL: Duration = Duration::from_millis(600);
/// How long a status message stays on screen.
const TOAST_LIFETIME: Duration = Duration::from_secs(3);

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Findings,
    Hidden,
    Signals,
    Limits,
}

pub struct App {
    lang: Lang,
    dark: bool,
    preset: Option<Preset>,
    profile: Profile,

    input: String,
    report: Report,
    reveal: bool,

    tab: Tab,
    show_about: bool,

    clipboard: Option<arboard::Clipboard>,
    watch_clipboard: bool,
    last_clipboard: String,
    next_poll: Instant,

    toast: Option<(String, Instant)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx, true);

        let profile = Profile::standard();
        App {
            lang: Lang::detect(),
            dark: true,
            preset: Some(Preset::Standard),
            profile,
            input: String::new(),
            report: analyze("", &profile),
            reveal: false,
            tab: Tab::Findings,
            show_about: false,
            clipboard: arboard::Clipboard::new().ok(),
            watch_clipboard: false,
            last_clipboard: String::new(),
            next_poll: Instant::now(),
            toast: None,
        }
    }

    fn recompute(&mut self) {
        self.report = analyze(&self.input, &self.profile);
    }

    fn set_preset(&mut self, preset: Preset) {
        self.preset = Some(preset);
        self.profile = Profile::from_preset(preset);
        self.recompute();
    }

    /// Any manual change to an option means the profile no longer matches a
    /// named preset, so the preset buttons stop looking selected.
    fn mark_custom(&mut self) {
        self.preset = None;
        self.recompute();
    }

    fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    fn t(&self, key: &str) -> &'static str {
        i18n::t(self.lang, key)
    }

    fn copy_output(&mut self) {
        if self.report.cleaned.is_empty() {
            let msg = self.t("ui.nothing_to_copy");
            self.toast(msg);
            return;
        }
        let text = self.report.cleaned.clone();
        let result = self.clipboard.as_mut().map(|c| c.set_text(text.clone()));
        match result {
            Some(Ok(())) => {
                // Remember what we wrote so the watcher does not immediately
                // treat our own output as new input.
                self.last_clipboard = text;
                let msg = self.t("ui.copied");
                self.toast(msg);
            }
            _ => {
                let msg = self.t("ui.clipboard_error");
                self.toast(msg);
            }
        }
    }

    fn paste_input(&mut self) {
        let result = self.clipboard.as_mut().map(|c| c.get_text());
        match result {
            Some(Ok(text)) => {
                self.input = text;
                self.recompute();
                let msg = self.t("ui.pasted");
                self.toast(msg);
            }
            _ => {
                let msg = self.t("ui.clipboard_error");
                self.toast(msg);
            }
        }
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Text", &["txt", "md", "json", "csv", "html", "srt"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.input = text;
                self.recompute();
            }
            Err(e) => self.toast(format!("{}: {e}", path.display())),
        }
    }

    fn save_output(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("cleaned.txt")
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, &self.report.cleaned) {
            Ok(()) => {
                let msg = self.t("ui.saved");
                self.toast(msg);
            }
            Err(e) => self.toast(format!("{}: {e}", path.display())),
        }
    }

    /// Poll the clipboard and clean it in place when it changes.
    fn poll_clipboard(&mut self, ctx: &egui::Context) {
        if !self.watch_clipboard {
            return;
        }
        ctx.request_repaint_after(CLIPBOARD_POLL);
        if Instant::now() < self.next_poll {
            return;
        }
        self.next_poll = Instant::now() + CLIPBOARD_POLL;

        let Some(clipboard) = self.clipboard.as_mut() else {
            return;
        };
        let Ok(text) = clipboard.get_text() else {
            return;
        };
        if text == self.last_clipboard || text.is_empty() {
            return;
        }
        self.last_clipboard = text.clone();

        let report = analyze(&text, &self.profile);
        if report.cleaned == text {
            return;
        }
        if clipboard.set_text(report.cleaned.clone()).is_ok() {
            self.last_clipboard = report.cleaned.clone();
            self.input = text;
            self.report = report;
            let removed = self.report.stats.removed + self.report.stats.replaced;
            let msg = i18n::tf(self.lang, "ui.clipboard_cleaned", &[&removed.to_string()]);
            self.toast(msg);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_clipboard(ctx);

        self.header(ctx);
        self.controls(ctx);
        self.status_bar(ctx);
        self.details(ctx);
        self.editors(ctx);
        self.about_window(ctx);
    }
}

impl App {
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("no-watermark")
                        .size(21.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.label(RichText::new(self.t("ui.tagline")).weak());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.t("ui.about")).clicked() {
                        self.show_about = true;
                    }
                    let theme_label = if self.dark {
                        self.t("ui.theme_light")
                    } else {
                        self.t("ui.theme_dark")
                    };
                    if ui.button(theme_label).clicked() {
                        self.dark = !self.dark;
                        theme::apply(ui.ctx(), self.dark);
                    }
                    egui::ComboBox::from_id_salt("lang")
                        .selected_text(self.lang.endonym())
                        .show_ui(ui, |ui| {
                            for lang in Lang::ALL {
                                ui.selectable_value(&mut self.lang, lang, lang.endonym());
                            }
                        });
                    ui.label(self.t("ui.language"));
                });
            });
            ui.add_space(6.0);
        });
    }

    fn controls(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(self.t("ui.profile")).strong());
                for preset in Preset::ALL {
                    let selected = self.preset == Some(preset);
                    let response = ui
                        .selectable_label(selected, i18n::preset(self.lang, preset))
                        .on_hover_text(i18n::preset_desc(self.lang, preset));
                    if response.clicked() {
                        self.set_preset(preset);
                    }
                }

                ui.separator();

                if ui.button(self.t("ui.paste")).clicked() {
                    self.paste_input();
                }
                if ui.button(self.t("ui.copy")).clicked() {
                    self.copy_output();
                }
                if ui.button(self.t("ui.open_file")).clicked() {
                    self.open_file();
                }
                if ui.button(self.t("ui.save_file")).clicked() {
                    self.save_output();
                }
                if ui.button(self.t("ui.clear")).clicked() {
                    self.input.clear();
                    self.recompute();
                }

                ui.separator();
                let reveal_label = self.t("ui.show_invisibles");
                ui.checkbox(&mut self.reveal, reveal_label);
                let (watch_label, watch_hint) = (
                    self.t("ui.watch_clipboard"),
                    self.t("ui.watch_clipboard_hint"),
                );
                let watch = ui
                    .checkbox(&mut self.watch_clipboard, watch_label)
                    .on_hover_text(watch_hint);
                if watch.changed() && self.watch_clipboard {
                    // Do not clean whatever happens to be on the clipboard
                    // right now: only react to the next thing the user copies.
                    self.last_clipboard = self
                        .clipboard
                        .as_mut()
                        .and_then(|c| c.get_text().ok())
                        .unwrap_or_default();
                    self.next_poll = Instant::now() + CLIPBOARD_POLL;
                }
            });

            ui.add_space(2.0);
            egui::CollapsingHeader::new(self.t("ui.options"))
                .id_salt("options")
                .show(ui, |ui| self.option_grid(ui));
            ui.add_space(4.0);
        });
    }

    fn option_grid(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut p = self.profile;

        egui::Grid::new("options-grid")
            .num_columns(3)
            .spacing([28.0, 4.0])
            .show(ui, |ui| {
                changed |= ui
                    .checkbox(&mut p.remove_invisible, self.t("ui.opt_invisible"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.remove_tags, self.t("ui.opt_tags"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.remove_bidi, self.t("ui.opt_bidi"))
                    .changed();
                ui.end_row();

                changed |= ui
                    .checkbox(&mut p.remove_variation_selectors, self.t("ui.opt_vs"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.normalize_spaces, self.t("ui.opt_spaces"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.fold_homoglyphs, self.t("ui.opt_homoglyphs"))
                    .changed();
                ui.end_row();

                changed |= ui
                    .checkbox(&mut p.ascii_typography, self.t("ui.opt_typography"))
                    .changed();
                changed |= ui.checkbox(&mut p.nfkc, self.t("ui.opt_nfkc")).changed();
                changed |= ui
                    .checkbox(&mut p.trim_trailing_whitespace, self.t("ui.opt_trim"))
                    .changed();
                ui.end_row();

                changed |= ui
                    .checkbox(&mut p.collapse_blank_lines, self.t("ui.opt_blank"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.preserve_emoji_joiners, self.t("ui.opt_emoji"))
                    .changed();
                changed |= ui
                    .checkbox(&mut p.preserve_script_joiners, self.t("ui.opt_script"))
                    .changed();
                ui.end_row();
            });

        if changed {
            self.profile = p;
            self.mark_custom();
        }
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let verdict = self.report.verdict;
                ui.label(
                    RichText::new(format!(" {} ", i18n::verdict(self.lang, verdict)))
                        .strong()
                        .color(theme::verdict_color(verdict)),
                )
                .on_hover_text(i18n::verdict_desc(self.lang, verdict));

                ui.separator();
                stat(ui, self.t("ui.stat_removed"), self.report.stats.removed);
                stat(ui, self.t("ui.stat_replaced"), self.report.stats.replaced);
                stat(ui, self.t("ui.stat_kept"), self.report.stats.kept);
                stat(ui, self.t("ui.stat_chars"), self.report.stats.input_chars);

                if let Some((message, at)) = self.toast.clone() {
                    if at.elapsed() < TOAST_LIFETIME {
                        ui.separator();
                        ui.label(RichText::new(message).color(theme::ACCENT));
                        ctx.request_repaint_after(Duration::from_millis(250));
                    } else {
                        self.toast = None;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(self.t("ui.made_by")).weak().small());
                });
            });
            ui.add_space(3.0);
        });
    }

    fn details(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("details")
            .resizable(true)
            .default_height(230.0)
            .min_height(90.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let hidden = format!(
                        "{} ({})",
                        self.t("report.hidden_payloads"),
                        self.report.payloads.len()
                    );
                    let findings = format!(
                        "{} ({})",
                        self.t("report.findings"),
                        self.report.findings.len()
                    );
                    if ui
                        .selectable_label(self.tab == Tab::Findings, findings)
                        .clicked()
                    {
                        self.tab = Tab::Findings;
                    }
                    let hidden_label = if self.report.payloads.is_empty() {
                        RichText::new(hidden)
                    } else {
                        RichText::new(hidden).color(theme::CRITICAL).strong()
                    };
                    if ui
                        .selectable_label(self.tab == Tab::Hidden, hidden_label)
                        .clicked()
                    {
                        self.tab = Tab::Hidden;
                    }
                    if ui
                        .selectable_label(self.tab == Tab::Signals, self.t("report.signals"))
                        .clicked()
                    {
                        self.tab = Tab::Signals;
                    }
                    if ui
                        .selectable_label(self.tab == Tab::Limits, self.t("limits.title"))
                        .clicked()
                    {
                        self.tab = Tab::Limits;
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("details-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Findings => self.findings_table(ui),
                        Tab::Hidden => self.hidden_panel(ui),
                        Tab::Signals => self.signals_panel(ui),
                        Tab::Limits => self.limits_panel(ui),
                    });
            });
    }

    fn findings_table(&mut self, ui: &mut egui::Ui) {
        if self.report.findings.is_empty() {
            ui.label(RichText::new(self.t("report.no_findings")).color(theme::OK));
            return;
        }
        egui::Grid::new("findings")
            .num_columns(6)
            .striped(true)
            .spacing([16.0, 3.0])
            .show(ui, |ui| {
                for header in [
                    "ui.col_position",
                    "ui.col_char",
                    "ui.col_name",
                    "ui.col_category",
                    "ui.col_action",
                    "ui.col_note",
                ] {
                    ui.label(RichText::new(self.t(header)).strong().small());
                }
                ui.end_row();

                for f in &self.report.findings {
                    ui.label(
                        RichText::new(format!("{}:{}", f.line, f.column))
                            .monospace()
                            .small(),
                    );
                    ui.label(
                        RichText::new(f.display.as_str())
                            .monospace()
                            .small()
                            .color(theme::severity_color(f.severity)),
                    );
                    ui.label(RichText::new(f.name.as_str()).small());
                    ui.label(RichText::new(i18n::category(self.lang, f.category)).small());
                    ui.label(RichText::new(action_text(self.lang, &f.action)).small());
                    ui.label(
                        RichText::new(f.note.map(|n| i18n::note(self.lang, n)).unwrap_or(""))
                            .small()
                            .weak(),
                    );
                    ui.end_row();
                }
            });
    }

    fn hidden_panel(&mut self, ui: &mut egui::Ui) {
        if self.report.payloads.is_empty() {
            ui.label(RichText::new(self.t("report.no_findings")).color(theme::OK));
            return;
        }
        ui.label(
            RichText::new(self.t("report.payload_warning"))
                .color(theme::CRITICAL)
                .strong(),
        );
        ui.add_space(6.0);
        for (i, p) in self.report.payloads.iter().enumerate() {
            ui.label(
                RichText::new(format!(
                    "{}. {} — {} chars @ {}",
                    i + 1,
                    i18n::payload_kind(self.lang, p.kind),
                    p.len_chars,
                    p.start_char
                ))
                .strong(),
            );
            match &p.text {
                Some(text) => {
                    ui.label(
                        RichText::new(self.t("report.decoded_as_text"))
                            .small()
                            .weak(),
                    );
                    let mut shown = text.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(2),
                    );
                }
                None => {
                    ui.label(
                        RichText::new(self.t("report.decoded_as_hex"))
                            .small()
                            .weak(),
                    );
                    ui.label(RichText::new(p.hex.as_str()).monospace().small());
                }
            }
            ui.add_space(8.0);
        }
    }

    fn signals_panel(&mut self, ui: &mut egui::Ui) {
        if self.report.signals.is_empty() {
            ui.label(RichText::new(self.t("report.no_findings")).color(theme::OK));
            return;
        }
        for s in &self.report.signals {
            ui.label(
                RichText::new(format!("{} — {}", i18n::signal(self.lang, s.kind), s.count))
                    .strong(),
            );
            ui.label(
                RichText::new(i18n::signal_desc(self.lang, s.kind))
                    .small()
                    .weak(),
            );
            ui.add_space(6.0);
        }
    }

    fn limits_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(self.t("limits.title")).strong());
        ui.add_space(4.0);
        ui.label(self.t("limits.body"));
    }

    fn editors(&mut self, ctx: &egui::Context) {
        let placeholder = self.t("ui.placeholder_input");
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut edited = false;
            ui.columns(2, |cols| {
                cols[0].label(RichText::new(self.t("ui.input")).strong());
                egui::ScrollArea::vertical()
                    .id_salt("input-scroll")
                    .auto_shrink([false, false])
                    .show(&mut cols[0], |ui| {
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .font(TextStyle::Monospace)
                                .hint_text(placeholder)
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                        edited = response.changed();
                    });

                cols[1].label(RichText::new(self.t("ui.output")).strong());
                egui::ScrollArea::vertical()
                    .id_salt("output-scroll")
                    .auto_shrink([false, false])
                    .show(&mut cols[1], |ui| {
                        // A throwaway copy keeps the field selectable and
                        // copyable while making edits to it meaningless.
                        let mut shown = if self.reveal {
                            visualize(&self.report.cleaned)
                        } else {
                            self.report.cleaned.clone()
                        };
                        ui.add(
                            egui::TextEdit::multiline(&mut shown)
                                .font(TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                    });
            });
            if edited {
                self.recompute();
            }
        });
    }

    fn about_window(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let mut open = true;
        egui::Window::new(self.t("ui.about"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new("no-watermark").size(18.0).strong());
                ui.label(format!("version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(6.0);
                ui.label(self.t("ui.tagline"));
                ui.add_space(6.0);
                ui.label(self.t("ui.made_by"));
                ui.label(self.t("ui.about_body"));
                ui.add_space(6.0);
                ui.hyperlink_to(
                    "github.com/BitJacker/no-watermark",
                    "https://github.com/BitJacker/no-watermark",
                );
                ui.add_space(10.0);
                ui.label(RichText::new(self.t("limits.title")).strong());
                ui.label(RichText::new(self.t("limits.body")).small());
            });
        self.show_about = open;
    }
}

fn stat(ui: &mut egui::Ui, label: &str, value: usize) {
    ui.label(RichText::new(format!("{label}:")).small().weak());
    ui.label(RichText::new(value.to_string()).small().strong());
}

fn action_text(lang: Lang, action: &Action) -> String {
    match action {
        Action::Removed => i18n::t(lang, "action.removed").to_string(),
        Action::Replaced(v) => format!("{} {:?}", i18n::t(lang, "action.replaced"), v),
        Action::Kept => i18n::t(lang, "action.kept").to_string(),
    }
}
