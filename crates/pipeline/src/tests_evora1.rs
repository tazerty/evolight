//! Evora (EVORA-REV-98) : la version du rendu d'Evora, `Evora1` (numéro 1001), au niveau du rendu
//! entier. Ce que ces tests gardent :
//!
//! - R12, `evora1_raw_matches_v6` : un brut, avec ou sans courbe d'appareil, se rend sous `Evora1`
//!   exactement comme sous V6 ;
//! - `evora1_hdr_render_matches_v6` : le rendu HDR d'un fichier rendu aussi.
//!
//! Ce sont les deux cas où `Evora1` garde le rendu de V6 pour de bon. Le rendu SDR d'un fichier rendu
//! est le seul que le rendu d'Evora change.
//!
//! Depuis EVORA-REV-112 ce rendu est branché (pont de scène à la fin de l'étage linéaire, table de
//! tons d'Evora, clés de cache marquées), et R12 et son jumeau HDR sont des gardes réelles : ils
//! vérifient d'abord que le rendu SDR d'un fichier rendu, lui, a quitté V6
//! (`assert_sdr_rendered_file_leaves_v6`). Les tests du branchement sont dans le module fils `wired` :
//!
//! - `neutral_rendered_file_is_the_file` : aux réglages neutres, le rendu d'un fichier est ce fichier ;
//! - `render_matches_the_reference_where_the_gamut_return_rests` : exposition, contraste, blancs et
//!   noirs face aux vecteurs d'or de la référence ;
//! - `wired_tone_table_is_increasing_and_bounded` : la table du noyau est celle d'Evora, sous le
//!   prédicat seulement ;
//! - `exposure_creates_no_white_patch`, `chroma_memory_returns_colour_to_highlights` : deux contrôles
//!   de la référence ;
//! - `version_switch_recomputes_the_linear_stage` : un cache de rendu ne mêle pas les deux versions.
//!
//! Depuis EVORA-REV-113 le noyau ramène aussi dans la gamme de sortie une couleur que les tons en ont
//! fait sortir (retour de gamme), avant la borne finale. Ses tests au niveau du rendu entier sont
//! dans le module `wired::pushed` :
//!
//! - `render_matches_the_reference_through_the_gamut_return` : la chaîne par point face aux vecteurs
//!   d'or de la référence, là où son retour de gamme et sa borne finale travaillent ;
//! - `pushed_colours_keep_their_hue` : le contrôle « teinte tenue » de la référence ;
//! - `display_render_runs_the_gamut_return_in_the_display_space` : le rendu pour un écran ;
//! - `neutral_wide_file_keeps_the_engine_gamut_map` : aux réglages neutres, un fichier P3 rendu en
//!   sRGB rend ce que rend V6 (loi du retour de gamme d'EVORA-REV-114).
//!
//! Les rendus de V1 à V6 eux-mêmes sont gardés par R11 (`tests_frozen`).

use evolight_develop::{DevelopSettings, Process};
use evolight_raster::Rgb32f;

use crate::{DeepSamples, OutputDepth, RenderRequest, Rendered, SourceInfo, render};

// Evora (EVORA-REV-112) : les tests du rendu d'Evora branché, dans `tests_evora1/wired.rs`. Module
// fils : il lit les aides et les constantes de ce fichier.
mod wired;

/// Crans de l'exposition des cas, en stops.
const STEPS: [f64; 4] = [-1.0, 0.0, 1.0, 2.0];
/// Côté de la boîte de rendu des cas « photo neuve » (la scène de démonstration y est réduite).
const PLAIN_BOX: usize = 96;
/// Le plus petit bord long de sortie, en points, où le médian de la base des tons clairs et sombres
/// a un rayon non nul. `local` prend 0,0015 point de rayon par point de bord long, arrondi : 0,4995 à
/// 333 points (rayon 0), 0,501 à 334 (rayon 1). Sous ce bord, le rendu ne dépend pas de l'aiguillage
/// de ce rayon par version, et aucune égalité de rendus ne le voit. R12 vérifie cette constante sur
/// le code de `local`.
const SMALLEST_MEDIAN_EDGE: usize = 334;
/// La boîte des cas « retouche », au-dessus de `SMALLEST_MEDIAN_EDGE`, et leur source (scène de
/// démonstration des tests du pipeline), de même bord long : la source y est rendue sans réduction, à
/// l'échelle du point d'origine. C'est la boîte du jeu « grand » des empreintes figées. À 480 points,
/// la base des tons clairs et sombres a un sigma de 7,2 points, franchement dans le pas 2 du filtre
/// guidé (`local::guided_fast_step`) ; à 400 points, ce sigma tombait pile sur le seuil de ce pas
/// (0,015 × 400 = 6,0) et la branche confrontée tenait à un arrondi.
const RETOUCH_BOX: usize = 480;
const RETOUCH_SOURCE: (usize, usize) = (480, 320);
const _: () = assert!(RETOUCH_BOX >= SMALLEST_MEDIAN_EDGE && RETOUCH_SOURCE.0 >= RETOUCH_BOX, "boîte de la retouche trop petite");
/// Le pas du filtre guidé de la base des tons clairs et sombres à `RETOUCH_BOX` points.
const RETOUCH_BASE_STEP: usize = 2;
/// Netteté et réduction du bruit de couleur des cas « retouche » : les valeurs qu'un brut neuf reçoit
/// (`DevelopSettings::for_raw`). Non nulles, elles font entrer dans le rendu les deux lectures de la
/// version qui se font par ordre (`local::pixel_sharpen`, `local::joint_colour_nr`).
const RETOUCH_SHARPEN: f64 = 40.0;
const RETOUCH_COLOUR_NR: f64 = 25.0;
/// Marge du rendu HDR des cas de `evora1_hdr_render_matches_v6`, en stops au-dessus du blanc SDR :
/// celle d'une photo dont on vient d'allumer le rendu HDR.
const HDR_HEADROOM: f64 = evolight_develop::Hdr::DEFAULT_MAX_EV;

/// Une courbe d'appareil de test, avec sa courbe de chroma (le chemin des bruts qui l'ont).
fn camera_curve() -> crate::tone::CameraTone {
    let knots = std::array::from_fn(|i| {
        let x = 0.004 * 1.18f32.powi(i as i32);
        [x, 1.0 - (-2.0 * x).exp()]
    });
    let Some(curve) = crate::tone::CameraTone::new(knots).and_then(|c| c.with_chroma([1.3, 1.2, 1.1, 1.0, 0.9, 0.7, 0.5, 0.4])) else {
        panic!("courbe d'appareil de test refusée");
    };
    curve
}

/// Un brut sans courbe d'appareil, puis un brut avec.
fn raw_sources() -> [SourceInfo; 2] {
    let raw = SourceInfo { raw: true, as_shot_temp: 5200.0, as_shot_tint: 4.0, ..Default::default() };
    [raw.clone(), SourceInfo { camera_tone: Some(camera_curve()), ..raw }]
}

/// Les réglages par défaut sous `process`, au cran `step` (en stops) : le cas « photo neuve ». Pour un
/// brut, ce ne sont pas tout à fait ceux d'un brut neuf, qui reçoit en plus une netteté et une
/// réduction du bruit de couleur (`DevelopSettings::for_raw`) : la retouche les porte (`retouched`).
fn at(process: Process, step: f64) -> DevelopSettings {
    let mut s = DevelopSettings { process: process.version(), ..Default::default() };
    s.light.exposure = step;
    s
}

/// La retouche des cas, qui fait entrer dans le rendu les cinq lectures de la version qui pèsent sur
/// un point :
/// - contraste, blancs et noirs, que porte la table de tons (`finish::base_tone`) et, pour un brut à
///   courbe d'appareil, les opérateurs de contraste et de noirs de `light_v3` ;
/// - tons clairs et sombres, dont la base passe par le médian de `local` au-dessus de
///   `SMALLEST_MEDIAN_EDGE` ;
/// - netteté, qui passe par le plan à l'échelle du point à partir de V3 (`local::pixel_sharpen`) ;
/// - réduction du bruit de couleur, conjointe à partir de V6 (`local::joint_colour_nr`).
///
/// Les deux dernières lisent la version par ordre (`>= V3`, `>= V6`) : elles ne valent pour `Evora1`
/// que parce qu'elle est déclarée après V6. Sans netteté ni réduction du bruit de couleur, aucune des
/// deux ne pèse sur un point, et l'égalité des rendus ne les verrait pas. Les clés des plans
/// (`local::base_key`, `local::sharpen_key`) nomment aussi la version, sans peser sur un point.
fn retouched(process: Process, step: f64) -> DevelopSettings {
    let mut s = at(process, step);
    let l = &mut s.light;
    (l.contrast, l.highlights, l.shadows, l.whites, l.blacks) = (30.0, -60.0, 40.0, 15.0, -20.0);
    (s.detail.sharpen_amount, s.detail.nr_color) = (RETOUCH_SHARPEN, RETOUCH_COLOUR_NR);
    s
}

/// Les réglages `s`, rendu HDR allumé avec `HDR_HEADROOM` stops de marge.
fn hdr_on(mut s: DevelopSettings) -> DevelopSettings {
    s.hdr.enabled = true;
    s.hdr.max_ev = HDR_HEADROOM;
    s
}

/// Les deux sorties SDR des cas : 8 bits, et 16 bits.
fn requests(w: usize, h: usize) -> [RenderRequest; 2] {
    [RenderRequest::fit(w, h), RenderRequest { depth: OutputDepth::U16, ..RenderRequest::fit(w, h) }]
}

/// La sortie HDR des cas : flottants linéaires, blanc SDR à 1.
fn hdr_request(w: usize, h: usize) -> RenderRequest {
    RenderRequest { depth: OutputDepth::F32Hdr, ..RenderRequest::fit(w, h) }
}

/// Les octets d'un rendu SDR : l'image 8 bits, et les échantillons 16 bits s'il y en a.
fn bits(r: Rendered) -> (Vec<[u8; 4]>, Vec<u16>) {
    let deep = match r.deep {
        Some(d) => match d.samples {
            DeepSamples::U16(v) => v,
            DeepSamples::F32(_) => panic!("échantillons 16 bits attendus"),
        },
        None => Vec::new(),
    };
    (r.image.data, deep)
}

/// Les octets d'un rendu HDR : l'image 8 bits, et les échantillons flottants lus bit pour bit (deux
/// zéros de signes opposés, ou deux valeurs qui ne sont pas des nombres, n'y passent pas pour égaux
/// ou différents à tort).
fn hdr_bits(r: Rendered) -> (Vec<[u8; 4]>, Vec<u32>) {
    let Some(deep) = r.deep else {
        panic!("rendu HDR sans échantillons flottants");
    };
    let DeepSamples::F32(v) = deep.samples else {
        panic!("échantillons flottants attendus");
    };
    let mut out = Vec::with_capacity(v.len());
    for x in &v {
        out.push(x.to_bits());
    }
    (r.image.data, out)
}

/// La source des cas « photo neuve » : la scène de démonstration des empreintes figées.
fn plain_scene() -> Rgb32f {
    crate::tests_frozen::frozen_scene()
}

/// La source des cas « retouche », du bord long de `RETOUCH_BOX`.
fn retouch_scene() -> Rgb32f {
    evolight_scenes::demo_library()[0].render(RETOUCH_SOURCE.0, RETOUCH_SOURCE.1)
}

/// Garde de portée des cas « retouche ». Sous `process`, `retouched` rendu à `RETOUCH_BOX` points de
/// bord long atteint bien chaque lecture de la version :
/// - un médian de rayon non nul sous la base des tons clairs et sombres (`local::plane_sigmas`), et le
///   pas `RETOUCH_BASE_STEP` de son filtre guidé ;
/// - pour un brut à courbe d'appareil seulement, les opérateurs de `light_v3` ;
/// - la netteté par le plan à l'échelle du point, demandé (`local::pixel_sharpen`) ;
/// - la réduction conjointe du bruit de couleur, avec ses paramètres (`local::joint_colour_nr`,
///   `local::nr_params`, `local::nrc_params`).
///
/// Sans elle, une matière qui n'atteindrait plus ces aiguillages laisserait l'égalité des rendus verte
/// sans rien confronter. Elle rougit aussi d'elle-même si `Evora1` quitte sa place dans l'ordre des
/// versions (déclarée avant V6, elle perd la réduction conjointe ; avant V3, le plan de netteté).
fn assert_retouch_reaches_the_switches(process: Process, info: &SourceInfo) {
    let s = retouched(process, 1.0);
    let sigmas = crate::local::plane_sigmas(&s, RETOUCH_BOX as f64, RETOUCH_SOURCE.0 as f64, crate::Quality::Full);
    let median = sigmas.base_detail;
    assert!(median >= 1, "{process:?} : rayon du médian {median} à {RETOUCH_BOX} points, la retouche ne l'atteint pas");
    let step = sigmas.base.map(crate::local::guided_fast_step);
    assert_eq!(step, Some(RETOUCH_BASE_STEP), "{process:?} : pas du filtre guidé de la base à {RETOUCH_BOX} points");
    let operators = crate::light_v3::LightOps::new(&s, info).is_some();
    let camera = info.raw && info.camera_tone.is_some();
    assert_eq!(operators, camera, "{process:?}, brut {}, courbe {} : opérateurs de light_v3 {operators}", info.raw, info.camera_tone.is_some());
    // les deux lectures par ordre : netteté (à partir de V3), réduction conjointe du bruit de couleur (à partir de V6)
    assert!(crate::local::pixel_sharpen(&s), "{process:?} : la netteté ne passe pas par le plan à l'échelle du point");
    assert!(sigmas.sharpen.is_some(), "{process:?} : plan de netteté non demandé à {RETOUCH_BOX} points, la retouche ne l'atteint pas");
    assert!(crate::local::joint_colour_nr(&s), "{process:?} : la réduction du bruit de couleur n'est pas la réduction conjointe");
    let scale = crate::local::detail_scale(RETOUCH_BOX as f64, RETOUCH_SOURCE.0);
    let (_, colour) = crate::local::nr_params(&s, scale);
    assert!(colour.is_some(), "{process:?} : réduction du bruit de couleur éteinte à l'échelle {scale}, la retouche ne l'atteint pas");
    assert!(crate::local::nrc_params(&s, scale).is_some(), "{process:?} : réduction conjointe sans paramètres à l'échelle {scale}");
}

/// Un rendu de `retouch_scene` dans `RETOUCH_BOX`, son bord long vérifié.
fn retouch_render(src: &Rgb32f, info: &SourceInfo, s: &DevelopSettings, req: &RenderRequest) -> Rendered {
    let r = render(src, info, s, req);
    assert_eq!(r.image.width.max(r.image.height), RETOUCH_BOX, "bord long du rendu de la retouche");
    r
}

/// Les cas SDR d'une source : la photo neuve à `PLAIN_BOX` points et la retouche à `RETOUCH_BOX`
/// points, en 8 et en 16 bits, aux crans de `STEPS`, rendus sous V6 et sous `Evora1` : égaux au bit
/// près. `what` nomme la source dans les messages.
fn assert_sdr_matches_v6(plain: &Rgb32f, large: &Rgb32f, info: &SourceInfo, what: &str) {
    for process in [Process::V6, Process::Evora1] {
        assert_retouch_reaches_the_switches(process, info);
    }
    for step in STEPS {
        for req in requests(PLAIN_BOX, PLAIN_BOX) {
            let v6 = bits(render(plain, info, &at(Process::V6, step), &req));
            let evora = bits(render(plain, info, &at(Process::Evora1, step), &req));
            assert!(v6 == evora, "{what}, cran {step}, {:?} : Evora1 diffère de V6", req.depth);
        }
        for req in requests(RETOUCH_BOX, RETOUCH_BOX) {
            let v6 = bits(retouch_render(large, info, &retouched(Process::V6, step), &req));
            let evora = bits(retouch_render(large, info, &retouched(Process::Evora1, step), &req));
            assert!(v6 == evora, "{what} retouché, cran {step}, {:?} : Evora1 diffère de V6", req.depth);
        }
    }
}

/// Garde de portée de R12 et de son jumeau HDR (EVORA-REV-112). Le rendu d'Evora est branché : le
/// rendu SDR d'un fichier rendu, retouché, n'est plus celui de V6, en 8 comme en 16 bits. Sans cette
/// garde, l'égalité exigée d'un brut et d'un rendu HDR ne dirait rien du prédicat : elle tiendrait
/// aussi avec un rendu d'Evora jamais branché.
///
/// Elle remplace `assert_unwired_rendered_file_matches_v6` d'EVORA-REV-98, qui affirmait l'inverse
/// tant que chaque bras d'`Evora1` recopiait celui de V6.
fn assert_sdr_rendered_file_leaves_v6(large: &Rgb32f) {
    let info = SourceInfo::default();
    assert!(!info.raw, "la source de cette garde est un fichier rendu");
    for req in requests(RETOUCH_BOX, RETOUCH_BOX) {
        let v6 = bits(retouch_render(large, &info, &retouched(Process::V6, 1.0), &req));
        let evora = bits(retouch_render(large, &info, &retouched(Process::Evora1, 1.0), &req));
        assert!(v6 != evora, "fichier rendu retouché, {:?} : Evora1 rend encore comme V6, le rendu d'Evora n'est pas branché", req.depth);
    }
}

/// R12 (Evora, EVORA-REV-98) : un brut, sans puis avec courbe d'appareil, rendu sous `Evora1` et sous
/// V6, en 8 et en 16 bits, aux crans −1 à +2 : égal au bit près. Deux cas :
/// - la photo neuve (réglages par défaut), à `PLAIN_BOX` points, qui confronte les bras des bruts de
///   la table de tons ;
/// - la retouche (`retouched`), à `RETOUCH_BOX` points, qui fait entrer dans le rendu les quatre
///   autres lectures de la version : les opérateurs de contraste et de noirs de `light_v3` (brut à
///   courbe d'appareil), le rayon du médian de la base des tons clairs et sombres de `local`, et les
///   deux qui se font par ordre, la netteté par le plan à l'échelle du point (`>= V3`) et la réduction
///   conjointe du bruit de couleur (`>= V6`). À `PLAIN_BOX` points, sans ces curseurs, aucune n'agit.
///   Une garde de portée vérifie d'abord, sous V6 comme sous `Evora1`, que la retouche les atteint
///   (`assert_retouch_reaches_the_switches`).
///
/// Sabotages qui le font rougir : un bras des bruts d'`Evora1` pris sur V4, ou qui laisse tomber les
/// blancs ; les opérateurs de `light_v3` éteints sous `Evora1` ; le rayon du médian nul sous `Evora1` ;
/// `Evora1` déclarée avant V6 dans la liste des versions, `Process::ALL` inchangé (elle perd la
/// réduction conjointe du bruit de couleur). La garde de portée rougit sur ces trois derniers, avant
/// toute comparaison.
///
/// Depuis EVORA-REV-112, le rendu d'Evora est branché et R12 garde pour de bon la condition « pas un
/// brut » du prédicat. Sabotages de plus : le prédicat ouvert aux bruts (un brut passerait le pont de
/// scène, ou lirait la table de tons d'Evora) ; le pont posé à l'étage linéaire sans lire le plan.
/// Une seconde garde de portée vérifie que le fichier rendu, lui, a bien quitté V6
/// (`assert_sdr_rendered_file_leaves_v6`).
#[test]
fn evora1_raw_matches_v6() {
    assert_eq!(crate::local::base_detail_radius((SMALLEST_MEDIAN_EDGE - 1) as f64), 0, "médian sous le plus petit bord");
    assert_eq!(crate::local::base_detail_radius(SMALLEST_MEDIAN_EDGE as f64), 1, "médian au plus petit bord");
    let (plain, large) = (plain_scene(), retouch_scene());
    for info in raw_sources() {
        let what = if info.camera_tone.is_some() { "brut à courbe d'appareil" } else { "brut" };
        assert_sdr_matches_v6(&plain, &large, &info, what);
    }
    assert_sdr_rendered_file_leaves_v6(&large);
}

/// Evora (EVORA-REV-98) : le rendu HDR d'un fichier rendu (rendu HDR allumé, sortie en flottants)
/// sous `Evora1` et sous V6, photo neuve et retouche, aux crans −1 à +2 : égal bit pour bit,
/// échantillons flottants compris. Le rendu d'Evora ne s'applique qu'au rendu SDR d'un fichier
/// rendu : ce test garde la condition « pas un rendu HDR », que R12 ne voit pas (il garde la
/// condition « pas un brut »).
///
/// Une garde de portée vérifie d'abord que les cas sont bien des rendus HDR : les réglages que le
/// rendu choisit pour la sortie en flottants gardent une marge au-dessus du blanc SDR, alors que
/// ceux d'une sortie 8 bits de la même édition n'en ont pas.
/// Sabotages qui le font rougir : le rendu d'Evora appliqué à un fichier rendu sans regarder la
/// marge HDR ; un bras d'`Evora1` de la table de tons qui laisse tomber la marge.
#[test]
fn evora1_hdr_render_matches_v6() {
    let info = SourceInfo::default();
    assert!(!info.raw, "la source de ce test est un fichier rendu");
    let (plain, large) = (plain_scene(), retouch_scene());
    for process in [Process::V6, Process::Evora1] {
        assert_retouch_reaches_the_switches(process, &info);
        let s = hdr_on(retouched(process, 1.0));
        let hdr = crate::settings_for(&s, &hdr_request(RETOUCH_BOX, RETOUCH_BOX));
        assert!(hdr.hdr.peak() > 1.0, "{process:?} : la sortie en flottants n'est pas un rendu HDR");
        let sdr = crate::settings_for(&s, &RenderRequest::fit(RETOUCH_BOX, RETOUCH_BOX));
        assert!(sdr.hdr.peak() <= 1.0, "{process:?} : la sortie 8 bits d'une édition HDR est un rendu SDR");
    }
    // Evora (EVORA-REV-112) : le rendu SDR du même fichier, lui, a quitté V6 : l'égalité qui suit
    // tient à la condition « pas un rendu HDR » du prédicat
    assert_sdr_rendered_file_leaves_v6(&large);
    for step in STEPS {
        let req = hdr_request(PLAIN_BOX, PLAIN_BOX);
        let v6 = hdr_bits(render(&plain, &info, &hdr_on(at(Process::V6, step)), &req));
        let evora = hdr_bits(render(&plain, &info, &hdr_on(at(Process::Evora1, step)), &req));
        assert!(v6 == evora, "fichier rendu en HDR, cran {step} : Evora1 diffère de V6");
        let req = hdr_request(RETOUCH_BOX, RETOUCH_BOX);
        let v6 = hdr_bits(retouch_render(&large, &info, &hdr_on(retouched(Process::V6, step)), &req));
        let evora = hdr_bits(retouch_render(&large, &info, &hdr_on(retouched(Process::Evora1, step)), &req));
        assert!(v6 == evora, "fichier rendu retouché en HDR, cran {step} : Evora1 diffère de V6");
    }
}
