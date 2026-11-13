//! Evora (EVORA-REV-99) : le rendu d'Evora, celui de la version du rendu `Evora1` (numéro 1001).
//!
//! Le module porte le prédicat du rendu, [`renders`], les constantes nommées du rendu (plus bas), le
//! pont de scène ([`bridge`], depuis EVORA-REV-110) et la table de tons du rendu ([`tone`], depuis
//! EVORA-REV-111). Hors du prédicat (un brut, un rendu HDR, une version de l'amont), rien de ce module
//! n'agit sur le rendu : un brut et un rendu HDR sous `Evora1` rendent comme V6, au bit près (bras
//! recopiés d'EVORA-REV-98). Le chemin graphique lit le prédicat, pour laisser au chemin principal
//! les rendus d'Evora (`run`, dans `evolight_gpu`).
//!
//! Depuis EVORA-REV-112 le rendu d'Evora est branché. Sous le prédicat (le rendu SDR d'un fichier
//! rendu sous `Evora1`), le chemin principal lit ce module en trois endroits, par les structures de
//! [`stage`] :
//! - à la fin de l'étage linéaire, l'image passe le pont aller ([`Frame::to_scene`]) : tout ce qui
//!   suit lit une image de scène, et les clés de cache de cet étage portent la marque d'Evora ;
//! - la table de tons du noyau par point est celle d'Evora ([`tone::table`], choisie par
//!   `finish::base_tone`), qui finit par le pont de retour ;
//! - le noyau borne au blanc la luminance lue dans cette table ([`Kernel::bound`]).
//!
//! Depuis EVORA-REV-113 le noyau ramène aussi dans la gamme de sortie, sans la laisser virer, une
//! couleur vive que les tons en ont fait sortir par le haut ([`gamut`], [`Kernel::gamut_return`]),
//! juste avant la borne à la crête de l'amont, qui sous le prédicat est la borne finale du rendu
//! d'Evora.
//!
//! Aux réglages neutres, le rendu d'un fichier est ce fichier, à une réserve près, que pose le retour
//! de gamme : il juge la couleur contre sa source sans marge (`mx > ref`, comme la référence,
//! pont.py:88-89), et le bruit d'arrondi du pont peut l'éveiller. Si la source tient dans la gamme de
//! sortie, il ne ramène en général la couleur que de quelques millièmes de code (un canal Rec.2020
//! négatif de la source, que OkLab lit à zéro, y est toutefois perdu : quelques codes pour le rouge
//! d'un fichier P3 rendu en P3) ; si elle en sort déjà (un fichier P3 rendu en sRGB), il la ramène
//! d'un coup, de plusieurs dizaines de codes (`tests_evora1::wired::pushed`, test
//! `neutral_wide_file_wakes_the_gamut_return`). Question portée au propriétaire de la référence.
//!
//! Les curseurs que le rendu d'Evora ne reprend pas encore (tons clairs et sombres, contraste local,
//! relief fin, atténuation de la brume, couleur) appliquent pour l'instant leur loi de V6 à l'image
//! de scène : état intermédiaire, que les commits suivants soldent un à un.
//!
//! Règle d'écriture du rendu d'Evora : chaque branche neuve du pipeline se ferme par ce prédicat, et
//! par lui seul. Le code d'Evora ne range jamais deux versions du rendu par leur ordre.

// Evora (EVORA-REV-110) : le pont de scène, entre la lumière d'affichage d'un fichier rendu et la
// lumière de scène où les curseurs d'Evora agissent.
pub mod bridge;
// Evora (EVORA-REV-111) : la table de tons du rendu d'Evora, contraste, blancs et noirs en EV de
// scène. Dans ce fichier, `tone` désigne désormais ce sous-module : la table de tons du moteur
// s'écrit `crate::tone`.
pub mod tone;
// Evora (EVORA-REV-112) : ce que le rendu d'Evora pose dans le plan, la préparation et les
// paramètres du noyau, une structure par étage.
pub mod stage;
// Evora (EVORA-REV-113) : le retour de gamme, qui ramène dans la gamme de sortie une couleur que les
// tons en ont fait sortir par le haut.
pub mod gamut;

pub use stage::{Frame, Kernel, Prep};

use evolight_develop::DevelopSettings;

use crate::SourceInfo;

/// Evora (EVORA-REV-99) : vrai si le rendu de `s` sur la source `info` est celui d'Evora. Trois
/// conditions, toutes requises :
/// - la version du rendu de la photo est une version d'Evora : `Evora1`, ou un numéro plus récent
///   de la plage d'Evora, qui se lit `Evora1` ;
/// - la source n'est pas un brut ;
/// - le rendu n'est pas un rendu HDR : sa crête ([`evolight_develop::Hdr::peak`]) ne dépasse pas le
///   blanc SDR.
///
/// Faux, le rendu est celui de V6 au bit près. C'est le cas d'un brut et d'un rendu HDR sous une
/// version d'Evora (`tests_evora1::evora1_raw_matches_v6`, `tests_evora1::evora1_hdr_render_matches_v6`).
///
/// `s` est le jeu de réglages du rendu, celui que [`crate::settings_for`] choisit pour la demande, et
/// non l'édition telle qu'elle est enregistrée. Une édition HDR rendue en 8 ou 16 bits l'est par sa
/// déclinaison SDR, rendu HDR éteint : c'est un rendu d'Evora. Rendue en flottants HDR, elle garde sa
/// marge au-dessus du blanc SDR : c'est un rendu de V6. Le prédicat interrogé avant ce choix
/// répondrait « V6 » pour les deux.
pub fn renders(s: &DevelopSettings, info: &SourceInfo) -> bool {
    s.process.process().is_evora() && !info.raw && s.hdr.peak() <= 1.0
}

// --- Evora (EVORA-REV-110) : constantes nommées du rendu d'Evora -----------------------------------
//
// Ce fichier est le seul à porter les réglages du rendu d'Evora. Chaque constante est un champ de
// `Parametres`, dans la référence du rendu (evora1.py:28-137), sous le nom anglais du plan de portage ;
// le champ d'origine est donné entre accents graves. Aucune valeur n'est choisie ici : la référence
// fait foi, et `tests::parameters_are_the_reference_ones` confronte chaque constante à la valeur
// relevée par outil dans la référence. Les valeurs sont en double précision, comme dans la
// référence ; le code par point les arrondit en simple précision là où il les lit.
//
// Sont lues : les constantes du pont (`bridge`), depuis EVORA-REV-111 celles du contraste, des blancs
// et des noirs (`tone`), et depuis EVORA-REV-113 `GAMUT_KEPT` (`gamut`). Les autres attendent les
// commits qui portent leur loi.

/// Evora (EVORA-REV-110) : le gris moyen de la référence (`GRIS`, couleur.py:13), en double
/// précision. Le pont se calcule depuis cette valeur exacte et non depuis [`crate::tone::GREY`], le
/// même gris en simple précision : élargi en double précision, celui-ci écarte `q` de 1,7e-9.
pub const GREY: f64 = 0.18;
/// Evora (EVORA-REV-110) : luminance au-dessus de laquelle le pont applique son gain, dans les deux
/// sens (pont.py:67). Seuil strict : à cette luminance et en dessous, le point passe tel quel.
pub const LUMINANCE_FLOOR: f64 = 1e-9;

// Pont de scène (evora1.py:29-33).
/// `pont_n` : exposant du pont (pente du pied, en puissance).
pub const BRIDGE_N: f64 = 1.3;
/// `pont_blanc` : blanc de scène, la luminance de scène que le pont rend au blanc d'affichage.
pub const BRIDGE_WHITE: f64 = 16.0;
/// `chroma_min` : échelle de chroma au blanc d'affichage.
pub const CHROMA_MIN: f64 = 0.8;
/// `chroma_de` : luminance d'affichage d'où l'échelle de chroma descend vers `CHROMA_MIN`.
pub const CHROMA_FROM: f64 = 0.45;

// Contraste, blancs, noirs, en EV de scène autour du gris moyen (evora1.py:34-44).
/// `k_contraste` : à fond de curseur, le contraste multiplie la pente des EV au pivot par 2 puissance
/// cette valeur.
pub const K_CONTRAST: f64 = 0.55;
/// `contraste_pivot` : EV de scène du ton qui ne bouge pas.
pub const CONTRAST_PIVOT: f64 = 0.0;
/// `contraste_pied` : portée du contraste vers les ombres, en EV (au-delà, il s'éteint).
pub const CONTRAST_TOE: f64 = 3.0;
/// `contraste_epaule` : portée du contraste vers les clairs, en EV.
pub const CONTRAST_SHOULDER: f64 = 4.0;
/// `k_blancs`.
pub const K_WHITES: f64 = 2.4;
/// `blancs_de`.
pub const WHITES_FROM: f64 = 0.7;
/// `blancs_a`.
pub const WHITES_TO: f64 = 5.5;
/// `k_noirs`.
pub const K_BLACKS: f64 = 1.7;
/// `noirs_de`.
pub const BLACKS_FROM: f64 = -5.0;
/// `noirs_a`.
pub const BLACKS_TO: f64 = 0.0;
/// `gamme_retenue` (evora1.py:45-47) : pour une couleur vive que les tons poussent hors de la gamme
/// de sortie, part de l'excès rendue en luminance (teinte et richesse gardées) ; le reste part en
/// désaturation.
pub const GAMUT_KEPT: f64 = 0.5;

// Hautes lumières et ombres, sur une base à bords gardés (evora1.py:48-65).
/// `k_hautes` : EV retirés au plus quand le curseur descend.
pub const K_HIGHLIGHTS: f64 = 1.9;
/// `k_hautes_plus` : EV ajoutés au plus quand il monte.
pub const K_HIGHLIGHTS_UP: f64 = 1.5;
/// `hautes_de`. La référence l'a déplacé après le plan de portage, qui écrivait 1,2.
pub const HIGHLIGHTS_FROM: f64 = 0.7;
/// `hautes_a`.
pub const HIGHLIGHTS_TO: f64 = 5.0;
/// `hautes_garde` : part de la baisse rendue aux blancs purs (lampes, reflets).
pub const HIGHLIGHTS_GUARD: f64 = 0.45;
/// `hautes_garde_de`.
pub const HIGHLIGHTS_GUARD_FROM: f64 = 5.2;
/// `hautes_garde_a`.
pub const HIGHLIGHTS_GUARD_TO: f64 = 6.5;
/// `k_ombres` : EV ajoutés au plus quand le curseur monte.
pub const K_SHADOWS: f64 = 1.8;
/// `k_ombres_moins` : EV retirés au plus quand il descend.
pub const K_SHADOWS_DOWN: f64 = 1.3;
/// `ombres_de`.
pub const SHADOWS_FROM: f64 = -4.5;
/// `ombres_a`.
pub const SHADOWS_TO: f64 = 0.5;
/// `ombres_ancre` : part de la hausse refusée aux noirs profonds (le noir reste noir).
pub const SHADOWS_ANCHOR: f64 = 0.65;
/// `ombres_ancre_de`.
pub const SHADOWS_ANCHOR_FROM: f64 = -8.0;
/// `ombres_ancre_a`.
pub const SHADOWS_ANCHOR_TO: f64 = -3.5;
/// `base_sigma` : sigma de la base, en fraction du bord long.
pub const BASE_SIGMA: f64 = 0.015;
/// `base_eps`.
pub const BASE_EPS_EVORA: f64 = 0.15;
/// `base_median` : rayon du médian de la base, en fraction du bord long. C'est déjà
/// [`crate::local::BASE_DETAIL`] dans le moteur ; la fiche des constantes tient l'égalité.
pub const BASE_MEDIAN: f64 = 0.0015;

// Contraste local (evora1.py:66-75).
/// `local_sigma`, en fraction du bord long.
pub const LOCAL_SIGMA: f64 = 0.012;
/// `local_eps`.
pub const LOCAL_EPS: f64 = 0.3;
/// `k_local`.
pub const K_LOCAL: f64 = 3.4;
/// `local_tau`.
pub const LOCAL_TAU: f64 = 0.8;
/// `local_bruit`.
pub const LOCAL_NOISE: f64 = 0.03;
/// `k_local_moins`.
pub const K_LOCAL_DOWN: f64 = 0.9;
/// `local_part_fin` : part de la bande fine jointe à la bande moyenne (contraste local positif).
pub const LOCAL_FINE_SHARE: f64 = 0.5;
/// `local_tau_moins`.
pub const LOCAL_TAU_DOWN: f64 = 0.8;
/// `local_garde` : dispersion locale, en EV², où le contraste local est divisé par deux (garde de
/// contour).
pub const LOCAL_GUARD: f64 = 2.0;

// Relief fin (evora1.py:76-85).
/// `fin_sigma`, en fraction du bord long.
pub const FINE_SIGMA: f64 = 0.0018;
/// `fin_eps`.
pub const FINE_EPS: f64 = 0.1;
/// `k_fin`.
pub const K_FINE: f64 = 4.2;
/// `fin_tau`.
pub const FINE_TAU: f64 = 0.3;
/// `fin_bruit`.
pub const FINE_NOISE: f64 = 0.04;
/// `k_fin_moins`.
pub const K_FINE_DOWN: f64 = 0.9;
/// `fin_tau_moins`.
pub const FINE_TAU_DOWN: f64 = 0.5;
/// `fin_garde` : la même garde de contour pour le relief fin, à son échelle.
pub const FINE_GUARD: f64 = 1.0;
/// `fin_creux` : part gardée du creusement (détail plus sombre que son entourage).
pub const FINE_HOLLOW: f64 = 0.7;

// Garde de la peau, commune au contraste local et au relief fin positifs (evora1.py:86-91).
/// `peau_largeur` : demi-largeur de teinte, en radians, autour de la teinte de peau.
pub const SKIN_WIDTH: f64 = 0.35;
/// `peau_c_de` : chroma où la garde commence (un gris n'a pas de teinte).
pub const SKIN_C_FROM: f64 = 0.02;
/// `peau_c_a`.
pub const SKIN_C_TO: f64 = 0.06;
/// `local_peau` : part du contraste local positif retirée sur la peau.
pub const LOCAL_SKIN: f64 = 0.4;
/// `fin_peau` : part du relief fin positif retirée sur la peau.
pub const FINE_SKIN: f64 = 0.5;

// Atténuation de la brume (evora1.py:92-118).
/// `voile_sigma`, en fraction du bord long.
pub const VEIL_SIGMA: f64 = 0.02;
/// `voile_erosion`, en fraction du bord long.
pub const VEIL_EROSION: f64 = 0.004;
/// `voile_eps`.
pub const VEIL_EPS: f64 = 0.01;
/// `voile_omega`.
pub const VEIL_OMEGA: f64 = 0.9;
/// `voile_omega_zone`.
pub const VEIL_OMEGA_ZONE: f64 = 0.95;
/// `voile_zone`, en fraction du bord long.
pub const VEIL_ZONE: f64 = 0.15;
/// `voile_zone_cases` : un nombre de cases, entier dans la référence.
pub const VEIL_ZONE_CELLS: usize = 8;
/// `voile_zone_part`.
pub const VEIL_ZONE_SHARE: f64 = 0.95;
/// `voile_zone_marge`.
pub const VEIL_ZONE_MARGIN: f64 = 0.02;
/// `voile_zone_max`.
pub const VEIL_ZONE_MAX: f64 = 0.8;
/// `voile_t_min`.
pub const VEIL_T_MIN: f64 = 0.3;
/// `voile_plancher_ev`.
pub const VEIL_FLOOR_EV: f64 = 3.5;
/// `voile_physique`.
pub const VEIL_PHYSICAL: f64 = 0.7;
/// `voile_blanc_de`.
pub const VEIL_WHITE_FROM: f64 = 0.25;
/// `voile_blanc_a`.
pub const VEIL_WHITE_TO: f64 = 0.9;
/// `voile_haut`.
pub const VEIL_UP: f64 = 0.8;
/// `voile_kappa`.
pub const VEIL_KAPPA: f64 = 1.0;
/// `voile_chroma_fond`.
pub const VEIL_CHROMA_ZONE: f64 = 0.14;
/// `voile_teinte`.
pub const VEIL_TINT: f64 = 0.35;
/// `brume_max`.
pub const HAZE_MAX: f64 = 0.38;
/// `brume_courbe`.
pub const HAZE_CURVE: f64 = 1.5;
/// `brume_fond`.
pub const HAZE_FLOOR: f64 = 0.35;
/// `brume_niveau`.
pub const HAZE_LEVEL: f64 = 0.5;
/// `brume_lumiere_min_ev` : plancher de la lumière de brume, en EV sous le gris moyen. Champ ajouté
/// à la référence après le plan de portage, qui ne lui donnait pas de nom : celui-ci est d'EVORA-REV-110.
pub const HAZE_LIGHT_MIN_EV: f64 = -0.5;
/// `brume_teinte`.
pub const HAZE_TINT: f64 = 0.5;
/// `brume_chroma`.
pub const HAZE_CHROMA: f64 = 0.6;

// Vibrance et saturation (evora1.py:119-137).
/// `k_vibrance`.
pub const K_VIBRANCE: f64 = 2.1;
/// `vibrance_neutre`.
pub const VIBRANCE_NEUTRAL: f64 = 0.12;
/// `vibrance_c_neutre`.
pub const VIBRANCE_C_NEUTRAL: f64 = 0.09;
/// `vibrance_ombre`.
pub const VIBRANCE_SHADOW: f64 = 0.35;
/// `vibrance_l_ombre`.
pub const VIBRANCE_L_SHADOW: f64 = 0.4;
/// `vibrance_peau`.
pub const VIBRANCE_SKIN: f64 = 0.7;
/// `vibrance_peau_largeur`.
pub const VIBRANCE_SKIN_WIDTH: f64 = 0.35;
/// `vibrance_retrait`.
pub const VIBRANCE_PULL: f64 = 0.5;
/// `vibrance_tasse`.
pub const VIBRANCE_PACK: f64 = 1.0;
/// `vibrance_peau_moins`.
pub const VIBRANCE_SKIN_DOWN: f64 = 0.3;
/// `vibrance_c_vive`.
pub const VIBRANCE_C_VIVID: f64 = 0.22;
/// `k_saturation`.
pub const K_SATURATION: f64 = 0.9;
/// `saturation_neutre`.
pub const SATURATION_NEUTRAL: f64 = 0.2;
/// `saturation_c_neutre`.
pub const SATURATION_C_NEUTRAL: f64 = 0.1;
/// `saturation_ombre`.
pub const SATURATION_SHADOW: f64 = 0.5;
/// `saturation_l_ombre`.
pub const SATURATION_L_SHADOW: f64 = 0.4;
/// `saturation_peau`.
pub const SATURATION_SKIN: f64 = 0.25;
/// `densite`.
pub const DENSITY: f64 = 0.5;

// Hors de `Parametres` : valeurs que la référence écrit ailleurs, relevées à l'ancre citée. Les noms
// des six dernières sont d'EVORA-REV-110 (la référence les écrit en clair dans ses formules).
/// `CHROMA_HAUT` (chroma.py:22) : chroma d'où part la recherche de la limite de la gamme.
pub const CHROMA_TOP: f64 = 0.6;
/// `PAS_RECHERCHE` (chroma.py:23) : nombre de pas de la dichotomie de cette recherche.
pub const SEARCH_STEPS: usize = 16;
/// `TABLE_LUMS` (chroma.py:24) : luminosités de la table de gamme.
pub const TABLE_LUMS: usize = 65;
/// `TABLE_TEINTES` (chroma.py:25) : teintes de la table de gamme.
pub const TABLE_HUES: usize = 256;
/// `MARGE_TABLE` (chroma.py:26) : la table est lue un rien en dessous de la limite exacte.
pub const TABLE_MARGIN: f64 = 0.985;
/// Terme de milieu du contraste local, `plancher + part · exp(−(EV / largeur)²)` (evora1.py:255) :
/// son plancher.
pub const LOCAL_MIDDLE_FLOOR: f64 = 0.35;
/// Terme de milieu du contraste local : sa part variable.
pub const LOCAL_MIDDLE_SHARE: f64 = 0.65;
/// Terme de milieu du contraste local : sa largeur, en EV.
pub const LOCAL_MIDDLE_WIDTH_EV: f64 = 3.2;
/// Plus petit sigma, en points, de la base, du contraste local et du voile (evora1.py:232, 254 et
/// 259 ; voile.py:30).
pub const SIGMA_MIN: f64 = 1.0;
/// Plus petit sigma, en points, du relief fin (evora1.py:248 et 266).
pub const FINE_SIGMA_MIN: f64 = 0.6;
/// Centile du plan sombre au-dessus duquel la teinte du voile est mesurée (voile.py:43).
pub const VEIL_TINT_PERCENTILE: f64 = 99.0;

/// Evora (EVORA-REV-110) : le passage lisse de 0 à 1 entre `e0` et `e1` (`lisse`, couleur.py:81-84),
/// en double précision, pour ce que le rendu d'Evora calcule une fois (nœuds, tables). C'est la
/// formule des copies du moteur, qui sont privées à leurs modules et en simple précision.
fn smooth(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use evolight_develop::{DevelopSettings, EVORA_BASE, Process, ProcessVersion};

    use super::renders;
    use crate::{OutputDepth, RenderRequest, SourceInfo, settings_for};

    /// Une photo neuve sous le numéro de version `process`, rendu HDR allumé ou non (marge par défaut).
    fn edit(process: ProcessVersion, hdr: bool) -> DevelopSettings {
        let mut s = DevelopSettings { process, ..Default::default() };
        s.hdr.enabled = hdr;
        s
    }

    /// Evora (EVORA-REV-99) : la table de vérité du prédicat. Vrai quand les trois conditions sont
    /// réunies ; faux dès qu'une seule manque (version de l'amont, brut, rendu HDR) ; et, pour une
    /// édition HDR, jugé sur le jeu de réglages que le rendu choisit pour la demande.
    ///
    /// Sabotages qui le font rougir : une des trois conditions retirée du prédicat ; `<=` devenu
    /// `<` sur la crête (plus aucun rendu SDR n'est d'Evora) ; la plage du numéro lue à la place de
    /// la version résolue (1000 passerait pour une version d'Evora).
    #[test]
    fn renders_only_the_sdr_render_of_a_rendered_file_under_evora() {
        let file = SourceInfo::default();
        let raw = SourceInfo { raw: true, ..Default::default() };
        assert!(!file.raw, "la source par défaut de ce test est un fichier rendu");
        let evora = Process::Evora1.version();

        // les trois conditions réunies
        assert!(renders(&edit(evora, false), &file), "fichier rendu, SDR, Evora1");
        // un numéro plus récent de la plage d'Evora se lit Evora1
        assert!(renders(&edit(ProcessVersion(evora.0 + 1), false), &file), "fichier rendu, SDR, numéro d'Evora suivant");

        // la version : aucune version de l'amont, ni un numéro qui se lit V6
        for process in [Process::V1, Process::V2, Process::V3, Process::V4, Process::V5, Process::V6] {
            assert!(!renders(&edit(process.version(), false), &file), "fichier rendu, SDR, {process:?}");
        }
        for number in [0, 7, EVORA_BASE] {
            assert!(!renders(&edit(ProcessVersion(number), false), &file), "fichier rendu, SDR, numéro {number}");
        }
        // la source : un brut garde le rendu de V6
        assert!(!renders(&edit(evora, false), &raw), "brut, SDR, Evora1");
        // la crête : un rendu HDR garde le rendu de V6
        let hdr = edit(evora, true);
        assert!(hdr.hdr.peak() > 1.0, "garde de portée : l'édition HDR de ce test a une marge au-dessus du blanc SDR");
        assert!(!renders(&hdr, &file), "fichier rendu, HDR, Evora1");
        assert!(!renders(&hdr, &raw), "brut, HDR, Evora1");
        // sans marge (0 stop au-dessus du blanc SDR), la crête vaut le blanc SDR : rendu d'Evora
        let mut flat = hdr.clone();
        flat.hdr.max_ev = 0.0;
        assert!(renders(&flat, &file), "fichier rendu, HDR sans marge, Evora1");

        // une édition HDR, jugée sur le jeu de réglages du rendu
        let float = RenderRequest { depth: OutputDepth::F32Hdr, ..RenderRequest::fit(64, 64) };
        let as_hdr = settings_for(&hdr, &float);
        assert!(!renders(&as_hdr, &file), "édition HDR d'un fichier rendu, sortie en flottants HDR : rendu de V6");
        for depth in [OutputDepth::U8, OutputDepth::U16] {
            let as_sdr = settings_for(&hdr, &RenderRequest { depth, ..RenderRequest::fit(64, 64) });
            assert!(renders(&as_sdr, &file), "édition HDR d'un fichier rendu, sortie {depth:?} : rendu d'Evora");
            assert!(!renders(&as_sdr, &raw), "édition HDR d'un brut, sortie {depth:?} : rendu de V6");
        }
    }

    // --- Evora (EVORA-REV-110) : fiche des constantes nommées du rendu d'Evora ---------------------
    //
    // Les tableaux qui suivent sont écrits par outil (`poser_rev110.py`, à côté des vecteurs d'or),
    // depuis le relevé des réglages de la référence (`00-parametres.json`) et depuis la référence
    // elle-même. Ils ne se retouchent pas à la main ; ils se régénèrent.

    // >>> or:PARAMETRES (EVORA-REV-110 : écrit par poser_rev110.py depuis 00-parametres.json et texte de la référence ; ne pas retoucher)
    /// Evora (EVORA-REV-110) : (champ de `Parametres`, valeur dans la référence), evora1.py:28-137, dans l'ordre de la référence.
    #[rustfmt::skip]
    const GOLD_PARAMETERS: [(&str, f64); 99] = [
        ("pont_n", 1.3),
        ("pont_blanc", 16.0),
        ("chroma_min", 0.8),
        ("chroma_de", 0.45),
        ("k_contraste", 0.55),
        ("contraste_pivot", 0.0),
        ("contraste_pied", 3.0),
        ("contraste_epaule", 4.0),
        ("k_blancs", 2.4),
        ("blancs_de", 0.7),
        ("blancs_a", 5.5),
        ("k_noirs", 1.7),
        ("noirs_de", -5.0),
        ("noirs_a", 0.0),
        ("gamme_retenue", 0.5),
        ("k_hautes", 1.9),
        ("k_hautes_plus", 1.5),
        ("hautes_de", 0.7),
        ("hautes_a", 5.0),
        ("hautes_garde", 0.45),
        ("hautes_garde_de", 5.2),
        ("hautes_garde_a", 6.5),
        ("k_ombres", 1.8),
        ("k_ombres_moins", 1.3),
        ("ombres_de", -4.5),
        ("ombres_a", 0.5),
        ("ombres_ancre", 0.65),
        ("ombres_ancre_de", -8.0),
        ("ombres_ancre_a", -3.5),
        ("base_sigma", 0.015),
        ("base_eps", 0.15),
        ("base_median", 0.0015),
        ("local_sigma", 0.012),
        ("local_eps", 0.3),
        ("k_local", 3.4),
        ("local_tau", 0.8),
        ("local_bruit", 0.03),
        ("k_local_moins", 0.9),
        ("local_part_fin", 0.5),
        ("local_tau_moins", 0.8),
        ("local_garde", 2.0),
        ("fin_sigma", 0.0018),
        ("fin_eps", 0.1),
        ("k_fin", 4.2),
        ("fin_tau", 0.3),
        ("fin_bruit", 0.04),
        ("k_fin_moins", 0.9),
        ("fin_tau_moins", 0.5),
        ("fin_garde", 1.0),
        ("fin_creux", 0.7),
        ("peau_largeur", 0.35),
        ("peau_c_de", 0.02),
        ("peau_c_a", 0.06),
        ("local_peau", 0.4),
        ("fin_peau", 0.5),
        ("voile_sigma", 0.02),
        ("voile_erosion", 0.004),
        ("voile_eps", 0.01),
        ("voile_omega", 0.9),
        ("voile_omega_zone", 0.95),
        ("voile_zone", 0.15),
        ("voile_zone_cases", 8.0),
        ("voile_zone_part", 0.95),
        ("voile_zone_marge", 0.02),
        ("voile_zone_max", 0.8),
        ("voile_t_min", 0.3),
        ("voile_plancher_ev", 3.5),
        ("voile_physique", 0.7),
        ("voile_blanc_de", 0.25),
        ("voile_blanc_a", 0.9),
        ("voile_haut", 0.8),
        ("voile_kappa", 1.0),
        ("voile_chroma_fond", 0.14),
        ("voile_teinte", 0.35),
        ("brume_max", 0.38),
        ("brume_courbe", 1.5),
        ("brume_fond", 0.35),
        ("brume_niveau", 0.5),
        ("brume_lumiere_min_ev", -0.5),
        ("brume_teinte", 0.5),
        ("brume_chroma", 0.6),
        ("k_vibrance", 2.1),
        ("vibrance_neutre", 0.12),
        ("vibrance_c_neutre", 0.09),
        ("vibrance_ombre", 0.35),
        ("vibrance_l_ombre", 0.4),
        ("vibrance_peau", 0.7),
        ("vibrance_peau_largeur", 0.35),
        ("vibrance_retrait", 0.5),
        ("vibrance_tasse", 1.0),
        ("vibrance_peau_moins", 0.3),
        ("vibrance_c_vive", 0.22),
        ("k_saturation", 0.9),
        ("saturation_neutre", 0.2),
        ("saturation_c_neutre", 0.1),
        ("saturation_ombre", 0.5),
        ("saturation_l_ombre", 0.4),
        ("saturation_peau", 0.25),
        ("densite", 0.5),
    ];

    /// Evora (EVORA-REV-110) : (champ de `Parametres`, constante du moteur qui le porte), dans le même ordre.
    #[rustfmt::skip]
    const PORTED_PARAMETERS: [(&str, f64); 99] = [
        ("pont_n", super::BRIDGE_N),
        ("pont_blanc", super::BRIDGE_WHITE),
        ("chroma_min", super::CHROMA_MIN),
        ("chroma_de", super::CHROMA_FROM),
        ("k_contraste", super::K_CONTRAST),
        ("contraste_pivot", super::CONTRAST_PIVOT),
        ("contraste_pied", super::CONTRAST_TOE),
        ("contraste_epaule", super::CONTRAST_SHOULDER),
        ("k_blancs", super::K_WHITES),
        ("blancs_de", super::WHITES_FROM),
        ("blancs_a", super::WHITES_TO),
        ("k_noirs", super::K_BLACKS),
        ("noirs_de", super::BLACKS_FROM),
        ("noirs_a", super::BLACKS_TO),
        ("gamme_retenue", super::GAMUT_KEPT),
        ("k_hautes", super::K_HIGHLIGHTS),
        ("k_hautes_plus", super::K_HIGHLIGHTS_UP),
        ("hautes_de", super::HIGHLIGHTS_FROM),
        ("hautes_a", super::HIGHLIGHTS_TO),
        ("hautes_garde", super::HIGHLIGHTS_GUARD),
        ("hautes_garde_de", super::HIGHLIGHTS_GUARD_FROM),
        ("hautes_garde_a", super::HIGHLIGHTS_GUARD_TO),
        ("k_ombres", super::K_SHADOWS),
        ("k_ombres_moins", super::K_SHADOWS_DOWN),
        ("ombres_de", super::SHADOWS_FROM),
        ("ombres_a", super::SHADOWS_TO),
        ("ombres_ancre", super::SHADOWS_ANCHOR),
        ("ombres_ancre_de", super::SHADOWS_ANCHOR_FROM),
        ("ombres_ancre_a", super::SHADOWS_ANCHOR_TO),
        ("base_sigma", super::BASE_SIGMA),
        ("base_eps", super::BASE_EPS_EVORA),
        ("base_median", super::BASE_MEDIAN),
        ("local_sigma", super::LOCAL_SIGMA),
        ("local_eps", super::LOCAL_EPS),
        ("k_local", super::K_LOCAL),
        ("local_tau", super::LOCAL_TAU),
        ("local_bruit", super::LOCAL_NOISE),
        ("k_local_moins", super::K_LOCAL_DOWN),
        ("local_part_fin", super::LOCAL_FINE_SHARE),
        ("local_tau_moins", super::LOCAL_TAU_DOWN),
        ("local_garde", super::LOCAL_GUARD),
        ("fin_sigma", super::FINE_SIGMA),
        ("fin_eps", super::FINE_EPS),
        ("k_fin", super::K_FINE),
        ("fin_tau", super::FINE_TAU),
        ("fin_bruit", super::FINE_NOISE),
        ("k_fin_moins", super::K_FINE_DOWN),
        ("fin_tau_moins", super::FINE_TAU_DOWN),
        ("fin_garde", super::FINE_GUARD),
        ("fin_creux", super::FINE_HOLLOW),
        ("peau_largeur", super::SKIN_WIDTH),
        ("peau_c_de", super::SKIN_C_FROM),
        ("peau_c_a", super::SKIN_C_TO),
        ("local_peau", super::LOCAL_SKIN),
        ("fin_peau", super::FINE_SKIN),
        ("voile_sigma", super::VEIL_SIGMA),
        ("voile_erosion", super::VEIL_EROSION),
        ("voile_eps", super::VEIL_EPS),
        ("voile_omega", super::VEIL_OMEGA),
        ("voile_omega_zone", super::VEIL_OMEGA_ZONE),
        ("voile_zone", super::VEIL_ZONE),
        ("voile_zone_cases", super::VEIL_ZONE_CELLS as f64),
        ("voile_zone_part", super::VEIL_ZONE_SHARE),
        ("voile_zone_marge", super::VEIL_ZONE_MARGIN),
        ("voile_zone_max", super::VEIL_ZONE_MAX),
        ("voile_t_min", super::VEIL_T_MIN),
        ("voile_plancher_ev", super::VEIL_FLOOR_EV),
        ("voile_physique", super::VEIL_PHYSICAL),
        ("voile_blanc_de", super::VEIL_WHITE_FROM),
        ("voile_blanc_a", super::VEIL_WHITE_TO),
        ("voile_haut", super::VEIL_UP),
        ("voile_kappa", super::VEIL_KAPPA),
        ("voile_chroma_fond", super::VEIL_CHROMA_ZONE),
        ("voile_teinte", super::VEIL_TINT),
        ("brume_max", super::HAZE_MAX),
        ("brume_courbe", super::HAZE_CURVE),
        ("brume_fond", super::HAZE_FLOOR),
        ("brume_niveau", super::HAZE_LEVEL),
        ("brume_lumiere_min_ev", super::HAZE_LIGHT_MIN_EV),
        ("brume_teinte", super::HAZE_TINT),
        ("brume_chroma", super::HAZE_CHROMA),
        ("k_vibrance", super::K_VIBRANCE),
        ("vibrance_neutre", super::VIBRANCE_NEUTRAL),
        ("vibrance_c_neutre", super::VIBRANCE_C_NEUTRAL),
        ("vibrance_ombre", super::VIBRANCE_SHADOW),
        ("vibrance_l_ombre", super::VIBRANCE_L_SHADOW),
        ("vibrance_peau", super::VIBRANCE_SKIN),
        ("vibrance_peau_largeur", super::VIBRANCE_SKIN_WIDTH),
        ("vibrance_retrait", super::VIBRANCE_PULL),
        ("vibrance_tasse", super::VIBRANCE_PACK),
        ("vibrance_peau_moins", super::VIBRANCE_SKIN_DOWN),
        ("vibrance_c_vive", super::VIBRANCE_C_VIVID),
        ("k_saturation", super::K_SATURATION),
        ("saturation_neutre", super::SATURATION_NEUTRAL),
        ("saturation_c_neutre", super::SATURATION_C_NEUTRAL),
        ("saturation_ombre", super::SATURATION_SHADOW),
        ("saturation_l_ombre", super::SATURATION_L_SHADOW),
        ("saturation_peau", super::SATURATION_SKIN),
        ("densite", super::DENSITY),
    ];

    /// Evora (EVORA-REV-110) : (valeur que la référence écrit hors de `Parametres`, sa valeur) : gris moyen (couleur.py),
    /// plancher de luminance du pont (pont.py), table de gamme (chroma.py), terme de milieu et planchers des sigmas
    /// (evora1.py, voile.py), centile de la teinte du voile (voile.py).
    #[rustfmt::skip]
    const GOLD_OUTSIDE: [(&str, f64); 13] = [
        ("couleur.GRIS", 0.18),
        ("pont.plancher_luminance", 1e-09),
        ("chroma.CHROMA_HAUT", 0.6),
        ("chroma.PAS_RECHERCHE", 16.0),
        ("chroma.TABLE_LUMS", 65.0),
        ("chroma.TABLE_TEINTES", 256.0),
        ("chroma.MARGE_TABLE", 0.985),
        ("evora1.milieu.plancher", 0.35),
        ("evora1.milieu.part", 0.65),
        ("evora1.milieu.largeur_ev", 3.2),
        ("sigma.plancher", 1.0),
        ("sigma.plancher_fin", 0.6),
        ("voile.centile_teinte", 99.0),
    ];

    /// Evora (EVORA-REV-110) : (la même valeur, constante du moteur qui la porte), dans le même ordre.
    #[rustfmt::skip]
    const PORTED_OUTSIDE: [(&str, f64); 13] = [
        ("couleur.GRIS", super::GREY),
        ("pont.plancher_luminance", super::LUMINANCE_FLOOR),
        ("chroma.CHROMA_HAUT", super::CHROMA_TOP),
        ("chroma.PAS_RECHERCHE", super::SEARCH_STEPS as f64),
        ("chroma.TABLE_LUMS", super::TABLE_LUMS as f64),
        ("chroma.TABLE_TEINTES", super::TABLE_HUES as f64),
        ("chroma.MARGE_TABLE", super::TABLE_MARGIN),
        ("evora1.milieu.plancher", super::LOCAL_MIDDLE_FLOOR),
        ("evora1.milieu.part", super::LOCAL_MIDDLE_SHARE),
        ("evora1.milieu.largeur_ev", super::LOCAL_MIDDLE_WIDTH_EV),
        ("sigma.plancher", super::SIGMA_MIN),
        ("sigma.plancher_fin", super::FINE_SIGMA_MIN),
        ("voile.centile_teinte", super::VEIL_TINT_PERCENTILE),
    ];

    /// Evora (EVORA-REV-110) : nœuds de l'échelle de chroma de la référence (`CHROMA_N`, pont.py:41).
    #[rustfmt::skip]
    const GOLD_CHROMA_N: usize = 8;
    /// Evora (EVORA-REV-110) : poids de luminance Rec.2020 de la référence (`LUMA_2020`, couleur.py:12).
    #[rustfmt::skip]
    const GOLD_LUMA_2020: [f64; 3] = [0.2627, 0.678, 0.0593];
    // <<< or:PARAMETRES

    /// Evora (EVORA-REV-110) : chaque constante nommée du rendu d'Evora vaut le réglage de la référence
    /// dont elle porte le nom, à l'égalité stricte (les deux côtés lisent la même écriture décimale).
    /// Avec elles, ce que le pont reprend du moteur au lieu de le recopier : le gris moyen, le nombre
    /// de nœuds de chroma, les poids de luminance, le rayon du médian de la base.
    ///
    /// La référence peut encore bouger : quand un réglage y change, les vecteurs d'or se régénèrent, les
    /// tableaux se reposent, et ce test rougit tant que la constante du moteur n'a pas suivi.
    ///
    /// Sabotages qui le font rougir : une constante retouchée ; un champ ajouté, retiré ou déplacé dans
    /// le relevé sans que la fiche suive ; le gris du moteur ou ses poids de luminance changés.
    #[test]
    fn parameters_are_the_reference_ones() {
        for (&(field, ported), &(gold_field, gold)) in PORTED_PARAMETERS.iter().zip(GOLD_PARAMETERS.iter()) {
            assert_eq!(field, gold_field, "la fiche et le relevé ne nomment pas le même champ de la référence");
            assert!(ported == gold, "`{field}` vaut {ported} dans le moteur, {gold} dans la référence");
        }
        for (&(what, ported), &(gold_what, gold)) in PORTED_OUTSIDE.iter().zip(GOLD_OUTSIDE.iter()) {
            assert_eq!(what, gold_what, "la fiche et le relevé ne nomment pas la même valeur de la référence");
            assert!(ported == gold, "`{what}` vaut {ported} dans le moteur, {gold} dans la référence");
        }
        // ce que le rendu d'Evora reprend du moteur
        assert!(super::GREY as f32 == crate::tone::GREY, "le gris moyen du moteur n'est plus celui de la référence");
        assert!(crate::tone::CHROMA_N == GOLD_CHROMA_N, "la table de tons du moteur n'a plus les {GOLD_CHROMA_N} nœuds de chroma de la référence");
        for (engine, gold) in evolight_color::LUMA_2020.iter().zip(GOLD_LUMA_2020.iter()) {
            assert!(*engine == *gold as f32, "poids de luminance du moteur {engine}, {gold} dans la référence");
        }
        assert!(super::BASE_MEDIAN == crate::local::BASE_DETAIL, "le rayon du médian de la base n'est plus celui du moteur");
    }
}
