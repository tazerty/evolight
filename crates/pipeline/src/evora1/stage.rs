//! Evora (EVORA-REV-112) : ce que le rendu d'Evora pose dans les trois structures du rendu, une par
//! étage. Chacune n'existe que sous le prédicat du rendu ([`renders`]) ; absente (`None`), le rendu
//! est celui de V6 au bit près.
//!
//! - [`Frame`], dans le plan du rendu (`Plan::evora1`) : l'étage linéaire. Elle porte le pont de
//!   scène et mène l'image de l'étage linéaire en lumière de scène ([`Frame::to_scene`]), une fois
//!   par image, après la réduction du bruit.
//! - [`Prep`], dans la préparation (`Prepared::evora1`) : l'étage des plans. Vide à ce commit ; les
//!   plans du rendu d'Evora y viendront.
//! - [`Kernel`], dans les paramètres du noyau par point (`FinishParams::evora1`) : il dit au noyau que
//!   la table de tons est celle d'Evora ([`super::tone::table`]), à borner au blanc après lecture.
//!   Depuis EVORA-REV-113 il porte aussi le retour de gamme ([`Kernel::gamut_return`]) : la matrice
//!   qui ramène de l'espace de sortie à l'espace de travail, et le pont, par lequel le noyau
//!   retrouve la source d'un point depuis sa couleur de scène.
//!
//! Les trois lisent le même pont, celui du rendu d'Evora ([`Bridge::default`]) : le pont aller de
//! l'étage linéaire et le pont de retour de la table de tons se défont l'un l'autre, et aux réglages
//! neutres le rendu d'un fichier est ce fichier.

use std::sync::Arc;

use evolight_color::REC2020;
use evolight_develop::DevelopSettings;
use evolight_raster::Rgb32f;

use super::bridge::Bridge;
use super::gamut;
use super::renders;
use crate::SourceInfo;
use crate::output::{DisplaySpace, OutputSpace};

/// Evora (EVORA-REV-112) : la luminance d'affichage du blanc. La table de tons d'Evora n'est pas
/// bornée (elle passe ce blanc au-delà du blanc de scène) : le noyau la borne ici après lecture
/// (`Pont.vers_affichage`, pont.py:33).
pub const DISPLAY_WHITE: f32 = 1.0;

/// Evora (EVORA-REV-112) : ce que le rendu d'Evora porte dans le plan d'un rendu (`Plan::evora1`).
/// Construite par [`Frame::for_plan`], sous le prédicat seulement.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    bridge: Bridge,
}

impl Frame {
    /// Evora (EVORA-REV-112) : la part d'Evora du plan d'un rendu de `s` sur la source `info`.
    /// `None` hors du prédicat ([`renders`]) : le rendu est alors celui de V6. `s` est le jeu de
    /// réglages du rendu, celui que [`crate::settings_for`] choisit pour la demande.
    pub fn for_plan(s: &DevelopSettings, info: &SourceInfo) -> Option<Arc<Frame>> {
        renders(s, info).then(|| Arc::new(Frame { bridge: Bridge::default() }))
    }

    /// Le pont de scène de ce rendu.
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Evora (EVORA-REV-112) : le pont aller sur l'image de l'étage linéaire, en place
    /// (`image_vers_scene`, pont.py:60-68). L'image arrive en lumière d'affichage, balance des blancs
    /// faite, retouchée et débruitée ; elle repart en lumière de scène. Tout ce qui suit (plans,
    /// masques, noyau par point) lit l'image de scène.
    pub fn to_scene(&self, image: &mut Rgb32f) {
        self.bridge.image_to_scene(image);
    }
}

/// Evora (EVORA-REV-112) : ce que le rendu d'Evora prépare pour le noyau par point
/// (`Prepared::evora1`), à côté des plans de l'amont. Vide à ce commit.
#[derive(Clone, Debug, Default)]
pub struct Prep {}

/// Evora (EVORA-REV-112) : ce que le rendu d'Evora dit au noyau par point (`FinishParams::evora1`).
/// Construit par [`Kernel::for_render`], sous le prédicat seulement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kernel {
    bridge: Bridge,
    /// Evora (EVORA-REV-113) : de l'espace de sortie linéaire au Rec.2020 linéaire, rangée par
    /// lignes : l'inverse de la matrice de sortie du noyau (`FinishParams::to_out`). Le retour de
    /// gamme y ramène la couleur qu'il a désaturée dans l'espace de sortie.
    from_out: [[f32; 3]; 3],
}

/// Evora (EVORA-REV-113) : la matrice de retour d'un noyau dont la sortie n'est pas encore dite :
/// celle de l'espace de travail vers lui-même.
const WORKING_SPACE: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

impl Kernel {
    /// Evora (EVORA-REV-112) : la part d'Evora des paramètres du noyau pour un rendu de `s` sur la
    /// source `info`. `None` hors du prédicat ([`renders`]).
    ///
    /// Evora (EVORA-REV-113) : le noyau naît dans l'espace de travail ; qui le pose dit ensuite sa
    /// sortie, par [`Kernel::in_space`] ou [`Kernel::on_display`].
    pub fn for_render(s: &DevelopSettings, info: &SourceInfo) -> Option<Kernel> {
        renders(s, info).then(|| Kernel { bridge: Bridge::default(), from_out: WORKING_SPACE })
    }

    /// Evora (EVORA-REV-113) : ce noyau, pour un rendu dans l'espace de sortie `space`. La matrice
    /// de retour est calculée en double précision, dans le sens du retour (elle n'est pas l'inverse
    /// en simple précision de la matrice de sortie), puis arrondie.
    pub fn in_space(self, space: OutputSpace) -> Kernel {
        Kernel { from_out: space.rgb_space().to_space(&REC2020).to_f32(), ..self }
    }

    /// Evora (EVORA-REV-113) : ce noyau, pour un rendu dans les primaires de l'écran `display`
    /// (`FinishParams::for_display`), qui porte déjà sa matrice de retour.
    pub fn on_display(self, display: &DisplaySpace) -> Kernel {
        Kernel { from_out: display.to_working, ..self }
    }

    /// Evora (EVORA-REV-113) : la matrice de retour du noyau, de sa sortie linéaire au Rec.2020
    /// linéaire.
    pub fn from_out(&self) -> &[[f32; 3]; 3] {
        &self.from_out
    }

    /// Le pont de scène de ce rendu : celui de la table de tons, et celui de [`Frame`].
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Evora (EVORA-REV-112) : la luminance d'affichage `o`, lue dans la table de tons d'Evora,
    /// bornée au blanc ([`DISPLAY_WHITE`]). Une valeur qui n'est pas un nombre passe telle quelle,
    /// comme dans la référence et comme sous V6 : le noyau rend alors du noir.
    #[inline]
    pub fn bound(&self, o: f32) -> f32 {
        if o > DISPLAY_WHITE { DISPLAY_WHITE } else { o }
    }

    /// Evora (EVORA-REV-113) : le retour de gamme du noyau (`retour_gamme`, evora1.py:278). `d` est
    /// la couleur d'affichage du point après les tons et l'échelle de chroma, `scene` sa couleur de
    /// scène telle que l'étage linéaire l'a laissée (avant l'exposition), `to_out` et `luma` la
    /// matrice de sortie du noyau et les poids de luminance de sa mise en gamme.
    ///
    /// C'est la loi de [`gamut::gamut_return`], écrite une seule fois ([`gamut::gamut_return_with`]),
    /// avec pour source la couleur d'affichage d'origine du point, retrouvée par le pont
    /// ([`Bridge::source_of`]) seulement quand elle peut compter : la référence du retour de gamme
    /// vaut au moins le blanc, et tant qu'aucun canal de sortie ne le passe, rien n'est poussé. Aux
    /// réglages neutres, l'arrondi peut faire passer d'un dernier chiffre les points qui touchent le
    /// bord de la gamme : depuis EVORA-REV-114 ils reçoivent la désaturation du moteur, et la teinte
    /// n'y est rendue qu'en proportion de l'excès relatif, soit l'arrondi multiplié par l'inverse de
    /// la marge, que leur source tienne dans la gamme ou en sorte déjà (ordre de grandeur mesuré dans
    /// le module [`gamut`]). La part de l'excès rendue en luminance et la marge sont celles du rendu
    /// ([`gamut::KEPT`], [`gamut::MARGIN`]).
    #[inline]
    pub fn gamut_return(&self, d: [f32; 3], scene: [f32; 3], to_out: &[[f32; 3]; 3], luma: [f32; 3]) -> [f32; 3] {
        gamut::gamut_return_with(d, || self.bridge.source_of(scene), to_out, &self.from_out, luma, gamut::KEPT, gamut::MARGIN)
    }
}

#[cfg(test)]
mod tests {
    use evolight_develop::{DevelopSettings, Process};

    use super::{DISPLAY_WHITE, Frame, Kernel};
    use crate::SourceInfo;
    use crate::evora1::bridge::Bridge;

    /// Les réglages par défaut sous `process`, rendu HDR allumé ou non.
    fn edit(process: Process, hdr: bool) -> DevelopSettings {
        let mut s = DevelopSettings { process: process.version(), ..Default::default() };
        s.hdr.enabled = hdr;
        s
    }

    /// Evora (EVORA-REV-112) : les structures du rendu d'Evora n'existent que sous le prédicat, et
    /// lisent toutes le pont du rendu ; la borne du noyau ne touche que ce qui passe le blanc.
    ///
    /// Sabotages qui le font rougir : une structure construite sans le prédicat (un brut, un rendu HDR
    /// ou une version de l'amont en recevrait une) ; un pont propre à l'une des deux (le pont aller et
    /// le pont de retour ne se déferaient plus) ; la borne écrite par un minimum, qui rendrait le blanc
    /// pour une luminance qui n'est pas un nombre.
    #[test]
    fn stage_parts_exist_only_under_the_predicate() {
        let file = SourceInfo::default();
        let raw = SourceInfo { raw: true, ..Default::default() };
        assert!(!file.raw, "la source par défaut de ce test est un fichier rendu");

        let evora = edit(Process::Evora1, false);
        let (Some(frame), Some(kernel)) = (Frame::for_plan(&evora, &file), Kernel::for_render(&evora, &file)) else {
            panic!("fichier rendu, SDR, Evora1 : structures du rendu d'Evora absentes");
        };
        assert_eq!(frame.bridge(), &Bridge::default(), "le plan porte le pont du rendu");
        assert_eq!(kernel.bridge(), frame.bridge(), "le noyau et le plan lisent le même pont");

        let (v6, hdr) = (edit(Process::V6, false), edit(Process::Evora1, true));
        assert!(hdr.hdr.peak() > 1.0, "garde de portée : l'édition HDR de ce test a une marge au-dessus du blanc SDR");
        for (s, info, what) in [(&v6, &file, "V6"), (&evora, &raw, "brut"), (&hdr, &file, "rendu HDR")] {
            assert!(Frame::for_plan(s, info).is_none(), "{what} : le plan reçoit une part d'Evora");
            assert!(Kernel::for_render(s, info).is_none(), "{what} : le noyau reçoit une part d'Evora");
        }

        // la borne : le blanc pour tout ce qui le passe, rien d'autre
        for o in [0.0, 0.18, 0.999_999_9, DISPLAY_WHITE] {
            assert_eq!(kernel.bound(o), o, "luminance {o} sous le blanc : touchée");
        }
        for o in [1.000_000_1, 1.013, f32::INFINITY] {
            assert_eq!(kernel.bound(o), DISPLAY_WHITE, "luminance {o} au-dessus du blanc : non bornée");
        }
        assert!(kernel.bound(f32::NAN).is_nan(), "une luminance qui n'est pas un nombre passe telle quelle");
        // témoin rouge : le minimum, lui, rendrait le blanc
        assert_eq!(f32::NAN.min(DISPLAY_WHITE), DISPLAY_WHITE, "témoin : un minimum ne rend plus le blanc pour ce qui n'est pas un nombre");
    }
}
