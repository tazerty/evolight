//! Design tokens (colours, sizes, fonts). Values measured from black-box observation of the
//! reference app's dark theme (see plan/lightroom/10-observed-ui.md); everything is our own code.
//!
//! Themes come in two families, picked by the Appearance Mode setting (Settings ▸ Interface):
//! - dark: **Charcoal** (the default, the observed dark look) and **Midnight** (near black);
//! - light: **Silver** (light grey chrome) and **Paper** (near white).

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, Visuals};

pub const FONT_SEMIBOLD: &str = "semibold";
const WEB_CHINESE_FONT: &str = "web Noto Sans CJK SC";

/// Shared colour-label palette for badges, thumbnail surrounds and feedback.
pub fn label_color(label: evolight_catalog::ColorLabel) -> Color32 {
    use evolight_catalog::ColorLabel;
    match label {
        ColorLabel::Red => Color32::from_rgb(222, 72, 72),
        ColorLabel::Yellow => Color32::from_rgb(232, 196, 58),
        ColorLabel::Green => Color32::from_rgb(88, 176, 92),
        ColorLabel::Blue => Color32::from_rgb(72, 130, 222),
        ColorLabel::Purple => Color32::from_rgb(158, 100, 210),
    }
}

/// A translucent label colour over thumbnail chrome; selection remains brighter.
pub fn label_background(base: Color32, label: Option<evolight_catalog::ColorLabel>, selected: bool) -> Color32 {
    label.map_or(base, |l| base.lerp_to_gamma(label_color(l), if selected { 0.32 } else { 0.22 }))
}

/// Evora (E2, EVORA-REV-104) : le contraste minimal d'un texte de corps (WCAG 2, niveau AA).
pub const MIN_TEXT_CONTRAST: f32 = 4.5;

/// Evora (E2, EVORA-REV-104) : la part de `cell_selected` dans une cellule survolée de la bande de
/// miniatures, posée en transparence sur le `canvas` de la bande.
pub const CELL_HOVER_FILL: f32 = 0.6;

/// Evora (E2, EVORA-REV-104) : la couleur d'un texte secondaire posé sur `background`, comme le nom
/// et le format sous une vignette, sur une cellule choisie ou teintée par une étiquette. Sous
/// l'identité d'Evora, le premier des gris `text_dim` puis `text_label` qui atteint
/// [`MIN_TEXT_CONTRAST`] sur ce fond, `text` sinon. Hors d'Evora, `text_dim`, comme dans l'amont.
/// Le repli sur `text` ne vaut que pour un fond de la même polarité que le thème (les cellules de
/// la bande et de la grille, testées) : sur une pastille toujours sombre d'un thème clair, il
/// faut une encre fixe.
pub fn secondary_text_on(t: &Tokens, background: Color32) -> Color32 {
    if !evora_palette() {
        return t.text_dim;
    }
    [t.text_dim, t.text_label].into_iter().find(|ink| crate::button::contrast(*ink, background) >= MIN_TEXT_CONTRAST).unwrap_or(t.text)
}

/// Evora (EVORA-REV-105, E2) : le bord des boutons propres de l'application (bouton secondaire,
/// bouton texte, segments). Sous l'identité d'Evora, celui des champs, que les contrôles natifs
/// d'egui ont depuis EVORA-REV-104 (`field_border` : 3,20:1 sur le chrome en sombre, 3,45:1 en
/// clair) ; `button_border` (#333333 en sombre, 1,35:1) laissait les segments presque invisibles
/// sur le graphite. Hors d'Evora, `button_border`, comme dans l'amont. Les séparateurs des barres
/// gardent `button_border`.
pub fn control_border(t: &Tokens) -> Color32 {
    if evora_palette() { t.field_border } else { t.button_border }
}

/// Evora (EVORA-REV-106, E2) : les encres d'un dessin posé sur un voile ou une pastille toujours
/// sombres, quel que soit le thème : marques d'une vignette sur leur voile noir, badge de format de
/// la grille, toast, étoile de favori d'un profil. Sous l'identité d'Evora, les jetons du sombre
/// d'Evora, clairs et fixes : en clair, ceux du thème (`star` #404040, `pick` #262626, `text`
/// #171717) s'y perdaient (1,06:1 à 1,68:1). Seules les couleurs sont à y lire. Hors d'Evora, les
/// jetons du thème, comme dans l'amont.
pub fn on_dark_veil(t: &Tokens) -> Tokens {
    if evora_palette() { evora_tokens(true) } else { *t }
}

/// Evora (EVORA-REV-106, E2) : l'encre d'un tracé dont l'amont fixe la couleur pour un fond sombre
/// (courbes, repères d'histogramme), alors qu'il est posé sur une surface du thème. Sous l'identité
/// d'Evora, `token`, lisible en clair comme en sombre ; hors d'Evora, `upstream`, la couleur de
/// l'amont.
pub fn evora_ink(token: Color32, upstream: Color32) -> Color32 {
    if evora_palette() { token } else { upstream }
}

/// Evora (EVORA-REV-106, E2) : la part de `text` dans le quadrillage d'un graphe, tiré de son fond.
/// Sur le canvas, en sombre, c'est le gris 52 de l'amont (1,45:1) ; en clair, #D2D2D2 (1,33:1).
pub const GRAPH_GRID_SHARE: f32 = 0.14;

/// Evora (EVORA-REV-106, E2) : la part de `text` dans la diagonale d'un graphe, tirée de son fond.
/// Sur le canvas, en sombre, c'est le gris 70 de l'amont (1,92:1) ; en clair, #BFBFBF (1,61:1).
pub const GRAPH_AXIS_SHARE: f32 = 0.225;

/// Evora (EVORA-REV-106, E2) : un filet de graphe (quadrillage, diagonale) posé sur `background`.
/// Sous l'identité d'Evora, ce fond tiré vers `text` de `share` : le filet reste discret dans les
/// deux polarités, au lieu d'un gris fixe presque noir sur un fond clair (10,92:1). Hors d'Evora,
/// `upstream`, la couleur de l'amont.
pub fn graph_rule(t: &Tokens, background: Color32, share: f32, upstream: Color32) -> Color32 {
    evora_ink(background.lerp_to_gamma(t.text, share), upstream)
}

/// Evora (EVORA-REV-106, E2) : l'hôte mène-t-il l'apparence ? Vrai sous le pont d'Evora, dont le
/// masque retire la commande d'apparence des menus et du clavier ([`crate::menubar::host_masked`]) :
/// la coquille choisit le mode clair ou sombre et l'envoie au moteur. Faux dans les builds propres
/// de l'application, qui n'ont pas de masque. Evora (EVORA-REV-108, E2) : faux aussi tant que
/// l'identité d'Evora n'est pas montrée ([`evora_palette`]) : un pont ouvert sans thème valable
/// laisse à l'application ses thèmes, donc son bouton et ses réglages d'apparence.
pub fn host_leads_appearance() -> bool {
    evora_palette() && crate::menubar::host_masked("view.appearance")
}

/// Label confirmations use a pale colour and dark text, distinct from neutral/error HUDs.
pub fn label_toast_colors(label: evolight_catalog::ColorLabel) -> (Color32, Color32) {
    (Color32::WHITE.lerp_to_gamma(label_color(label), 0.3), Color32::from_rgb(36, 24, 24))
}

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    /// Top bar, side panels, bottom bar, tool strip.
    pub chrome: Color32,
    /// Photo canvas (detail view) and filmstrip.
    pub canvas: Color32,
    /// Grid background behind the cells.
    pub grid_bg: Color32,
    pub cell: Color32,
    pub cell_selected: Color32,
    /// A thumbnail's place while its picture is still being made: static, apart from the grid, the
    /// cells and the canvas in every theme.
    pub skeleton: Color32,
    /// The ring round the active photo (outer line, 3 pt): 3:1 or more against the grid and the
    /// square grid's cells. The photo itself is parted from it by [`Tokens::ring_inner`].
    pub ring_active: Color32,
    /// The thin line (1 pt) between the active photo and its ring: the ring's opposite, so the ring
    /// reads on a photo of any brightness.
    pub ring_inner: Color32,
    /// The ring round the other selected photos (1.5 pt, against the active one's 3 pt): 3:1 or
    /// more against the grid.
    pub ring_selected: Color32,
    pub divider: Color32,
    pub inset: Color32,
    pub field: Color32,
    pub field_border: Color32,
    pub button: Color32,
    pub button_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub tool_active: Color32,
    pub text: Color32,
    pub text_label: Color32,
    pub text_dim: Color32,
    pub text_disabled: Color32,
    /// Icon glyphs in the chrome.
    pub icon: Color32,
    pub track: Color32,
    pub thumb: Color32,
    pub thumb_hover: Color32,
    pub accent: Color32,
    pub star: Color32,
    pub pick: Color32,
    pub reject: Color32,
    /// Cautionary notices (e.g. a raw shown from its embedded preview): a muted amber.
    pub caution: Color32,
    pub mask_overlay: Color32,
    // metrics (points)
    pub top_bar_h: f32,
    pub bottom_bar_h: f32,
    pub panel_w: f32,
    pub strip_w: f32,
    pub slider_row_h: f32,
    pub section_h: f32,
    pub film_h: f32,
}

/// An interface theme. Each belongs to the dark or the light family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeKind {
    #[default]
    Charcoal,
    Midnight,
    Silver,
    Paper,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 4] = [ThemeKind::Charcoal, ThemeKind::Midnight, ThemeKind::Silver, ThemeKind::Paper];

    pub fn is_dark(self) -> bool {
        matches!(self, ThemeKind::Charcoal | ThemeKind::Midnight)
    }

    /// The theme's id (settings value and command suffix).
    pub fn id(self) -> &'static str {
        match self {
            ThemeKind::Charcoal => "charcoal",
            ThemeKind::Midnight => "midnight",
            ThemeKind::Silver => "silver",
            ThemeKind::Paper => "paper",
        }
    }

    /// The English display name.
    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::Charcoal => "Charcoal",
            ThemeKind::Midnight => "Midnight",
            ThemeKind::Silver => "Silver",
            ThemeKind::Paper => "Paper",
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Tokens {
            chrome: Color32::from_rgb(0x2d, 0x2d, 0x2d),
            canvas: Color32::from_rgb(0x1c, 0x1c, 0x1c),
            grid_bg: Color32::from_rgb(0x0f, 0x0f, 0x0f),
            cell: Color32::from_rgb(0x1c, 0x1c, 0x1c),
            cell_selected: Color32::from_rgb(0x2d, 0x2d, 0x2d),
            skeleton: Color32::from_rgb(0x25, 0x25, 0x25),
            ring_active: Color32::from_rgb(0xff, 0xff, 0xff),
            ring_inner: Color32::from_rgb(0x00, 0x00, 0x00),
            ring_selected: Color32::from_rgb(0xc0, 0xc0, 0xc0),
            divider: Color32::from_rgb(0x1c, 0x1c, 0x1c),
            inset: Color32::from_rgb(0x23, 0x23, 0x23),
            field: Color32::from_rgb(0x22, 0x22, 0x22),
            field_border: Color32::from_rgb(0x3c, 0x3c, 0x3c),
            button: Color32::from_rgb(0x24, 0x24, 0x24),
            button_border: Color32::from_rgb(0x4c, 0x4c, 0x4c),
            hover: Color32::from_rgb(0x3a, 0x3a, 0x3a),
            pressed: Color32::from_rgb(0x3f, 0x3f, 0x3f),
            tool_active: Color32::from_rgb(0x3f, 0x3f, 0x3f),
            text: Color32::from_rgb(0xe2, 0xe2, 0xe2),
            text_label: Color32::from_rgb(0xbc, 0xbc, 0xbc),
            text_dim: Color32::from_rgb(0x8e, 0x8e, 0x8e),
            text_disabled: Color32::from_rgb(0x5c, 0x5c, 0x5c),
            icon: Color32::from_rgb(0x9a, 0x9a, 0x9a),
            track: Color32::from_rgb(0x5a, 0x5a, 0x5a),
            thumb: Color32::from_rgb(0xa0, 0xa0, 0xa0),
            thumb_hover: Color32::from_rgb(0xe0, 0xe0, 0xe0),
            accent: Color32::from_rgb(0x01, 0x65, 0xdd),
            star: Color32::from_rgb(0xd8, 0xd8, 0xd8),
            pick: Color32::from_rgb(0xf0, 0xf0, 0xf0),
            reject: Color32::from_rgb(0xe0, 0x4a, 0x4a),
            caution: Color32::from_rgb(0xe3, 0xa8, 0x3c),
            mask_overlay: Color32::from_rgba_unmultiplied(0xe0, 0x20, 0x30, 110),
            top_bar_h: 42.0,
            bottom_bar_h: 48.0,
            panel_w: 270.0,
            strip_w: 48.0,
            slider_row_h: 45.0,
            section_h: 57.0,
            film_h: 142.0,
        }
    }
}

thread_local! {
    /// Evora (E2): the host's identity in place of the themes' colours ([`set_evora_palette`]).
    static EVORA_PALETTE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Evora (EVORA-REV-33, REV-16): the accent the host sent (`evora:init`, `evora:theme`), shown
    /// by Evora's tokens in place of [`EVORA_ACCENT`] ([`set_evora_accent`]).
    static EVORA_ACCENT_SHOWN: std::cell::Cell<Color32> = const { std::cell::Cell::new(EVORA_ACCENT) };
}

/// Evora's accent colour (Tailwind indigo-600, `#4F46E5`): the default until the host sends one.
pub const EVORA_ACCENT: Color32 = Color32::from_rgb(0x4f, 0x46, 0xe5);

/// Evora (EVORA-REV-33, REV-16): the accent the host imposes (`evora:init.accent`,
/// `evora:theme.accent`), used by Evora's identity from the next [`apply_kind`].
pub fn set_evora_accent(accent: Color32) {
    EVORA_ACCENT_SHOWN.with(|shown| shown.set(accent));
}

/// The accent Evora's identity shows ([`set_evora_accent`]; [`EVORA_ACCENT`] by default).
pub fn evora_accent() -> Color32 {
    EVORA_ACCENT_SHOWN.with(std::cell::Cell::get)
}

/// Evora (E2, EVORA-REV-100) : montre l'identité d'Evora ([`evora_tokens`] : graphite neutre en
/// sombre, gris neutres en clair, accent de l'hôte ou [`EVORA_ACCENT`]) à la place des couleurs de
/// chaque thème, clair ou sombre selon le mode d'apparence. La police d'interface est déjà Inter.
/// L'hôte web la pose avant la première image (`?theme=`).
pub fn set_evora_palette(on: bool) {
    EVORA_PALETTE.with(|palette| palette.set(on));
}

/// Is Evora's identity shown ([`set_evora_palette`])?
pub fn evora_palette() -> bool {
    EVORA_PALETTE.with(std::cell::Cell::get)
}

/// Jetons d'Evora, sombres ou clairs (EVORA-REV-100, E2).
///
/// - Sombre : le graphite d'Evora, des gris strictement neutres (R = G = B, aucune dominante).
///   Entourage de l'image `#161616`, chrome des panneaux et des barres `#1C1C1C`, champs en creux
///   `#141414` bordés de `#6B6B6B` (3,46:1 sur le champ, 3,20:1 sur le chrome), ceux d'egui compris
///   ([`apply_kind`] en fait le bord au repos de ses contrôles natifs), filets de structure
///   `#0E0E0E`, sélection persistante `#383838`. Textes `#EBEBEB` (14,30:1 sur le
///   chrome), `#BDBDBD` (9,07:1) et `#949494` (5,62:1 ; 4,73:1 sur le survol).
/// - Clair : les mêmes rôles en gris neutres clairs. Ses anneaux étaient ceux du thème sombre de
///   l'amont, hérités de [`Tokens::default`] : un anneau blanc sur une grille presque blanche. Ils
///   sont désormais sombres sur clair, comme ceux des thèmes clairs de l'amont.
/// - Anneau des autres photos choisies, en sombre : `#BDBDBD` (le gris des libellés) plutôt que
///   `#A3A3A3`, qui ne donnait que 2,31:1 sur une cellule choisie teintée de jaune ; `#BDBDBD` y
///   tient 3,10:1 (le plancher de 3:1 est `#BBBBBB`).
/// - L'accent reste celui que l'hôte envoie ([`evora_accent`]), tel quel.
/// - Mesures : barre du haut 40, barre du bas 36, colonne d'outils 44 ; leur contenu, centré, tient
///   dans ces hauteurs. Les autres mesures restent celles de l'application : leur dessin a encore
///   des cotes fixes (rangée de curseur, cellule de la bande de vignettes).
fn evora_tokens(dark: bool) -> Tokens {
    let rgb = |v: u32| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    let base = Tokens { top_bar_h: 40.0, bottom_bar_h: 36.0, strip_w: 44.0, ..Tokens::default() };
    if dark {
        Tokens {
            chrome: rgb(0x1c1c1c),
            canvas: rgb(0x161616),
            grid_bg: rgb(0x121212),
            cell: rgb(0x1c1c1c),
            cell_selected: rgb(0x383838),
            // v0.14.0: a thumbnail's place while it is made, apart from the grid and the cells
            skeleton: rgb(0x2a2a2a),
            ring_active: rgb(0xf5f5f5),
            ring_inner: rgb(0x111111),
            ring_selected: rgb(0xbdbdbd),
            divider: rgb(0x0e0e0e),
            inset: rgb(0x141414),
            field: rgb(0x141414),
            field_border: rgb(0x6b6b6b),
            button: rgb(0x262626),
            button_border: rgb(0x333333),
            hover: rgb(0x2a2a2a),
            pressed: rgb(0x343434),
            tool_active: rgb(0x383838),
            text: rgb(0xebebeb),
            text_label: rgb(0xbdbdbd),
            text_dim: rgb(0x949494),
            text_disabled: rgb(0x5c5c5c),
            icon: rgb(0xc4c4c4),
            track: rgb(0x3d3d3d),
            thumb: rgb(0xdadada),
            thumb_hover: rgb(0xffffff),
            accent: evora_accent(),
            star: rgb(0xdadada),
            pick: rgb(0xf5f5f5),
            reject: rgb(0xf87171),
            caution: rgb(0xfbbf24),
            ..base
        }
    } else {
        Tokens {
            chrome: rgb(0xffffff),
            canvas: rgb(0xf0f0f0),
            grid_bg: rgb(0xf5f5f5),
            cell: rgb(0xebebeb),
            cell_selected: rgb(0xd6d6d6),
            skeleton: rgb(0xc2c2c2),
            ring_active: rgb(0x141414),
            ring_inner: rgb(0xffffff),
            ring_selected: rgb(0x4a4a4a),
            divider: rgb(0xe0e0e0),
            inset: rgb(0xf5f5f5),
            field: rgb(0xffffff),
            field_border: rgb(0x8a8a8a),
            button: rgb(0xffffff),
            button_border: rgb(0xd4d4d4),
            hover: rgb(0xebebeb),
            pressed: rgb(0xe0e0e0),
            tool_active: rgb(0xd6d6d6),
            text: rgb(0x171717),
            text_label: rgb(0x404040),
            text_dim: rgb(0x666666),
            text_disabled: rgb(0xa3a3a3),
            icon: rgb(0x525252),
            track: rgb(0xd4d4d4),
            thumb: rgb(0x525252),
            thumb_hover: rgb(0x171717),
            accent: evora_accent(),
            star: rgb(0x404040),
            pick: rgb(0x262626),
            reject: rgb(0xdc2626),
            caution: rgb(0xd97706),
            ..base
        }
    }
}

impl Tokens {
    /// The tokens of `kind`. Metrics are shared; only colours change. Sous l'identité d'Evora (E2),
    /// tout thème sombre montre les jetons sombres d'Evora et tout thème clair ses jetons clairs,
    /// avec les mesures d'Evora ([`evora_tokens`]).
    pub fn for_kind(kind: ThemeKind) -> Tokens {
        if evora_palette() {
            return evora_tokens(kind.is_dark());
        }
        let rgb = |v: u32| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        let base = Tokens::default();
        match kind {
            ThemeKind::Charcoal => base,
            ThemeKind::Midnight => Tokens {
                chrome: rgb(0x1e1e1e),
                canvas: rgb(0x121212),
                grid_bg: rgb(0x080808),
                cell: rgb(0x121212),
                cell_selected: rgb(0x262626),
                skeleton: rgb(0x1d1d1d),
                divider: rgb(0x0e0e0e),
                inset: rgb(0x181818),
                field: rgb(0x161616),
                field_border: rgb(0x333333),
                button: rgb(0x191919),
                button_border: rgb(0x404040),
                hover: rgb(0x2e2e2e),
                pressed: rgb(0x343434),
                tool_active: rgb(0x343434),
                ..base
            },
            ThemeKind::Silver => Tokens {
                ring_active: rgb(0x141414),
                ring_inner: rgb(0xffffff),
                ring_selected: rgb(0x4a4a4a),
                chrome: rgb(0xe6e6e6),
                canvas: rgb(0xcfcfcf),
                grid_bg: rgb(0xd9d9d9),
                cell: rgb(0xcbcbcb),
                cell_selected: rgb(0xb4b4b4),
                skeleton: rgb(0xa6a6a6),
                divider: rgb(0xc4c4c4),
                inset: rgb(0xdcdcdc),
                field: rgb(0xf6f6f6),
                field_border: rgb(0xb8b8b8),
                button: rgb(0xf2f2f2),
                button_border: rgb(0xb0b0b0),
                hover: rgb(0xd2d2d2),
                pressed: rgb(0xc4c4c4),
                tool_active: rgb(0xcacaca),
                text: rgb(0x1c1c1c),
                text_label: rgb(0x383838),
                text_dim: rgb(0x6a6a6a),
                text_disabled: rgb(0xa2a2a2),
                icon: rgb(0x555555),
                track: rgb(0xa8a8a8),
                thumb: rgb(0x5e5e5e),
                thumb_hover: rgb(0x262626),
                accent: rgb(0x0a62d0),
                star: rgb(0x3a3a3a),
                pick: rgb(0x2a2a2a),
                reject: rgb(0xc83434),
                caution: rgb(0xb0761a),
                ..base
            },
            ThemeKind::Paper => Tokens {
                ring_active: rgb(0x141414),
                ring_inner: rgb(0xffffff),
                ring_selected: rgb(0x4a4a4a),
                chrome: rgb(0xfafafa),
                canvas: rgb(0xececec),
                grid_bg: rgb(0xf2f2f2),
                cell: rgb(0xe8e8e8),
                cell_selected: rgb(0xd2d2d2),
                skeleton: rgb(0xbcbcbc),
                divider: rgb(0xe0e0e0),
                inset: rgb(0xf0f0f0),
                field: rgb(0xffffff),
                field_border: rgb(0xcccccc),
                button: rgb(0xffffff),
                button_border: rgb(0xc6c6c6),
                hover: rgb(0xebebeb),
                pressed: rgb(0xdedede),
                tool_active: rgb(0xe2e2e2),
                text: rgb(0x161616),
                text_label: rgb(0x343434),
                text_dim: rgb(0x707070),
                text_disabled: rgb(0xababab),
                icon: rgb(0x585858),
                track: rgb(0xbdbdbd),
                thumb: rgb(0x606060),
                thumb_hover: rgb(0x202020),
                accent: rgb(0x0a62d0),
                star: rgb(0x3a3a3a),
                pick: rgb(0x2a2a2a),
                reject: rgb(0xc83434),
                caution: rgb(0xb0761a),
                ..base
            },
        }
    }

    pub fn get(ctx: &egui::Context) -> Tokens {
        ctx.data(|d| d.get_temp::<Tokens>(egui::Id::NULL)).unwrap_or_default()
    }
    pub fn font(&self, size: f32) -> FontId {
        FontId::proportional(size)
    }
    pub fn semibold(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(FONT_SEMIBOLD.into()))
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    install_fonts_with_chinese(ctx, None);
}

/// Install the browser's separately downloaded Chinese face alongside the embedded faces.
pub fn install_fonts_with_chinese(ctx: &egui::Context, chinese: Option<&Arc<FontData>>) {
    ctx.set_fonts(font_definitions_with_chinese(evolight_engine::EVO_FONTS, chinese));
}

/// Inter remains the default UI face. Egui's default faces and evo-fonts cover other scripts;
/// the browser's separate Chinese face precedes Japanese faces within the CJK fallback list.
/// Without evo-fonts or that separate face, CJK text shows boxes.
pub fn font_definitions(evo: &'static [evolight_engine::EvoFont]) -> FontDefinitions {
    font_definitions_with_chinese(evo, None)
}

/// egui's emoji face (monochrome Noto Emoji), one of its bundled defaults.
pub const EMOJI_FACE: &str = "NotoEmoji-Regular";

/// egui's bundled faces without its emoji face. Evora (E10, "zero emoji" directive): no UI family
/// falls back on an emoji face, and the face is not even loaded; the symbols the UI uses (★ ⌘ ⇧ →…)
/// come from text faces, which the web build's render audit (`web.renderAudit`) checks.
pub fn base_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.remove(EMOJI_FACE);
    for names in fonts.families.values_mut() {
        names.retain(|name| name != EMOJI_FACE);
    }
    fonts
}

/// The web build keeps the large Simplified Chinese face outside the size-limited WASM module.
/// Prefer it for Chinese UI text, including Traditional Chinese until it has its own face.
pub fn font_definitions_with_chinese(evo: &'static [evolight_engine::EvoFont], chinese: Option<&Arc<FontData>>) -> FontDefinitions {
    let mut fonts = base_definitions();
    fonts.font_data.insert("Inter".into(), Arc::new(FontData::from_static(include_bytes!("../../../assets/fonts/Inter-Regular.ttf"))));
    fonts.font_data.insert("Inter-SemiBold".into(), Arc::new(FontData::from_static(include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf"))));
    // Evo-fonts faces in preference order for a family drawn in `style`.
    let fallback = |style: &str| {
        evolight_engine::fonts::ui_fallback(evo, crate::i18n::language().script(), style)
            .into_iter()
            .map(evo_font_name)
            .collect::<Vec<String>>()
    };
    let (mut regular, mut bold) = (fallback("Regular"), fallback("Bold"));
    for name in regular.iter().chain(bold.iter()) {
        if let Some(font) = evo.iter().find(|font| &evo_font_name(font) == name) {
            fonts.font_data.insert(name.clone(), Arc::new(FontData::from_static(font.bytes)));
        }
    }
    // A variable face holds every weight in its one file: strong text draws it heavier, from the
    // same bytes. A face with one fixed weight stays as it is.
    for name in &mut bold {
        let Some(font) = evo.iter().find(|font| &evo_font_name(font) == name) else { continue };
        let Some(heavier) = strong_weight(font.bytes) else { continue };
        *name = format!("{name} strong");
        fonts.font_data.insert(name.clone(), Arc::new(heavier));
    }
    let script = crate::i18n::language().script();
    let prefer_chinese = chinese.is_some() && matches!(script, "Hans" | "Hant");
    if let Some(font) = chinese {
        fonts.font_data.insert(WEB_CHINESE_FONT.into(), font.clone());
        if prefer_chinese {
            regular.insert(0, WEB_CHINESE_FONT.into());
            bold.insert(0, WEB_CHINESE_FONT.into());
        } else {
            regular.push(WEB_CHINESE_FONT.into());
            bold.push(WEB_CHINESE_FONT.into());
        }
    }
    let defaults: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut prop = vec!["Inter".to_string()];
    prop.extend(defaults.iter().cloned());
    prop.extend(regular);
    fonts.families.insert(FontFamily::Proportional, prop);
    let mut semi = vec!["Inter-SemiBold".to_string()];
    semi.extend(defaults);
    semi.extend(bold);
    fonts.families.insert(FontFamily::Name(FONT_SEMIBOLD.into()), semi);
    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    let mut mono_fallback = fallback("Regular");
    if prefer_chinese {
        mono_fallback.insert(0, WEB_CHINESE_FONT.into());
    }
    if chinese.is_some() && !prefer_chinese {
        mono_fallback.push(WEB_CHINESE_FONT.into());
    }
    mono.extend(mono_fallback);
    fonts
}

/// The font families used by this app instance, including the browser's separately loaded face.
pub fn font_credits(chinese_loaded: bool) -> String {
    let mut families = vec!["Inter"];
    for f in evolight_engine::EVO_FONTS {
        if !families.contains(&f.family) {
            families.push(f.family);
        }
    }
    if chinese_loaded && !families.contains(&"Noto Sans CJK SC") {
        families.push("Noto Sans CJK SC");
    }
    families.join(" / ")
}

/// The weight of strong text in a variable face, to go with Inter SemiBold.
const STRONG_WEIGHT: f32 = 600.0;

/// `bytes` as a face drawn at [`STRONG_WEIGHT`], when it is a variable font whose weight axis goes
/// heavier than the weight it draws at by default. The bytes are borrowed, not copied.
fn strong_weight(bytes: &'static [u8]) -> Option<FontData> {
    let face = FontData::from_static(bytes);
    let axis = face.variation_axes().into_iter().find(|axis| axis.tag.to_be_bytes() == *b"wght")?;
    let weight = STRONG_WEIGHT.min(axis.range.max);
    if weight.partial_cmp(&axis.default) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let tweak = egui::FontTweak { coords: egui::epaint::text::VariationCoords::new([(axis.tag, weight)]), ..Default::default() };
    Some(face.tweak(tweak))
}

fn evo_font_name(f: &evolight_engine::EvoFont) -> String {
    format!("evo-fonts {} {}", f.family, f.style)
}

/// Points one mouse-wheel notch (a line) scrolls.
pub const WHEEL_LINE_POINTS: f32 = 100.0;

/// Share of its visible size one mouse-wheel notch scrolls the photo grid and the filmstrip (Lightroom
/// Classic: three notches move a photo from one edge to the other, at any window size).
pub const WHEEL_STRIP_SHARE: f32 = 1.0 / 3.0;

/// Zoom per wheel point (`exp(speed × points)`): egui's 1/200 at its stock 40-point notch, rescaled so
/// one notch still zooms the same amount now that it scrolls [`WHEEL_LINE_POINTS`].
pub const WHEEL_ZOOM_PER_POINT: f32 = 40.0 / 200.0 / WHEEL_LINE_POINTS;

fn theme_id() -> egui::Id {
    egui::Id::unique("evolight-theme")
}

/// The theme `ctx` is styled with (none before the first [`apply_kind`]).
pub fn current(ctx: &egui::Context) -> Option<ThemeKind> {
    ctx.data(|d| d.get_temp::<ThemeKind>(theme_id()))
}

/// Windows only: tell the window its theme again, the first time it has focus. A first theme that
/// reaches a focused window as it comes up changes the title's colour but not the caption's, so
/// with EvoLight dark on a light system the title is white on a light caption (it looks blank),
/// and light on dark is dark on dark, until focus next moves (#3381). The same command repaints
/// the caption once the window is up. A window that opens without focus shows the right colours
/// from the start.
pub fn refresh_window_on_first_focus(ctx: &egui::Context) {
    let done = egui::Id::unique("evolight-window-theme-refreshed");
    if !cfg!(target_os = "windows") || !ctx.input(|i| i.focused) || ctx.data(|d| d.get_temp::<bool>(done)).is_some() {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
    let theme = match ctx.theme() {
        egui::Theme::Dark => egui::SystemTheme::Dark,
        egui::Theme::Light => egui::SystemTheme::Light,
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(theme));
}

/// Style `ctx` with the default theme.
pub fn apply(ctx: &egui::Context) {
    apply_kind(ctx, ThemeKind::default());
}

/// Style `ctx` with `kind`. The visuals go to both of egui's styles, so egui following the
/// system's dark/light setting on its own never swaps in its stock look.
pub fn apply_kind(ctx: &egui::Context, kind: ThemeKind) {
    // Keep photo zoom shortcuts from resizing egui itself.
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    // A mouse wheel notch scrolls 100 points (egui's default 40 took 50+ notches through an open
    // Edit panel; Lightroom moves about this much per notch). Trackpads report points and are
    // unaffected.
    // A modifier-wheel (and the loupe's plain-wheel) zoom notch stays about 1.22x, as it was at 40 points.
    ctx.options_mut(|o| {
        o.input_options.line_scroll_speed = WHEEL_LINE_POINTS;
        o.input_options.scroll_zoom_speed = WHEEL_ZOOM_PER_POINT;
    });
    let t = Tokens::for_kind(kind);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::NULL, t);
        d.insert_temp(theme_id(), kind);
    });
    let dark = kind.is_dark();
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.chrome;
    v.window_fill = t.chrome;
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.inset;
    v.window_stroke = Stroke::new(1.0, t.button_border);
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(6);
    v.selection.bg_fill = t.accent.gamma_multiply(0.6);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.override_text_color = Some(t.text_label);
    let (popup, window) = if dark { (140, 160) } else { (50, 60) };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(popup) };
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 30, spread: 0, color: Color32::from_black_alpha(window) };
    let w = &mut v.widgets;
    evo_ui::theme::WidgetPalette { noninteractive: t.chrome, inactive: t.button, hovered: t.hover, active: t.pressed, open: t.hover }.apply_to(
        w,
        CornerRadius::same(4),
        Stroke::new(1.0, t.text_label),
    );
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    // Evora (EVORA-REV-104, E2) : sous l'identité d'Evora, le bord au repos des contrôles natifs
    // d'egui (cases à cocher, boutons radio, champs de texte, listes déroulantes, boutons natifs)
    // est celui des champs, 3:1 ou plus sur le chrome et sur le champ. `button_border` (#333333 en
    // sombre) n'y tenait que 1,35:1 et laissait une case non cochée presque invisible. Une liste
    // déroulante ouverte garde ce bord (sinon celui par défaut, 1,54:1). Hors d'Evora, le bord de
    // l'application, et l'état ouvert inchangé. Evora (EVORA-REV-105, E2) : ce choix est
    // `control_border`, partagé avec les boutons propres de l'application.
    let resting_border = control_border(&t);
    w.inactive.bg_stroke = Stroke::new(1.0, resting_border);
    if evora_palette() {
        w.open.bg_stroke = Stroke::new(1.0, t.field_border);
    }
    ctx.set_visuals_of(egui::Theme::Dark, v.clone());
    ctx.set_visuals_of(egui::Theme::Light, v);
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    // Evora (EVORA-REV-100, E2) : texte plus dense sous l'identité d'Evora (corps et boutons 12,5,
    // petit 11, en-tête 14 en demi-gras) ; les rayons (4 pour les contrôles, 6 pour les fenêtres
    // et les menus), les ombres (celles des seuls calques flottants) et l'espacement restent ceux
    // ci-dessus. Hors d'Evora, les tailles de l'application.
    let (body, small, heading): (f32, f32, f32) = if evora_palette() { (12.5, 11.0, 14.0) } else { (13.0, 11.0, 16.0) };
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(body));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(body));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(small));
        s.text_styles.insert(egui::TextStyle::Heading, FontId::new(heading, FontFamily::Name(FONT_SEMIBOLD.into())));
        s.animation_time = 0.08;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::button::contrast;
    use crate::i18n::{Locale, set_language};

    /// The selection ring is a mark on the grid: 3:1 or more against what it is drawn over, in
    /// every theme. The active photo's ring reads on the grid and on a selected or unselected
    /// square-grid cell; the other selected photos' ring on the grid.
    #[test]
    fn selection_rings_stand_out_from_the_grid_in_every_theme() {
        for kind in ThemeKind::ALL {
            let t = Tokens::for_kind(kind);
            for (what, surface) in [("grid", t.grid_bg), ("cell", t.cell), ("selected cell", t.cell_selected)] {
                assert!(contrast(t.ring_active, surface) >= 3.0, "{kind:?}: active ring on the {what}: {:.2}", contrast(t.ring_active, surface));
            }
            // the selected ring also sits on a selected cell (square grid, filmstrip), label-tinted or not
            let mut selected_cells = vec![("grid", t.grid_bg), ("selected cell", t.cell_selected)];
            for label in evolight_catalog::ColorLabel::ALL {
                selected_cells.push(("labelled selected cell", label_background(t.cell_selected, Some(label), true)));
            }
            for (what, surface) in selected_cells {
                assert!(
                    contrast(t.ring_selected, surface) >= 3.0,
                    "{kind:?}: selected ring on the {what}: {:.2}",
                    contrast(t.ring_selected, surface)
                );
            }
            // the thin inner line of the active ring is the ring's opposite: it parts the ring from the photo's edge
            assert!(contrast(t.ring_inner, t.ring_active) >= 3.0, "{kind:?}: inner line against the active ring");
        }
    }

    /// Evora (EVORA-REV-100, E2) : sous l'identité d'Evora, en sombre graphite comme en clair, les
    /// gris sont neutres (R = G = B), les anneaux tiennent 3:1 sur ce qu'ils bordent (cellules
    /// teintées d'un libellé comprises), les textes 4,5:1 sur les surfaces qui les portent, le bord
    /// des champs 3:1, et l'accent reste celui que l'hôte a envoyé. Les tailles de texte d'Evora ne
    /// valent que sous son identité.
    #[test]
    fn evora_tokens_are_neutral_greys_with_their_contrasts_and_the_hosts_accent() {
        let host = Color32::from_rgb(0x0e, 0x7c, 0x66);
        set_evora_accent(host);
        set_evora_palette(true);
        for kind in ThemeKind::ALL {
            let t = Tokens::for_kind(kind);
            assert_eq!(t.accent, host, "{kind:?} : l'accent de l'hôte, tel quel");
            let greys = [
                ("chrome", t.chrome),
                ("canvas", t.canvas),
                ("grid_bg", t.grid_bg),
                ("cell", t.cell),
                ("cell_selected", t.cell_selected),
                ("skeleton", t.skeleton),
                ("ring_active", t.ring_active),
                ("ring_inner", t.ring_inner),
                ("ring_selected", t.ring_selected),
                ("divider", t.divider),
                ("inset", t.inset),
                ("field", t.field),
                ("field_border", t.field_border),
                ("button", t.button),
                ("button_border", t.button_border),
                ("hover", t.hover),
                ("pressed", t.pressed),
                ("tool_active", t.tool_active),
                ("text", t.text),
                ("text_label", t.text_label),
                ("text_dim", t.text_dim),
                ("text_disabled", t.text_disabled),
                ("icon", t.icon),
                ("track", t.track),
                ("thumb", t.thumb),
                ("thumb_hover", t.thumb_hover),
                ("star", t.star),
                ("pick", t.pick),
            ];
            for (what, c) in greys {
                assert!(c.r() == c.g() && c.g() == c.b(), "{kind:?} : {what} n'est pas un gris neutre ({c:?})");
            }
            for (what, surface) in [("grid", t.grid_bg), ("cell", t.cell), ("selected cell", t.cell_selected)] {
                let c = contrast(t.ring_active, surface);
                assert!(c >= 3.0, "{kind:?} : anneau actif sur {what} : {c:.2}");
            }
            let mut selected_cells = vec![t.grid_bg, t.cell_selected];
            for label in evolight_catalog::ColorLabel::ALL {
                selected_cells.push(label_background(t.cell_selected, Some(label), true));
            }
            for surface in selected_cells {
                let c = contrast(t.ring_selected, surface);
                assert!(c >= 3.0, "{kind:?} : anneau des photos choisies sur {surface:?} : {c:.2}");
            }
            assert!(contrast(t.ring_inner, t.ring_active) >= 3.0, "{kind:?} : liseré intérieur contre l'anneau actif");
            for (what, surface) in [("chrome", t.chrome), ("canvas", t.canvas), ("inset", t.inset), ("field", t.field), ("hover", t.hover)] {
                for (role, text) in [("text", t.text), ("text_label", t.text_label), ("text_dim", t.text_dim)] {
                    let c = contrast(text, surface);
                    assert!(c >= 4.5, "{kind:?} : {role} sur {what} : {c:.2}");
                }
            }
            for (what, surface) in [("field", t.field), ("chrome", t.chrome)] {
                let c = contrast(t.field_border, surface);
                assert!(c >= 3.0, "{kind:?} : bord de champ sur {what} : {c:.2}");
            }
            // le bord des boutons propres de l'application (EVORA-REV-105) : celui des champs, 3:1
            // ou plus sur les surfaces où ces boutons sont posés, et celui que prend un bouton
            // secondaire au repos
            assert_eq!(control_border(&t), t.field_border, "{kind:?} : bord des boutons propres");
            for (what, surface) in [("chrome", t.chrome), ("canvas", t.canvas), ("inset", t.inset), ("field", t.field)] {
                let c = contrast(control_border(&t), surface);
                assert!(c >= 3.0, "{kind:?} : bord des boutons propres sur {what} : {c:.2}");
            }
            let secondary = crate::button::look(&t, crate::button::Kind::Secondary, crate::button::State::Idle);
            assert_eq!(secondary.border, Some(control_border(&t)), "{kind:?} : bord d'un bouton secondaire");
            // le nom et le format sous une vignette (EVORA-REV-104) : 4,5:1 ou plus sur toute
            // cellule de la bande et de la grille, choisie ou non, survolée, teintée ou non, telle
            // qu'elle est composée sur le canvas de la bande (la survolée est un voile translucide)
            let hovered = t.cell_selected.gamma_multiply(CELL_HOVER_FILL);
            for (base, selected) in [(t.canvas, false), (hovered, false), (t.cell, false), (t.cell_selected, true)] {
                for label in std::iter::once(None).chain(evolight_catalog::ColorLabel::ALL.iter().copied().map(Some)) {
                    let surface = t.canvas.blend(label_background(base, label, selected));
                    let c = contrast(secondary_text_on(&t, surface), surface);
                    assert!(c >= MIN_TEXT_CONTRAST, "{kind:?} : nom sous une vignette sur {surface:?} : {c:.2}");
                }
            }
            assert_eq!(secondary_text_on(&t, t.chrome), t.text_dim, "{kind:?} : le gris secondaire là où il se lit");
        }
        let sizes = |ctx: &egui::Context| {
            let style = ctx.global_style();
            [egui::TextStyle::Body, egui::TextStyle::Small, egui::TextStyle::Heading].map(|s| style.text_styles.get(&s).map(|f| f.size))
        };
        let ctx = egui::Context::default();
        apply_kind(&ctx, ThemeKind::Charcoal);
        assert_eq!(sizes(&ctx), [Some(12.5), Some(11.0), Some(14.0)], "tailles d'Evora");
        // le bord au repos des contrôles natifs (cases à cocher, champs, listes) : 3:1 ou plus sur
        // le chrome et sur le champ, en sombre comme en clair
        for kind in ThemeKind::ALL {
            let ctx = egui::Context::default();
            apply_kind(&ctx, kind);
            let t = Tokens::for_kind(kind);
            let widgets = ctx.global_style().visuals.widgets.clone();
            for (state, border) in [("au repos", widgets.inactive.bg_stroke.color), ("ouvert", widgets.open.bg_stroke.color)] {
                for (what, surface) in [("chrome", t.chrome), ("field", t.field)] {
                    let c = contrast(border, surface);
                    assert!(c >= 3.0, "{kind:?} : bord {state} des contrôles natifs sur {what} : {c:.2}");
                }
            }
        }
        set_evora_palette(false);
        set_evora_accent(EVORA_ACCENT);
        let ctx = egui::Context::default();
        apply_kind(&ctx, ThemeKind::Charcoal);
        assert_eq!(sizes(&ctx), [Some(13.0), Some(11.0), Some(16.0)], "hors d'Evora, les tailles de l'application");
        let t = Tokens::for_kind(ThemeKind::Charcoal);
        assert_eq!(secondary_text_on(&t, t.cell_selected), t.text_dim, "hors d'Evora, le gris secondaire de l'application");
        assert_eq!(control_border(&t), t.button_border, "hors d'Evora, le bord des boutons de l'application");
    }

    /// Le masque d'un hôte qui mène l'apparence, comme celui du pont d'Evora.
    fn masks_appearance(id: &str) -> bool {
        id == "view.appearance"
    }

    /// Evora (EVORA-REV-106, E2) : sous l'identité d'Evora, en clair comme en sombre, les dessins à
    /// fond fixe restent lisibles. Les encres posées sur un voile ou une pastille sombres sont
    /// celles du sombre d'Evora (badge de format et toast à 4,5:1 ou plus, marques d'une vignette à
    /// 3:1 ou plus sur une photo gris moyen voilée) ; les tracés prennent un jeton du thème ; les
    /// filets d'un graphe se détachent de son fond sans le charger. Hors d'Evora, les couleurs de
    /// l'amont. Evora (EVORA-REV-107) : la réserve du coin de la pastille de l'hôte, que ce test
    /// jugeait aussi, est retirée ; reste le masque qui dit que l'hôte mène l'apparence.
    #[test]
    fn evora_fixed_ground_drawings_read_in_both_appearances() {
        set_evora_palette(true);
        let dark = Tokens::for_kind(ThemeKind::Charcoal);
        for kind in ThemeKind::ALL {
            let t = Tokens::for_kind(kind);
            let ink = on_dark_veil(&t);
            for (what, got, want) in [
                ("star", ink.star, dark.star),
                ("pick", ink.pick, dark.pick),
                ("reject", ink.reject, dark.reject),
                ("text_label", ink.text_label, dark.text_label),
                ("text", ink.text, dark.text),
            ] {
                assert_eq!(got, want, "{kind:?} : {what} sur un voile sombre");
            }
            // le badge de format de la grille, sur sa pastille fixe (gris 26)
            let c = contrast(ink.text_label, Color32::from_gray(26));
            assert!(c >= MIN_TEXT_CONTRAST, "{kind:?} : badge de format : {c:.2}");
            // le toast, sur son voile noir posé sur l'entourage de l'image ou sur la grille
            for (what, surface) in [("canvas", t.canvas), ("grid_bg", t.grid_bg)] {
                let c = contrast(ink.text, surface.blend(Color32::from_black_alpha(200)));
                assert!(c >= MIN_TEXT_CONTRAST, "{kind:?} : toast sur {what} : {c:.2}");
            }
            // les marques d'une vignette sur leur voile (profil 110, grille 120, bande 130), la photo
            // étant d'un gris moyen
            for alpha in [110, 120, 130] {
                let veiled = Color32::from_gray(128).blend(Color32::from_black_alpha(alpha));
                for (what, mark) in [("star", ink.star), ("pick", ink.pick), ("reject", ink.reject), ("text_label", ink.text_label)] {
                    let c = contrast(mark, veiled);
                    assert!(c >= 3.0, "{kind:?} : {what} sur un voile de {alpha} : {c:.2}");
                }
            }
            // les tracés : la courbe sur le canvas, la ligne neutre et les repères de l'histogramme
            // sur son fond (`button`)
            assert_eq!(evora_ink(t.text, Color32::from_gray(220)), t.text, "{kind:?} : un tracé prend le jeton");
            for (what, line, surface, floor) in [
                ("courbe sur le canvas", t.text, t.canvas, MIN_TEXT_CONTRAST),
                ("ligne neutre de l'histogramme", t.text_label, t.button, MIN_TEXT_CONTRAST),
                ("repères de l'histogramme", t.text_dim, t.button, 3.0),
            ] {
                let c = contrast(line, surface);
                assert!(c >= floor, "{kind:?} : {what} : {c:.2}");
            }
            // les filets du graphe : visibles sans charger le fond, la diagonale plus que le quadrillage
            let grid = contrast(graph_rule(&t, t.canvas, GRAPH_GRID_SHARE, Color32::from_gray(52)), t.canvas);
            let axis = contrast(graph_rule(&t, t.canvas, GRAPH_AXIS_SHARE, Color32::from_gray(70)), t.canvas);
            assert!(grid >= 1.2 && axis > grid && axis < 3.0, "{kind:?} : quadrillage {grid:.2}, diagonale {axis:.2}");
        }
        // l'hôte mène l'apparence quand son masque en retire la commande et que l'identité d'Evora
        // est montrée (EVORA-REV-108) : l'un sans l'autre laisse l'apparence à l'application
        assert!(!host_leads_appearance(), "sans masque d'hôte, l'application mène son apparence");
        crate::menubar::set_host_mask(Some(masks_appearance as fn(&str) -> bool));
        assert!(host_leads_appearance(), "le masque de l'hôte retire la commande d'apparence");
        set_evora_palette(false);
        assert!(!host_leads_appearance(), "sous un masque mais hors de l'identité d'Evora, l'application garde son apparence");
        crate::menubar::set_host_mask(None);
        assert!(!host_leads_appearance());
        // hors d'Evora : les jetons du thème et les couleurs de l'amont
        for kind in ThemeKind::ALL {
            let t = Tokens::for_kind(kind);
            let ink = on_dark_veil(&t);
            assert_eq!(
                (ink.star, ink.pick, ink.reject, ink.text_label, ink.text),
                (t.star, t.pick, t.reject, t.text_label, t.text),
                "{kind:?} : hors d'Evora, les encres du thème"
            );
            assert_eq!(evora_ink(t.text, Color32::from_gray(220)), Color32::from_gray(220), "{kind:?} : hors d'Evora, le tracé de l'amont");
            assert_eq!(graph_rule(&t, t.canvas, GRAPH_GRID_SHARE, Color32::from_gray(52)), Color32::from_gray(52), "{kind:?} : le filet de l'amont");
        }
    }

    /// The `SetTheme(Light)` commands one frame of `ctx` sends the window, with or without
    /// [`refresh_window_on_first_focus`] in it.
    #[cfg(target_os = "windows")]
    fn light_theme_commands(ctx: &egui::Context, focused: bool, refresh: bool) -> usize {
        let raw = egui::RawInput { focused, ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| {
            if refresh {
                refresh_window_on_first_focus(ui.ctx());
            }
        });
        out.textures_delta.clear();
        let commands = out.viewport_output.remove(&egui::ViewportId::ROOT).map(|v| v.commands).unwrap_or_default();
        commands.iter().filter(|c| matches!(c, egui::ViewportCommand::SetTheme(egui::SystemTheme::Light))).count()
    }

    /// #3381: egui sends the theme with the first frame, and the window hears it once more the
    /// first time it has focus, not on every later focus.
    #[cfg(target_os = "windows")]
    #[test]
    fn the_window_hears_its_theme_again_the_first_time_it_has_focus() {
        let ctx = egui::Context::default();
        apply_kind(&ctx, ThemeKind::Silver);
        let sent = |focused: bool| light_theme_commands(&ctx, focused, true);
        assert_eq!(sent(false), 1, "egui's own, with the first frame");
        assert_eq!(sent(false), 0, "no focus yet");
        assert_eq!(sent(true), 1, "the first frame with focus");
        assert_eq!(sent(true), 0, "still focused");
        assert_eq!(sent(false), 0, "focus lost");
        assert_eq!(sent(true), 0, "focus back: Windows repaints this by itself");
    }

    /// #3381: a window that has focus from its very first frame hears the theme once more in that
    /// frame, on top of egui's own command. (In the app the first frame never gets here: `ui`
    /// returns before the fonts are ready, so egui's command always goes a frame earlier.)
    #[cfg(target_os = "windows")]
    #[test]
    fn a_window_focused_from_its_first_frame_hears_its_theme_again_in_that_frame() {
        let first_frame = |refresh: bool| {
            let ctx = egui::Context::default();
            apply_kind(&ctx, ThemeKind::Silver);
            light_theme_commands(&ctx, true, refresh)
        };
        assert_eq!(first_frame(true), first_frame(false) + 1);
    }

    /// Every theme scrolls 100 points a wheel notch, and a zoom notch stays egui's stock ~1.22x
    /// (`exp(40 / 200)`) instead of growing with the longer scroll.
    #[test]
    fn a_wheel_notch_scrolls_further_but_zooms_as_before_in_every_theme() {
        for kind in [ThemeKind::Charcoal, ThemeKind::Midnight, ThemeKind::Silver, ThemeKind::Paper] {
            let ctx = egui::Context::default();
            apply_kind(&ctx, kind);
            let (line, zoom) = ctx.options(|o| (o.input_options.line_scroll_speed, o.input_options.scroll_zoom_speed));
            assert_eq!(line, WHEEL_LINE_POINTS, "{kind:?}");
            assert!(((zoom * line).exp() - (40.0_f32 / 200.0).exp()).abs() < 1e-4, "{kind:?}: a notch zooms {}", (zoom * line).exp());
        }
    }

    /// #736: the release input must carry Thai, not silently skip the check with an older pin.
    /// Exercise the Thai characters of the reported folder name through every egui family.
    #[test]
    fn thai_folder_name_has_glyphs_in_every_ui_family() {
        if evolight_engine::EVO_FONTS.is_empty() {
            evolight_testkit::skip!(Fonts, "built without EVO_FONTS_DIR: no evo-fonts to check");
            return;
        }
        let thai = evolight_engine::EVO_FONTS.iter().find(|f| f.covers("Thai")).expect("font input includes Thai");
        set_language(Locale::En);
        let definitions = font_definitions(evolight_engine::EVO_FONTS);
        let name = evo_font_name(thai);
        assert!(definitions.font_data.contains_key(&name));
        // (The strong family has the face's heavier twin instead when it is a variable font.)
        let strong = format!("{name} strong");
        for family in [FontFamily::Proportional, FontFamily::Name(FONT_SEMIBOLD.into()), FontFamily::Monospace] {
            let names = definitions.families.get(&family).expect("UI font family");
            assert!(names.contains(&name) || (family == semibold_family() && names.contains(&strong)), "{family:?} falls back on {name}");
        }
        let ctx = egui::Context::default();
        ctx.set_fonts(definitions);
        let mut frame = ctx.run_ui(egui::RawInput::default(), |_| {});
        frame.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            for family in [FontFamily::Proportional, FontFamily::Name(FONT_SEMIBOLD.into()), FontFamily::Monospace] {
                let id = FontId::new(13.0, family);
                // Check the Thai portion; egui 0.36's has_glyph reports false for characters
                // in the replacement glyph's own face (including normal monospace Latin).
                for ch in "Studio34 ถ่ายภาพงานแต่งงาน".chars().filter(|ch| !ch.is_ascii()) {
                    assert!(fonts.has_glyph(&id, ch), "{id:?} has {ch}");
                }
            }
        });
    }

    /// Evora (E10): no UI family falls back on egui's emoji face, and the face is not loaded.
    #[test]
    fn no_ui_family_uses_the_emoji_face() {
        let definitions = font_definitions(evolight_engine::EVO_FONTS);
        assert!(!definitions.font_data.contains_key(EMOJI_FACE));
        for (family, names) in &definitions.families {
            assert!(!names.iter().any(|name| name == EMOJI_FACE), "{family:?}");
        }
    }

    #[test]
    fn native_chinese_face_falls_back_after_default_faces() {
        static FACES: &[evolight_engine::EvoFont] = &[
            evolight_engine::EvoFont {
                family: "BIZ UDPGothic",
                style: "Regular",
                scripts: &["Jpan", "Latn"],
                bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
            },
            evolight_engine::EvoFont {
                family: "Noto Sans CJK SC",
                style: "Regular",
                scripts: &["Hans", "Latn"],
                bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
            },
        ];
        set_language(Locale::ZhHans);
        let definitions = font_definitions(FACES);
        let defaults = base_definitions();
        let default_prop = defaults.families.get(&FontFamily::Proportional).expect("default proportional fonts");
        let chinese = "evo-fonts Noto Sans CJK SC Regular";
        for (family, primary) in [(FontFamily::Proportional, "Inter"), (FontFamily::Name(FONT_SEMIBOLD.into()), "Inter-SemiBold")] {
            let names = definitions.families.get(&family).expect("UI font family");
            assert_eq!(names.first().map(String::as_str), Some(primary));
            assert_eq!(names.get(1), default_prop.first());
            assert!(names.iter().position(|name| name == chinese).is_some_and(|position| position > default_prop.len()));
        }
        let mono = definitions.families.get(&FontFamily::Monospace).expect("monospace font family");
        let default_mono = defaults.families.get(&FontFamily::Monospace).expect("default monospace fonts");
        assert_eq!(mono.first(), default_mono.first());
        assert!(mono.iter().position(|name| name == chinese).is_some_and(|position| position >= default_mono.len()));
        set_language(Locale::En);
    }

    #[test]
    fn separate_web_chinese_face_is_available_in_every_ui_family() {
        // Simulate the WASM build, which embeds only the Japanese face. The Chinese face arrives
        // as a separate file and must precede that face when the UI uses Chinese.
        static JAPANESE: &[evolight_engine::EvoFont] = &[evolight_engine::EvoFont {
            family: "BIZ UDPGothic",
            style: "Regular",
            scripts: &["Jpan", "Latn"],
            bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
        }];
        let embedded = evolight_engine::EVO_FONTS.iter().find(|font| font.family == "Noto Sans CJK SC" && font.style == "Regular");
        let fallback: &'static [u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        let source = embedded.map_or(fallback, |font| font.bytes);
        let chinese = Arc::new(FontData::from_static(source));
        set_language(Locale::ZhHans);
        let definitions = font_definitions_with_chinese(JAPANESE, Some(&chinese));
        let default_count = base_definitions().families.get(&FontFamily::Proportional).map_or(0, Vec::len);
        for (family, primary) in [(FontFamily::Proportional, "Inter"), (FontFamily::Name(FONT_SEMIBOLD.into()), "Inter-SemiBold")] {
            let names = definitions.families.get(&family).expect("UI font family");
            assert_eq!(names.first().map(String::as_str), Some(primary));
            assert_eq!(names.get(1 + default_count).map(String::as_str), Some(WEB_CHINESE_FONT));
        }
        assert!(definitions.families.get(&FontFamily::Monospace).is_some_and(|names| names.contains(&WEB_CHINESE_FONT.to_string())));

        if embedded.is_some() {
            let ctx = egui::Context::default();
            ctx.set_fonts(definitions);
            let mut frame = ctx.run_ui(egui::RawInput::default(), |_| {});
            frame.textures_delta.clear();
            ctx.fonts_mut(|fonts| {
                for family in [FontFamily::Proportional, FontFamily::Name(FONT_SEMIBOLD.into()), FontFamily::Monospace] {
                    let id = FontId::new(13.0, family);
                    assert!("简体中文字".chars().all(|ch| fonts.has_glyph(&id, ch)), "{id:?}");
                }
            });
        }

        set_language(Locale::Ja);
        let japanese = font_definitions_with_chinese(JAPANESE, Some(&chinese));
        let names = japanese.families.get(&FontFamily::Proportional).expect("proportional fonts");
        assert_eq!(names.get(1 + default_count).map(String::as_str), Some("evo-fonts BIZ UDPGothic Regular"));
        assert_eq!(names.get(2 + default_count).map(String::as_str), Some(WEB_CHINESE_FONT));
        set_language(Locale::En);
    }

    const ARABIC_WORD: &str = "الصور";

    fn semibold_family() -> FontFamily {
        FontFamily::Name(FONT_SEMIBOLD.into())
    }

    /// The width of `text` in `family` at the body size, and the ink its glyphs put in the font
    /// atlas (summed coverage), with `definitions` installed.
    fn width_and_ink(definitions: FontDefinitions, text: &str, family: FontFamily) -> (f32, u64) {
        let ctx = egui::Context::default();
        ctx.set_fonts(definitions);
        let mut frame = ctx.run_ui(egui::RawInput::default(), |_| {});
        frame.textures_delta.clear();
        ctx.fonts_mut(|fonts| {
            let galley = fonts.layout_no_wrap(text.into(), FontId::new(13.0, family), Color32::WHITE);
            let atlas = fonts.image();
            let mut ink = 0_u64;
            for glyph in galley.rows.iter().flat_map(|row| row.glyphs.iter()) {
                let (min, max) = (glyph.uv_rect.min, glyph.uv_rect.max);
                for y in min[1]..max[1] {
                    for x in min[0]..max[0] {
                        ink += u64::from(atlas[(usize::from(x), usize::from(y))].a());
                    }
                }
            }
            (galley.size().x, ink)
        })
    }

    /// The evo-fonts face for Arabic, when it is a variable font with a weight axis.
    fn variable_arabic_face() -> Option<&'static evolight_engine::EvoFont> {
        let face = evolight_engine::EVO_FONTS.iter().find(|font| font.covers("Arab"))?;
        FontData::from_static(face.bytes).variation_axes().iter().any(|axis| axis.tag.to_be_bytes() == *b"wght").then_some(face)
    }

    /// Strong text in a script whose fallback face is a variable font is really heavier: an Arabic
    /// tab name used to be drawn at regular weight beside a bold Latin one.
    #[test]
    fn strong_arabic_text_is_heavier_than_regular_with_a_variable_face() {
        // Given a build with evo-fonts whose Arabic face has a weight axis
        if variable_arabic_face().is_none() {
            eprintln!("skipped: built without a variable-weight evo-fonts face for Arabic");
            return;
        }
        set_language(Locale::En);
        // When an Arabic word is laid out regular and strong at the same size
        let (regular_width, regular_ink) = width_and_ink(font_definitions(evolight_engine::EVO_FONTS), ARABIC_WORD, FontFamily::Proportional);
        let (strong_width, strong_ink) = width_and_ink(font_definitions(evolight_engine::EVO_FONTS), ARABIC_WORD, semibold_family());
        eprintln!("{ARABIC_WORD}: regular {regular_width} px, ink {regular_ink}; strong {strong_width} px, ink {strong_ink}");
        // Then the strong one is wider and puts down more ink
        assert!(strong_width > regular_width, "strong Arabic is wider than regular: strong {strong_width} px, regular {regular_width} px");
        assert!(strong_ink as f64 > regular_ink as f64 * 1.05, "strong Arabic has more ink than regular: strong {strong_ink}, regular {regular_ink}");
    }

    /// The heavier face is for strong text only: regular and monospace text keep the face as the
    /// font file has it, and Latin text keeps Inter in both weights.
    #[test]
    fn regular_text_and_latin_are_unchanged_by_the_strong_variable_face() {
        // Given a build with evo-fonts whose Arabic face has a weight axis
        let Some(arabic) = variable_arabic_face() else {
            eprintln!("skipped: built without a variable-weight evo-fonts face for Arabic");
            return;
        };
        set_language(Locale::En);
        // When the UI builds its fonts
        let definitions = font_definitions(evolight_engine::EVO_FONTS);
        // Then the regular Arabic face is the font file untouched, and the only Arabic face of
        // the regular and monospace families
        let name = evo_font_name(arabic);
        assert_eq!(definitions.font_data.get(&name).map(|data| &data.tweak), Some(&egui::FontTweak::default()));
        let plain_weight = egui::epaint::text::VariationCoords::default();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let names = definitions.families.get(&family).expect("UI font family");
            assert!(names.contains(&name), "{family:?} falls back on {name}");
            for other in names.iter().filter(|other| **other != name) {
                let coords = definitions.font_data.get(other).map(|data| data.tweak.coords.clone()).unwrap_or_default();
                assert_eq!(coords, plain_weight, "{family:?}: {other} is drawn at the weight its file has");
            }
        }
        // And regular Arabic text is as wide as with that face alone after Inter
        let mut plain = FontDefinitions::default();
        plain.font_data.insert("Inter".into(), Arc::new(FontData::from_static(include_bytes!("../../../assets/fonts/Inter-Regular.ttf"))));
        plain.font_data.insert("Arabic".into(), Arc::new(FontData::from_static(arabic.bytes)));
        plain.families.insert(FontFamily::Proportional, vec!["Inter".into(), "Arabic".into()]);
        let (width, ink) = width_and_ink(definitions.clone(), ARABIC_WORD, FontFamily::Proportional);
        assert_eq!((width, ink), width_and_ink(plain, ARABIC_WORD, FontFamily::Proportional));
        // And Latin text, strong and regular, is as it is without evo-fonts
        for family in [FontFamily::Proportional, semibold_family()] {
            assert_eq!(
                width_and_ink(definitions.clone(), "Library", family.clone()),
                width_and_ink(font_definitions(&[]), "Library", family.clone()),
                "{family:?}"
            );
        }
        let (regular, _) = width_and_ink(definitions.clone(), "Library", FontFamily::Proportional);
        let (strong, _) = width_and_ink(definitions, "Library", semibold_family());
        assert!(strong > regular, "strong Latin stays wider than regular: {strong} px, {regular} px");
    }

    /// A face with one fixed weight is registered once: nothing heavier can be asked of it. Without
    /// evo-fonts there is nothing to register at all.
    #[test]
    fn a_static_face_has_no_strong_twin() {
        // Given a face without a weight axis for another script
        static FACES: &[evolight_engine::EvoFont] = &[evolight_engine::EvoFont {
            family: "Static Sans",
            style: "Regular",
            scripts: &["Thai"],
            bytes: include_bytes!("../../../assets/fonts/Inter-Regular.ttf"),
        }];
        set_language(Locale::En);
        for evo in [FACES, &[]] {
            // When the UI builds its fonts
            let definitions = font_definitions(evo);
            // Then no face is drawn at another weight than its file has, and the strong family
            // holds what the regular one does after Inter
            assert!(definitions.font_data.values().all(|data| data.tweak.coords == egui::epaint::text::VariationCoords::default()));
            let regular = definitions.families.get(&FontFamily::Proportional).expect("regular family");
            let strong = definitions.families.get(&semibold_family()).expect("strong family");
            assert_eq!(regular.get(1..), strong.get(1..));
            // Evora (E10): egui's emoji face is not loaded, so the count starts from base_definitions().
            assert_eq!(definitions.font_data.len(), base_definitions().font_data.len() + 2 + evo.len());
        }
    }
}
