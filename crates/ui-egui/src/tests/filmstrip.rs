//! Headless tests of the filmstrip: file-name labels (#266) — a name with multi-byte (CJK)
//! characters is drawn without panicking — its stack badges, which expand / collapse a stack, and
//! its height: within `FILM_HEIGHT`, lowered so the photo keeps `MIN_FILM_CANVAS`, changed by the
//! grip along its top edge, with cells that keep their proportion and a name that gives way to the
//! format in a narrow cell.

use std::time::Duration;

use egui::Rect;
use evolight_catalog::{Op, Stack, StackId};
use serde_json::{Value, json};

use crate::headless::Headless;
use crate::state::{FILM_HEIGHT, MIN_FILM_CANVAS, ViewMode};
use crate::tests_fixtures::SyntheticLibrary;
use crate::{EvoLightApp, Services};

const T: Duration = Duration::from_secs(20);
const SETTLE: Duration = Duration::from_secs(120);

/// A filmstrip cell's width at the strip's default height.
const CELL_W: f32 = 120.0;

#[test]
fn filmstrip_shows_a_cjk_file_name() {
    let mut session = evolight_engine::Session::with_demo();
    let first = session.visible_cloned()[0];
    let source = session.catalog.photo(first).expect("demo photo").source.clone();
    // the name from #266: cutting it at byte 13 lands inside '限'
    session.catalog.apply(Op::SetFile { id: first, file_name: "202407層三限定訂閱圖(4).jpg".into(), source }).unwrap();
    let app = EvoLightApp::new(session, Services { png: None, ..Default::default() });
    let mut h = Headless::new(app, [1200.0, 800.0], 1.0);
    let r = h.request("ui.set", json!({"view": "detail"}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.settle(SETTLE);
    assert!(h.app.ui.settings.film_names);
    let film = format!("film:{}", first.0);
    assert!(h.app.widgets.iter().any(|(w, _)| *w == film), "{film} drawn in the filmstrip");
}

#[test]
fn filmstrip_stack_badge_expands_and_collapses_the_stack() {
    // Badge routing needs three real photos, not full 24 MP demo renders.
    let library = crate::tests_fixtures::SyntheticLibrary::with_photos("filmstrip-stack-badge", 3);
    let mut app = library.app();
    let vis = app.session.visible_cloned();
    let (loose, top, member) = (vis[0], vis[1], vis[2]);
    let stack = Stack { id: StackId(1), photos: vec![top, member], collapsed: true };
    app.session.catalog.apply(Op::AddStack { stack }).unwrap();
    let mut h = Headless::new(app, [1200.0, 800.0], 1.0);
    // the loose photo is the one in the loupe: the stack isn't selected
    let r = h.request("engine.execute", json!({"command": "library.select", "params": {"ids": [loose.0]}}), T);
    assert_eq!(r["ok"], true, "{r}");
    let r = h.request("ui.set", json!({"view": "detail"}), T);
    assert_eq!(r["ok"], true, "{r}");
    assert!(h.settle(SETTLE), "the filmstrip settles before clicking its stack badge");
    assert!(!h.app.session.visible_cloned().contains(&member), "collapsed: only the top shows");
    let badge = format!("filmstack:{}", top.0);
    let r = h.request("ui.clickWidget", json!({"id": badge}), T);
    assert_eq!(r["ok"], true, "{r}");
    assert!(h.settle(SETTLE), "the expanded stack settles");
    assert!(!h.app.session.catalog.stack(StackId(1)).unwrap().collapsed, "a click on the badge expands the stack");
    assert!(h.app.session.visible_cloned().contains(&member));
    assert_eq!(h.app.session.selection.active, Some(loose), "the loupe keeps its photo");
    let r = h.request("ui.clickWidget", json!({"id": badge}), T);
    assert_eq!(r["ok"], true, "{r}");
    assert!(h.settle(SETTLE), "the collapsed stack settles");
    assert!(h.app.session.catalog.stack(StackId(1)).unwrap().collapsed, "a second click collapses it");
    assert!(!h.app.session.visible_cloned().contains(&member));
}

/// The filmstrip rings its selected photos by the same rule as the grid, in the theme's colours:
/// the active photo's ring is heavier than the other selected photo's, and a light theme gets no
/// fixed white or grey ring.
#[test]
fn the_filmstrip_marks_the_active_photo_heavier_than_the_other_selected_one() {
    use egui::epaint::Shape;
    let mut session = evolight_engine::Session::with_demo();
    let ids = session.visible_cloned();
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    let app = EvoLightApp::new(session, Services { png: None, ..Default::default() });
    let mut h = Headless::new(app, [1200.0, 800.0], 1.0);
    h.app.ui.settings.appearance_mode = crate::state::AppearanceMode::Light;
    let r = h.request("ui.set", json!({"view": "detail"}), T);
    assert_eq!(r["ok"], true, "{r}");
    let r = h.request("engine.execute", json!({"command": "library.select", "params": {"ids": [a.0, b.0]}}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.settle(SETTLE);
    let t = crate::theme::Tokens::get(&h.view.ctx);
    assert!(crate::theme::current(&h.view.ctx).is_some_and(|k| !k.is_dark()));
    let active = h.app.session.selection.active.expect("active");
    let other = if active == a { b } else { a };
    let shapes = h.view.painted_shapes();
    let strokes = |id: evolight_catalog::PhotoId| -> Vec<(f32, egui::Color32)> {
        let cell = h.app.widgets.iter().find(|(w, _)| *w == format!("film:{}", id.0)).map(|(_, r)| *r).expect("cell drawn");
        shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Rect(rs) if rs.stroke.width > 0.0 && rs.stroke_kind == egui::StrokeKind::Outside && cell.contains_rect(rs.rect) => {
                    Some((rs.stroke.width, rs.stroke.color))
                }
                _ => None,
            })
            .collect()
    };
    let (on_active, on_other, on_unselected) = (strokes(active), strokes(other), strokes(c));
    assert_eq!(on_active, vec![(3.0, t.ring_active), (1.0, t.ring_inner)]);
    assert_eq!(on_other, vec![(1.5, t.ring_selected)]);
    assert!(on_unselected.is_empty(), "{on_unselected:?}");
}

// ------------------------------------------------------------------------ height

/// The Detail view of `library` in a window of `size` points, settled.
fn detail(library: &SyntheticLibrary, size: [f32; 2]) -> Headless {
    let mut h = Headless::new(library.app(), size, 1.0);
    set(&mut h, json!({"view": "detail"}));
    assert!(h.settle(SETTLE), "the Detail view settles");
    h
}

/// `ui.set` `ui`, then one more frame.
fn set(h: &mut Headless, ui: Value) {
    let r = h.request("ui.set", ui, T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
}

fn widget(h: &Headless, id: &str) -> Rect {
    h.app.widgets.iter().rev().find(|(w, _)| w == id).map(|(_, r)| *r).unwrap_or_else(|| panic!("no widget {id}"))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The view's photo area with the filmstrip hidden: the strip, once shown, takes its bottom part.
fn photo_area(h: &mut Headless) -> Rect {
    h.app.ui.filmstrip = false;
    h.step();
    let area = h.app.canvas_rect.expect("the photo area is drawn");
    h.app.ui.filmstrip = true;
    h.step();
    h.step();
    area
}

/// The window resized to `height` points (its width kept), and the photo area it then has.
fn resized(h: &mut Headless, height: f32) -> Rect {
    let width = h.size.x;
    let r = h.request("ui.resize", json!({"width": width, "height": height}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
    photo_area(h)
}

/// The height the filmstrip is drawn at in `area` (see [`photo_area`]): from its grip, which runs
/// along its top edge against the photo, down to the bottom of `area`.
fn strip_height(h: &Headless, area: Rect) -> f32 {
    let canvas = h.app.canvas_rect.expect("the photo area is drawn");
    let grip = widget(h, "filmstrip:grip");
    assert!(close(canvas.top(), area.top()), "the photo {canvas:?} starts at the top of {area:?}");
    assert!(close(grip.top(), canvas.bottom()), "the grip {grip:?} runs along the photo's bottom edge {canvas:?}");
    area.bottom() - grip.top()
}

/// Drag the filmstrip's grip `dy` points (up when negative), 10 points a frame.
fn drag_grip(h: &mut Headless, dy: f32) {
    let steps = (dy.abs() / 10.0).ceil().max(1.0) as u64;
    let r = h.request("ui.dragWidget", json!({"id": "filmstrip:grip", "dy": dy, "steps": steps}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
}

/// The filmstrip's cells drawn, left to right.
fn film_cells(h: &Headless) -> Vec<Rect> {
    let mut cells: Vec<Rect> = h.app.widgets.iter().filter(|(w, _)| w.starts_with("film:")).map(|(_, r)| *r).collect();
    cells.sort_by(|a, b| a.left().total_cmp(&b.left()));
    cells
}

/// `filmHeight` below 100 or above 300 points draws a strip of 100 or 300 points, the photo taking
/// the rest; a height saved out of range is brought within on load. A window too low for the strip
/// lowers it so the photo keeps `MIN_FILM_CANVAS`, down to the strip's own minimum, and the
/// member's height comes back once the window is tall again.
#[test]
fn the_filmstrip_height_keeps_to_its_limits_and_leaves_the_photo_room() {
    let library = SyntheticLibrary::with_photos("filmstrip-height-limits", 6);
    let mut h = detail(&library, [1300.0, 1000.0]);
    let area = photo_area(&mut h);
    assert!(area.height() >= FILM_HEIGHT.max + MIN_FILM_CANVAS, "room for the tallest strip: {area:?}");
    assert!(close(strip_height(&h, area), FILM_HEIGHT.default), "the default height");
    let heights = [
        (40.0, FILM_HEIGHT.min),
        (FILM_HEIGHT.min, FILM_HEIGHT.min),
        (220.0, 220.0),
        (FILM_HEIGHT.max, FILM_HEIGHT.max),
        (900.0, FILM_HEIGHT.max),
    ];
    for (asked, drawn) in heights {
        set(&mut h, json!({"filmHeight": asked}));
        let inspect = h.request("ui.inspect", json!({}), T);
        let reported = inspect["result"]["ui"]["filmHeight"].as_f64().map(|v| v as f32);
        assert_eq!(reported, Some(drawn), "filmHeight {asked}: ui.inspect reports {drawn}");
        let strip = strip_height(&h, area);
        assert!(close(strip, drawn), "filmHeight {asked}: a strip of {strip} points, not {drawn}");
        let canvas = h.app.canvas_rect.expect("the photo area is drawn");
        assert!(close(canvas.height(), area.height() - drawn), "filmHeight {asked}: the photo takes the rest, {canvas:?}");
    }
    // the protocol's and the saved state's `filmHeight`, brought within the limits on load
    let read: crate::UiState = serde_json::from_value(json!({"filmHeight": 220.0})).expect("a UI state");
    assert_eq!(read.film_height, 220.0);
    let loaded = |v: f32| crate::UiState { film_height: v, ..Default::default() }.sanitized().film_height;
    assert_eq!((loaded(40.0), loaded(900.0), loaded(f32::NAN)), (FILM_HEIGHT.min, FILM_HEIGHT.max, FILM_HEIGHT.default));
    // a window too low for the tallest strip: the strip gives way, the photo keeps its minimum
    set(&mut h, json!({"filmHeight": FILM_HEIGHT.max}));
    let chrome = h.size.y - area.height();
    let low = resized(&mut h, chrome + MIN_FILM_CANVAS + 180.0);
    let room = low.height() - MIN_FILM_CANVAS;
    assert!(room > FILM_HEIGHT.min && room < FILM_HEIGHT.max, "a low window: {low:?}");
    let strip = strip_height(&h, low);
    assert!(close(strip, room), "the strip gives way: {strip} points in {low:?}");
    assert!(close(h.app.canvas_rect.expect("the photo area is drawn").height(), MIN_FILM_CANVAS), "the photo keeps its minimum");
    // lower still: the strip stays at its own minimum and the photo has what is left
    let lower = resized(&mut h, chrome + MIN_FILM_CANVAS + 40.0);
    assert!(lower.height() - MIN_FILM_CANVAS < FILM_HEIGHT.min, "a very low window: {lower:?}");
    assert!(close(strip_height(&h, lower), FILM_HEIGHT.min), "never below the strip's minimum");
    assert!(close(h.app.canvas_rect.expect("the photo area is drawn").height(), lower.height() - FILM_HEIGHT.min));
    // the member's height was kept: a tall window shows it again
    assert_eq!(h.app.ui.film_height, FILM_HEIGHT.max);
    let tall = resized(&mut h, 1000.0);
    assert!(close(strip_height(&h, tall), FILM_HEIGHT.max), "the member's height is back");
}

/// The grip along the filmstrip's top edge, as wide as the strip and above its cells: the pointer
/// shows a vertical resize over it, a drag up makes the strip taller and a drag down shorter, never
/// past `FILM_HEIGHT`, and a double click gives back the default height.
#[test]
fn the_filmstrip_grip_resizes_the_strip_and_a_double_click_restores_it() {
    let library = SyntheticLibrary::with_photos("filmstrip-grip", 6);
    let mut h = detail(&library, [1300.0, 1000.0]);
    let area = photo_area(&mut h);
    assert!(area.height() >= FILM_HEIGHT.max + MIN_FILM_CANVAS + 60.0, "room to drag: {area:?}");
    let grip = widget(&h, "filmstrip:grip");
    let strip = widget(&h, "filmstrip");
    assert!(close(grip.left(), area.left()) && close(grip.right(), area.right()), "the grip {grip:?} spans {area:?}");
    // (within a point: egui may round the cells' area to pixels)
    assert!(grip.height() > 0.0 && (grip.bottom() - strip.top()).abs() <= 1.0, "the cells start below the grip: {grip:?} {strip:?}");
    assert!(close(strip_height(&h, area), FILM_HEIGHT.default));
    let r = h.request("ui.hoverWidget", json!({"id": "filmstrip:grip"}), T);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(h.last_cursor, egui::CursorIcon::ResizeVertical, "the pointer shows a vertical resize");
    // up 60 points: about as much taller (the press may spend a step telling a drag from a click)
    drag_grip(&mut h, -60.0);
    let taller = strip_height(&h, area);
    assert!(taller > FILM_HEIGHT.default + 45.0 && taller < FILM_HEIGHT.default + 61.0, "{} → {taller}", FILM_HEIGHT.default);
    assert!(close(h.app.ui.film_height, taller), "the strip is drawn at the member's height {}", h.app.ui.film_height);
    let inspect = h.request("ui.inspect", json!({}), T);
    let reported = inspect["result"]["ui"]["filmHeight"].as_f64().map(|v| v as f32);
    assert_eq!(reported, Some(h.app.ui.film_height), "ui.inspect reports it as filmHeight");
    // down 80 points: shorter
    drag_grip(&mut h, 80.0);
    let shorter = strip_height(&h, area);
    assert!(shorter < taller - 65.0 && shorter > taller - 81.0, "{taller} → {shorter}");
    // never past the limits, however far it goes
    drag_grip(&mut h, -250.0);
    assert!(close(strip_height(&h, area), FILM_HEIGHT.max), "at most {}", FILM_HEIGHT.max);
    drag_grip(&mut h, 250.0);
    assert!(close(strip_height(&h, area), FILM_HEIGHT.min), "at least {}", FILM_HEIGHT.min);
    // a double click gives back the default height
    let r = h.request("ui.clickWidget", json!({"id": "filmstrip:grip", "count": 2}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
    assert_eq!(h.app.ui.film_height, FILM_HEIGHT.default);
    assert!(close(strip_height(&h, area), FILM_HEIGHT.default), "the default height is back");
}

/// The cells keep the proportion they have at the default height (`CELL_W` wide for 142 high):
/// round(120 × h / 142) points wide for a strip `h` high, side by side.
#[test]
fn filmstrip_cells_keep_their_proportion_as_the_strip_is_resized() {
    let library = SyntheticLibrary::with_photos("filmstrip-cells", 6);
    let mut h = detail(&library, [1300.0, 1000.0]);
    let area = photo_area(&mut h);
    for height in [FILM_HEIGHT.default, FILM_HEIGHT.min, 250.0] {
        set(&mut h, json!({"filmHeight": height}));
        assert!(close(strip_height(&h, area), height), "a strip {height} high");
        let want = (CELL_W * height / FILM_HEIGHT.default).round();
        let cells = film_cells(&h);
        assert!(cells.len() >= 3, "{height}: {} cells drawn", cells.len());
        for cell in &cells {
            assert!(close(cell.width(), want), "{height}: a cell {} wide, not {want}", cell.width());
        }
        for pair in cells.windows(2) {
            assert!(close(pair[1].left() - pair[0].left(), want), "{height}: cells side by side, {pair:?}");
        }
    }
}

/// In a cell of the lowest strip, a long file name gives way to the format: cut short with …, it
/// ends before the format, and both stay inside the cell.
#[test]
fn a_long_file_name_is_cut_short_before_the_format_in_a_narrow_cell() {
    let library = SyntheticLibrary::with_photos("filmstrip-long-name", 3);
    let mut app = library.app();
    let first = app.session.visible_cloned()[0];
    let source = app.session.catalog.photo(first).expect("a photo").source.clone();
    app.session.catalog.apply(Op::SetFile { id: first, file_name: "IMG_20240712_153012.png".into(), source }).unwrap();
    let mut h = Headless::new(app, [1300.0, 1000.0], 1.0);
    set(&mut h, json!({"view": "detail", "filmHeight": FILM_HEIGHT.min}));
    assert!(h.settle(SETTLE), "the Detail view settles");
    assert!(h.app.ui.settings.film_names, "file names are shown in the strip");
    let cell = widget(&h, &format!("film:{}", first.0));
    let texts: Vec<egui::epaint::TextShape> = h
        .view
        .painted_shapes()
        .into_iter()
        .filter_map(|s| match s {
            egui::epaint::Shape::Text(t) if cell.contains(t.pos) => Some(t),
            _ => None,
        })
        .collect();
    let name = texts.iter().find(|t| t.galley.job.text == "IMG_20240712_…").expect("the name, cut to 13 characters");
    let format = texts.iter().find(|t| t.galley.job.text == "PNG").expect("the format");
    let (name_r, format_r) = (name.visual_bounding_rect(), format.visual_bounding_rect());
    assert!(name.galley.elided, "a {} wide cell cuts the name short", cell.width());
    assert!(name_r.right() <= format_r.left(), "the name {name_r:?} ends before the format {format_r:?}");
    assert!(cell.contains_rect(name_r) && cell.contains_rect(format_r), "both inside {cell:?}: {name_r:?} {format_r:?}");
}

/// Compare draws the same filmstrip at the member's height, and its grip works there too.
#[test]
fn compare_draws_the_filmstrip_at_the_members_height() {
    let library = SyntheticLibrary::with_photos("filmstrip-compare-height", 6);
    let mut h = detail(&library, [1300.0, 1000.0]);
    set(&mut h, json!({"filmHeight": 260.0}));
    let vis: Vec<u64> = h.app.session.visible_cloned().iter().map(|p| p.0).collect();
    h.app.run("library.select", json!({"ids": [vis[0], vis[1]], "active": vis[0]})).unwrap();
    h.app.run("view.compare", json!({})).unwrap();
    assert!(h.settle(SETTLE), "Compare settles");
    assert_eq!(h.app.ui.view, ViewMode::Compare);
    let area = photo_area(&mut h);
    assert!(close(strip_height(&h, area), 260.0), "the member's height in Compare too: {area:?}");
    let r = h.request("ui.clickWidget", json!({"id": "filmstrip:grip", "count": 2}), T);
    assert_eq!(r["ok"], true, "{r}");
    h.step();
    assert!(close(strip_height(&h, area), FILM_HEIGHT.default), "a double click gives back the default height");
}
