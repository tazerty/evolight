//! The library filter bar above the grid: rating (≥ / = / ≤), flags, colour labels, kind,
//! edited state and camera / lens / keyword pickers — every control drives `library.filter`.
//! "Save as Smart Album…" keeps the current view as a live album; "Clear" resets the filter.

use egui::{Color32, Rect, Sense, Stroke, StrokeKind, vec2};
use evolight_catalog::{ColorLabel, Filter, Flag, MediaKind, RatingOp};
use serde_json::{Value, json};

use crate::EvoLightApp;
use crate::icons::{Icon, paint};
use crate::theme::Tokens;
use crate::widgets::register;

pub const HEIGHT: f32 = 40.0;
/// Space between two groups of controls.
const GROUP_GAP: f32 = 14.0;
/// Room `widgets::dropdown` gives its chevron after the text.
const CHEVRON: f32 = 20.0;

/// Colour-label swatches (UI colours, our own choice).
pub use crate::theme::label_color;

fn filter(app: &mut EvoLightApp, patch: Value) {
    let _ = app.run("library.filter", patch);
}

/// A small label before a group of controls.
fn caption(ui: &mut egui::Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(egui::RichText::new(crate::i18n::tr(text)).font(t.font(12.0)).color(t.text_dim));
}

/// A square toggle with an icon or a colour swatch.
fn toggle(ui: &mut egui::Ui, id: &str, on: bool, tip: &str, draw: impl FnOnce(&egui::Painter, Rect, Color32)) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
    crate::widgets::announce_selected(&resp, crate::i18n::tr(tip), on);
    register(ui, format!("filter:{id}"), r);
    let p = ui.painter();
    if on {
        p.rect_filled(r, 5.0, t.tool_active);
        p.rect_stroke(r, 5.0, Stroke::new(1.0, t.accent), StrokeKind::Inside);
    } else if resp.hovered() {
        p.rect_filled(r, 5.0, t.hover);
    }
    draw(
        p,
        r.shrink(5.0),
        if on {
            t.text
        } else if resp.hovered() {
            t.text_label
        } else {
            t.icon
        },
    );
    resp.on_hover_text(crate::i18n::tr(tip))
}

/// What picker `id` shows for `current`: a camera, lens or keyword picked from the library as it
/// is, anything else translated.
fn picker_text<'a>(id: &str, current: &'a str, active: bool) -> &'a str {
    if active && matches!(id, "camera" | "lens" | "keyword") { current } else { crate::i18n::tr(current) }
}

/// What a metadata picker shows: the camera, lens or keyword picked (cut short past 22
/// characters), else what it picks from (`all_label`, translated when drawn).
fn metadata_text(current: Option<&str>, all_label: &str) -> String {
    let shown = current.unwrap_or(all_label);
    if shown.chars().count() > 22 { format!("{}…", shown.chars().take(21).collect::<String>()) } else { shown.to_string() }
}

/// The width `text` takes in the bar's 12.5-point text, to plan a row before drawing it.
fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    let t = Tokens::get(ui.ctx());
    ui.painter().layout_no_wrap(text.to_string(), t.font(12.5), t.text).size().x
}

/// The width of a picker showing `text`, its chevron included.
fn picker_width(ui: &egui::Ui, text: &str) -> f32 {
    text_width(ui, text) + CHEVRON
}

/// The width of a button labelled `text`: the text and the button's padding either side.
fn button_width(ui: &egui::Ui, text: &str) -> f32 {
    text_width(ui, text) + 2.0 * ui.spacing().button_padding.x
}

/// A dropdown showing `current`; `items` are (label, filter patch, selected).
fn picker(app: &mut EvoLightApp, ui: &mut egui::Ui, id: &str, current: &str, active: bool, items: Vec<(String, Value, bool)>) {
    let t = Tokens::get(ui.ctx());
    let r = crate::widgets::dropdown(
        ui,
        &format!("filter-{id}"),
        picker_text(id, current, active),
        t.font(12.5),
        if active { t.text } else { t.text_label },
    );
    if active {
        ui.painter().rect_stroke(r.rect.expand(2.0), 4.0, Stroke::new(1.0, t.accent.gamma_multiply(0.7)), StrokeKind::Outside);
    }
    let mut chosen = None;
    egui::Popup::menu(&r).show(|ui| {
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (label, patch, sel) in &items {
                if ui
                    .selectable_label(
                        *sel,
                        if matches!(id, "camera" | "lens" | "keyword") && patch.get(id).is_some_and(|value| !value.is_null()) {
                            label.as_str()
                        } else {
                            crate::i18n::tr(label)
                        },
                    )
                    .clicked()
                {
                    chosen = Some(patch.clone());
                }
            }
        });
    });
    if let Some(p) = chosen {
        filter(app, p);
    }
}

pub fn show(app: &mut EvoLightApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    // Whether the bar takes one row or two depends on how wide its controls are in the language
    // shown, which is known once the first row is drawn: the background keeps its place under the
    // controls now and is painted at the end, with the bar's height.
    let backdrop = ui.painter().add(egui::Shape::Noop);
    let top = ui.available_rect_before_wrap().min;
    let width = ui.available_width();
    let row = |ui: &mut egui::Ui, i: f32| {
        let r = Rect::from_min_size(top + vec2(16.0, 4.0 + i * (HEIGHT - 8.0)), vec2(width - 32.0, HEIGHT - 8.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)));
        c.spacing_mut().item_spacing.x = 4.0;
        c
    };
    let mut first = row(ui, 0.0);
    let mut second = row(ui, 1.0);
    let two_rows = controls(app, &mut first, &mut second);
    let bar = Rect::from_min_size(top, vec2(width, if two_rows { 2.0 * HEIGHT - 8.0 } else { HEIGHT }));
    ui.painter().set(backdrop, egui::Shape::rect_filled(bar, 0.0, t.chrome));
    ui.painter().line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, t.divider));
    register(ui, "filterbar", bar);
    ui.advance_cursor_after_rect(bar);
}

/// Draws the controls on the bar's `first` row while it holds them all; otherwise the metadata
/// pickers and the actions go on the `second`, and the kind and edited pickers with them when
/// the first row has no room left even for those and the second has. Decided on the widths the
/// controls take in the language shown. Returns whether the second row was used.
fn controls(app: &mut EvoLightApp, first: &mut egui::Ui, second: &mut egui::Ui) -> bool {
    let f = app.session.filter.clone();
    marks(app, first, &f);

    // kind + edited
    let kind_label = match f.kind {
        None => "All types",
        Some(MediaKind::Image) => "Photos",
        Some(MediaKind::Raw) => "Raw",
        Some(MediaKind::Video) => "Videos",
    };
    let kind_label = match f.merged.as_deref() {
        Some("hdr") => "HDR",
        Some("panorama") => "Panoramas",
        Some("hdrPanorama") => "HDR Panoramas",
        Some(_) => "Merged",
        None => kind_label,
    };
    let kind_active = f.kind.is_some() || f.merged.is_some();
    let edited_label = match f.edited {
        None => "Edited or not",
        Some(true) => "Edited",
        Some(false) => "Unedited",
    };

    // metadata pickers: (id, filter key, what it picks from, the value picked, the library's values)
    let values = app.caches.filter_values(&app.session.catalog);
    let metadata = [
        ("camera", "camera", "Camera", f.camera.clone(), values.cameras.clone()),
        ("lens", "lens", "Lens", f.lens.clone(), values.lenses.clone()),
        ("keyword", "keyword", "Keyword", f.keyword.clone(), values.keywords.clone()),
    ];
    let shown: Vec<String> = metadata.iter().map(|(_, _, all_label, current, _)| metadata_text(current.as_deref(), all_label)).collect();

    // what is left of the first row, and what the groups after the labels take
    let probe: &egui::Ui = first;
    let gap = probe.spacing().item_spacing.x;
    let kinds = picker_width(probe, picker_text("kind", kind_label, kind_active))
        + gap
        + picker_width(probe, picker_text("edited", edited_label, f.edited.is_some()));
    let pickers: f32 =
        metadata.iter().zip(&shown).map(|((id, _, _, current, _), text)| picker_width(probe, picker_text(id, text, current.is_some())) + gap).sum();
    let [save, clear, presets] = action_labels();
    let rest = gap + pickers + GROUP_GAP + button_width(probe, save) + gap + button_width(probe, clear) + gap + picker_width(probe, presets);
    let room = probe.available_width();
    // (the kind and edited pickers leave the first row only for a second that holds them)
    let kinds_below = kinds > room && kinds + rest <= second.available_width();
    let rest_below = kinds_below || kinds + rest > room;

    let ui: &mut egui::Ui = if kinds_below { &mut *second } else { &mut *first };
    picker(
        app,
        ui,
        "kind",
        kind_label,
        kind_active,
        [
            ("All types", Value::Null, None),
            ("Photos", json!("image"), Some(MediaKind::Image)),
            ("Raw", json!("raw"), Some(MediaKind::Raw)),
            ("Videos", json!("video"), Some(MediaKind::Video)),
        ]
        .into_iter()
        .map(|(l, v, k)| (l.to_string(), json!({"kind": v, "merged": null}), f.kind == k && f.merged.is_none()))
        .chain(
            [("HDR", "hdr"), ("Panoramas", "panorama"), ("HDR Panoramas", "hdrPanorama")]
                .into_iter()
                .map(|(l, m)| (l.to_string(), json!({"kind": null, "merged": m}), f.merged.as_deref() == Some(m))),
        )
        .collect(),
    );
    picker(
        app,
        ui,
        "edited",
        edited_label,
        f.edited.is_some(),
        [("Edited or not", Value::Null, None), ("Edited", json!(true), Some(true)), ("Unedited", json!(false), Some(false))]
            .into_iter()
            .map(|(l, v, e)| (l.to_string(), json!({"edited": v}), f.edited == e))
            .collect(),
    );

    // metadata pickers
    let ui: &mut egui::Ui = if rest_below { second } else { first };
    for ((id, key, all_label, current, values), shown) in metadata.into_iter().zip(shown) {
        let mut items = vec![(format!("Any {}", all_label.to_lowercase()), json!({key: Value::Null}), current.is_none())];
        items.extend(values.into_iter().map(|v| (v.clone(), json!({key: v.clone()}), current.as_deref() == Some(v.as_str()))));
        picker(app, ui, id, &shown, current.is_some(), items);
    }
    action_buttons(app, ui);
    rest_below
}

/// The rating (operator and stars), flag and colour-label controls, each group after its caption.
fn marks(app: &mut EvoLightApp, ui: &mut egui::Ui, f: &Filter) {
    let t = Tokens::get(ui.ctx());
    // rating: operator + stars
    caption(ui, "Rating");
    let op_label = match f.rating_op {
        RatingOp::AtLeast => "≥",
        RatingOp::Exactly => "=",
        RatingOp::AtMost => "≤",
    };
    picker(
        app,
        ui,
        "ratingOp",
        op_label,
        false,
        [("≥  at least", "atLeast", RatingOp::AtLeast), ("=  exactly", "exactly", RatingOp::Exactly), ("≤  at most", "atMost", RatingOp::AtMost)]
            .into_iter()
            .map(|(l, k, op)| (l.to_string(), json!({"ratingOp": k}), f.rating_op == op))
            .collect(),
    );
    if let Some(r) = crate::rating::Rating::new("filter", f.rating).automation_ids("filter:star").show(ui) {
        filter(app, json!({"rating": r}));
    }
    ui.add_space(GROUP_GAP);

    // flags: several can be on at once (any of them matches)
    caption(ui, "Flag");
    let chosen: Vec<Flag> = f.flags_shown().unwrap_or_default();
    for (id, icon, flag, tip) in [
        ("pick", Icon::FlagPick, Flag::Pick, "Picked"),
        ("reject", Icon::FlagReject, Flag::Reject, "Rejected"),
        ("none", Icon::Circle, Flag::None, "Unflagged"),
    ] {
        let on = chosen.contains(&flag);
        let color = match flag {
            Flag::Pick if on => t.pick,
            Flag::Reject if on => t.reject,
            _ => Color32::TRANSPARENT,
        };
        let icon = if on && flag == Flag::Pick { Icon::FlagPickFilled } else { icon };
        let resp = toggle(ui, id, on, tip, |p, r, c| paint(p, r, icon, if color == Color32::TRANSPARENT { c } else { color }));
        if resp.clicked() {
            // in the bar's order, so the chip reads the same whichever was clicked first
            let names: Vec<String> = [Flag::Pick, Flag::Reject, Flag::None]
                .into_iter()
                .filter(|x| if *x == flag { !on } else { chosen.contains(x) })
                .map(|x| format!("{x:?}").to_lowercase())
                .collect();
            filter(app, json!({"flag": Value::Null, "flags": names}));
        }
    }
    ui.add_space(GROUP_GAP);

    // colour labels
    caption(ui, "Label");
    // several labels can be on at once (any of them matches); lit are those `label` and `labels` both let through
    let chosen: Vec<ColorLabel> = f.labels_shown().unwrap_or_default();
    for l in ColorLabel::ALL {
        let on = chosen.contains(&l);
        let name = format!("{l:?}").to_lowercase();
        let tip = crate::i18n::tr_format!("{} label (click more labels to show any of them)", crate::i18n::color_label(&app.session.catalog, l));
        let resp = toggle(ui, &format!("label-{name}"), on, &tip, |p, r, _| {
            p.circle_filled(r.center(), 5.5, label_color(l));
        });
        if resp.clicked() {
            let mut next = chosen.clone();
            if on {
                next.retain(|x| *x != l);
            } else {
                next.push(l);
            }
            next.sort();
            let names: Vec<String> = next.iter().map(|x| format!("{x:?}").to_lowercase()).collect();
            filter(app, json!({"label": Value::Null, "labels": names}));
        }
    }
    ui.add_space(GROUP_GAP);
}

/// The labels of the actions that end the bar, translated: Save as Smart Album, Clear and the
/// filter presets menu.
fn action_labels() -> [&'static str; 3] {
    [crate::i18n::tr("Save as Smart Album…"), crate::i18n::tr("Clear"), crate::i18n::tr("Presets")]
}

/// Save as Smart Album, Clear and the filter presets menu, after a space.
fn action_buttons(app: &mut EvoLightApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    let [save_label, clear_label, presets_label] = action_labels();
    // save / clear
    ui.add_space(GROUP_GAP);
    let filtering = app.session.filter != Default::default();
    let save = ui.add_enabled(filtering, egui::Button::new(egui::RichText::new(save_label).font(t.font(12.5))));
    register(ui, "button:filterSave", save.rect);
    if save.clicked() {
        let _ = app.run("dialog.newSmartAlbum", json!({}));
    }
    let clear = ui.add_enabled(filtering, egui::Button::new(egui::RichText::new(clear_label).font(t.font(12.5))));
    register(ui, "button:filterClear", clear.rect);
    if clear.clicked() {
        app.ui.search.clear();
        let _ = app.run("library.clearFilter", json!({}));
    }
    // filter presets: apply one, save the current filter, delete
    let presets = crate::widgets::dropdown(ui, "filterPresets", presets_label, t.font(12.5), t.text);
    register(ui, "button:filterPresets", presets.rect);
    egui::Popup::menu(&presets).show(|ui| {
        let names: Vec<String> = app.session.filter_presets.iter().map(|f| f.name.clone()).collect();
        for n in &names {
            let r = ui.button(n);
            if r.clicked() {
                let _ = app.run("filter.applyPreset", json!({"name": n}));
            }
            crate::widgets::context_menu(&r, |ui| {
                if ui.button(crate::i18n::tr("Delete Preset")).clicked() {
                    let _ = app.run("filter.deletePreset", json!({"name": n}));
                }
            });
        }
        if names.is_empty() {
            ui.label(egui::RichText::new(crate::i18n::tr("No filter presets yet")).color(t.text_dim));
        }
        ui.separator();
        if ui.add_enabled(filtering, egui::Button::new(crate::i18n::tr("Save Current Filter as Preset…"))).clicked() {
            crate::panels::dialogs::prompt(app, "Save Filter Preset", "Preset name", "", "filter.savePreset", json!({}), "name");
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::{HEIGHT, metadata_text};
    use crate::headless::Headless;
    use crate::i18n::{Locale, set_language};
    use crate::{EvoLightApp, Services};

    #[test]
    fn a_metadata_picker_shows_the_value_picked_cut_short_or_what_it_picks_from() {
        assert_eq!(metadata_text(None, "Camera"), "Camera");
        assert_eq!(metadata_text(Some("X-T5"), "Camera"), "X-T5");
        assert_eq!(metadata_text(Some("abcdefghijklmnopqrstuv"), "Lens"), "abcdefghijklmnopqrstuv", "22 characters fit");
        assert_eq!(metadata_text(Some("abcdefghijklmnopqrstuvw"), "Lens"), "abcdefghijklmnopqrstu…");
    }

    /// The bar takes a second row on the widths its controls have in the language shown: the
    /// longer French and German labels never push the last controls past its right edge (as a
    /// fixed width threshold set for English did), and a wide bar keeps them all on one row.
    #[test]
    fn the_filter_bar_keeps_every_control_inside_it_in_longer_languages() {
        let t = Duration::from_secs(10);
        let controls = [
            "dropdown:filter-kind",
            "dropdown:filter-edited",
            "dropdown:filter-camera",
            "dropdown:filter-lens",
            "dropdown:filter-keyword",
            "button:filterSave",
            "button:filterClear",
            "button:filterPresets",
        ];
        for width in [1600.0, 1900.0, 2300.0] {
            let mut app = EvoLightApp::new(evolight_engine::Session::with_demo(), Services { png: None, ..Default::default() });
            app.ui.view = crate::state::ViewMode::PhotoGrid;
            let mut h = Headless::new(app, [width, 800.0], 1.0);
            h.request("engine.execute", json!({"command": "view.filterBar"}), t);
            for locale in [Locale::En, Locale::Fr, Locale::De] {
                h.app.ui.language = locale;
                set_language(locale);
                h.step();
                h.step();
                let rect = |id: &str| h.app.widgets.iter().find(|(w, _)| w == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no widget {id}"));
                let bar = rect("filterbar");
                for id in controls {
                    let r = rect(id);
                    assert!(r.right() <= bar.right() && r.bottom() <= bar.bottom(), "{locale:?} at {width}: {id} {r:?} is outside the bar {bar:?}");
                }
                if width > 2000.0 {
                    assert!(bar.height() < HEIGHT + 0.5, "{locale:?} at {width}: one row is enough, the bar is {bar:?}");
                }
            }
            h.app.ui.language = Locale::En;
            set_language(Locale::En);
            h.settle(t);
        }
    }
}
