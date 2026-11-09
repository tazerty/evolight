//! The segmented control (`widgets::segmented`) keeps every label inside its segment: a label wider
//! than the whole control is cut short with … in a segment as wide as the control, still named in
//! full for screen readers, while short labels keep their equal shares.

use egui::{Rect, vec2};

use crate::headless::HeadlessView;

/// The window's width: room for three short labels side by side, far too little for [`LONG`].
const WIDTH: f32 = 200.0;
const LONG: &str = "A label far longer than this whole control could ever show";
const SHORT: [(&str, &str); 3] = [("One", "one"), ("Two", "two"), ("Six", "six")];

/// What the last frame of the two controls left.
struct Drawn {
    texts: Vec<egui::epaint::TextShape>,
    widgets: Vec<(String, Rect)>,
    /// The names a screen reader has.
    names: Vec<String>,
    /// The width the controls were given.
    width: f32,
    /// [`LONG`] on one line, in the controls' font.
    long_width: f32,
}

fn text_shapes(shape: &egui::Shape, out: &mut Vec<egui::epaint::TextShape>) {
    match shape {
        egui::Shape::Text(text) => out.push(text.clone()),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| text_shapes(s, out)),
        _ => {}
    }
}

/// Draw, one below the other in a window `WIDTH` wide: a control with [`LONG`] and a short label,
/// two to a row at most, then one with the [`SHORT`] labels, three to a row.
fn draw() -> Drawn {
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    crate::theme::apply(&ctx);
    ctx.enable_accesskit();
    let mut drawn = Drawn { texts: Vec::new(), widgets: Vec::new(), names: Vec::new(), width: 0.0, long_width: 0.0 };
    for frame in 0..3 {
        let raw = HeadlessView::raw_input(vec2(WIDTH, 400.0), 1.0, frame as f64 / 60.0, vec![]);
        let (mut width, mut long_width) = (0.0, 0.0);
        let mut out = ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
                let t = crate::theme::Tokens::get(ui.ctx());
                width = ui.available_width();
                long_width = ui.painter().layout_no_wrap(LONG.to_string(), t.semibold(11.5), t.text).size().x;
                let _ = crate::widgets::segmented(ui, "long", &[(LONG, "long"), ("Off", "off")], None, 2);
                let _ = crate::widgets::segmented(ui, "short", &SHORT, None, 3);
            });
        });
        out.textures_delta.clear();
        drawn.texts.clear();
        for shape in &out.shapes {
            text_shapes(&shape.shape, &mut drawn.texts);
        }
        drawn.widgets = crate::widgets::take_registry(&ctx);
        if let Some(update) = out.platform_output.accesskit_update.take() {
            drawn.names = update.nodes.iter().map(|(_, n)| n.label().unwrap_or_default().to_string()).collect();
        }
        (drawn.width, drawn.long_width) = (width, long_width);
    }
    drawn
}

fn segment(d: &Drawn, id: &str) -> Rect {
    d.widgets.iter().rev().find(|(w, _)| w == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no segment {id}: {:?}", d.widgets))
}

fn text<'a>(d: &'a Drawn, label: &str) -> &'a egui::epaint::TextShape {
    d.texts.iter().find(|t| t.galley.job.text == label).unwrap_or_else(|| panic!("{label:?} is not painted"))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// A label wider than the whole control: one segment to a row, each as wide as the control; the
/// long label is cut short with … inside its segment (named in full for a screen reader), the
/// short one beside it in the same control is drawn whole.
#[test]
fn a_label_wider_than_the_control_is_cut_short_in_a_full_width_segment() {
    let d = draw();
    assert!(d.long_width > d.width, "the label is wider than the control: {} for {}", d.long_width, d.width);
    let (long, off) = (segment(&d, "button:long-long"), segment(&d, "button:long-off"));
    assert!(close(long.width(), d.width) && close(off.width(), d.width), "{} wide segments: {long:?} {off:?}", d.width);
    assert!(off.top() >= long.bottom(), "the short label goes on the next row: {long:?} {off:?}");
    let cut = text(&d, LONG);
    assert!(cut.galley.elided, "the label is cut short");
    assert!(long.contains_rect(cut.visual_bounding_rect()), "{:?} stays inside {long:?}", cut.visual_bounding_rect());
    let whole = text(&d, "Off");
    assert!(!whole.galley.elided, "a short label is drawn whole");
    assert!(off.contains_rect(whole.visual_bounding_rect()), "{:?} stays inside {off:?}", whole.visual_bounding_rect());
    assert!(d.names.iter().any(|n| n == LONG), "a screen reader has the label in full: {:?}", d.names);
}

/// Labels that fit their share keep equal shares, side by side on one row filling the control, and
/// are drawn whole.
#[test]
fn short_labels_keep_equal_shares() {
    let d = draw();
    let segments: Vec<Rect> = SHORT.iter().map(|(_, key)| segment(&d, &format!("button:short-{key}"))).collect();
    let first = segments[0];
    for (r, (label, _)) in segments.iter().zip(SHORT) {
        assert!(close(r.width(), first.width()) && close(r.top(), first.top()), "{label}: {r:?} beside {first:?}");
        let t = text(&d, label);
        assert!(!t.galley.elided, "{label} is drawn whole");
        assert!(r.contains_rect(t.visual_bounding_rect()), "{label}: {:?} stays inside {r:?}", t.visual_bounding_rect());
    }
    let span = segments[2].right() - first.left();
    assert!(close(span, d.width), "the row fills the control, {} wide: {segments:?}", d.width);
}
