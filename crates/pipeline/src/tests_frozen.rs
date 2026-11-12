//! Evora (lot 1 « REV-TONS », P0, élargi par EVORA-REV-95) : empreintes figées des rendus des versions
//! V1 à V6, recette « éditions figées » côté Rust.
//!
//! `print_frozen_render_fingerprints`, ignoré par défaut, imprime une empreinte par cas : chaque
//! version de V1 à V6, les crans −1, 0, +1 et +2, et dix-huit couples de source et de jeu de réglages.
//! Un fichier rendu et un brut de test portent chacun huit jeux (« simple », « retouche », « vigueur »,
//! « douceur », « melange », « grand », « fenetre », « cache ») ; un brut à courbe d'appareil en porte
//! deux (« simple », « retouche »). Soit 18 × 6 × 4 = 432 lignes. L'étape « Empreintes figées » de la
//! CI d'Evora le joue sur l'arbre d'avant la version du rendu d'Evora et dépose ses lignes dans le
//! résumé et les diagnostics. Recopiées telles quelles dans `frozen_renders.txt` (EVORA-REV-96), elles
//! sont les constantes du test qui garde ces rendus figés : R11, `frozen_renders_v1_to_v6`.
//!
//! Ce que chaque jeu ajouté fige :
//! - « melange » : le mélangeur de couleur avec vibrance et saturation, et un masque visible à
//!   réglages locaux ;
//! - « grand » : une sortie de 480 points, où la base des tons clairs et sombres passe par son filtre
//!   sous-échantillonné et par son médian (rayon nul à 120 points) ;
//! - « fenetre » : le rendu d'une fenêtre de cette sortie, avec l'atténuation de la brume, dont le
//!   niveau d'atmosphère vient alors du cadre entier ;
//! - « cache » : le brouillon rendu après un changement de réduction du bruit, qui reprend les plans
//!   larges gardés par le cache d'étapes ; puis, sur le même cache, un rendu plein aux réglages
//!   d'origine, qui ne doit reprendre aucun des plans de ce brouillon ;
//! - la source « courbe » : les bras à courbe d'appareil de la table de tons, et de V4 à V6 les
//!   opérateurs de contraste et de noirs qui la suivent.
//!
//! Les rendus appellent `powf`, `exp2` et `log2` de la plateforme, dont le dernier bit peut changer
//! d'une plateforme à l'autre (même raison que le test d'or de V1 et V2 de l'amont) : une empreinte
//! ne vaut que pour la plateforme où elle a été relevée. La courbe d'appareil de la source « courbe »
//! est calculée ici avec `powi` et `exp` : même réserve pour ses 48 lignes.

use std::sync::Arc;

use evolight_develop::{DevelopSettings, LocalAdjustments, Mask, MaskComponent, MaskOp, MaskShape, Process};
use evolight_geom::Point;
use evolight_raster::Rgb32f;

use crate::tone::CameraTone;
use crate::{DeepSamples, OutputDepth, PixelWindow, Quality, RenderRequest, Rendered, SourceInfo, StageCache, render, render_cached};

/// Les versions figées : toutes celles de l'amont connues au moment du relevé.
pub(crate) const FROZEN_VERSIONS: [Process; 6] = [Process::V1, Process::V2, Process::V3, Process::V4, Process::V5, Process::V6];
/// Crans de la recette, en stops.
pub(crate) const FROZEN_STEPS: [f64; 4] = [-1.0, 0.0, 1.0, 2.0];
/// Sources de la recette, dans l'ordre où elles s'impriment : un fichier rendu, un brut de test, un
/// brut à courbe d'appareil. Chaque nom tient en un mot (une ligne d'empreinte compte six mots).
const FROZEN_SOURCES: [&str; 3] = ["rendu", "brut", "courbe"];
/// Jeux de réglages du fichier rendu et du brut de test, dans l'ordre où ils s'impriment. Un mot
/// chacun, sans accent.
const FROZEN_VARIANTS: &[&str] = &["simple", "retouche", "vigueur", "douceur", "melange", "grand", "fenetre", "cache"];
/// Jeux de réglages du brut à courbe d'appareil.
const FROZEN_CURVE_VARIANTS: &[&str] = &["simple", "retouche"];
/// Nombre de lignes d'empreintes : 18 couples (source, jeu) × 6 versions × 4 crans.
const FROZEN_TOTAL: usize = 432;
/// Côté de la boîte de rendu des jeux à 120 points (la scène de démonstration y est réduite de moitié).
const FROZEN_SIDE: usize = 120;
/// Côté de la boîte de rendu des jeux « grand » et « fenetre » (la grande scène y garde sa taille).
const FROZEN_LARGE_SIDE: usize = 480;
/// La fenêtre du jeu « fenetre », en points de la sortie de 480 × 320.
const FROZEN_WINDOW: PixelWindow = PixelWindow { x: 150, y: 90, w: 200, h: 140 };
/// Réduction du bruit de luminance posée entre le rendu plein et le brouillon du jeu « cache ».
const FROZEN_CACHE_NR: f64 = 50.0;
/// Préfixe des lignes d'empreintes, lu par la CI et par le fichier des constantes.
pub(crate) const LINE_PREFIX: &str = "frozen";
/// Préfixe de la ligne de provenance du fichier des constantes : le commit de l'arbre d'avant la
/// version du rendu d'Evora sur lequel la CI a relevé les empreintes. R11 l'exige ; la porte
/// « tests Rust » (EVORA-REV-97) vérifie qu'il est un ancêtre de la tête, que son arbre ne connaît
/// pas cette version et que la recette y annonce autant de cas que le fichier a d'empreintes. Elle
/// vérifie aussi que le dernier commit à avoir touché le fichier ne connaît pas cette version.
const COMMIT_PREFIX: &str = "frozen-commit";
/// La variable que la porte « tests Rust » pose à 1 ; un écart de plateforme y est un échec de R11,
/// et non une impression seule. Elle ne se pose qu'à 1 : toute autre valeur fait échouer R11.
const STRICT_VAR: &str = "EVORA_FROZEN_STRICT";

/// Un cas de la recette.
pub(crate) struct FrozenCase {
    pub version: Process,
    pub source: &'static str,
    pub variant: &'static str,
    pub step: f64,
}

impl FrozenCase {
    /// La clé du cas, telle qu'elle s'imprime : `v1 rendu simple -1.0`.
    pub(crate) fn key(&self) -> String {
        format!("v{} {} {} {:+.1}", self.version.version().0, self.source, self.variant, self.step)
    }
}

/// Les jeux de réglages d'une source.
fn variants_of(source: &str) -> &'static [&'static str] {
    if source == "courbe" { FROZEN_CURVE_VARIANTS } else { FROZEN_VARIANTS }
}

/// Tous les cas, dans l'ordre où ils s'impriment.
pub(crate) fn frozen_cases() -> Vec<FrozenCase> {
    let mut out = Vec::new();
    for version in FROZEN_VERSIONS {
        for source in FROZEN_SOURCES {
            for &variant in variants_of(source) {
                for step in FROZEN_STEPS {
                    out.push(FrozenCase { version, source, variant, step });
                }
            }
        }
    }
    out
}

/// La scène de démonstration des tests du pipeline, en lumière de scène linéaire.
pub(crate) fn frozen_scene() -> Rgb32f {
    evolight_scenes::demo_library()[0].render(240, 160)
}

/// La même scène à 480 × 320 points, pour les jeux « grand » et « fenetre ».
fn frozen_large_scene() -> Rgb32f {
    evolight_scenes::demo_library()[0].render(480, 320)
}

/// Les jeux rendus depuis la grande scène, dans la boîte de `FROZEN_LARGE_SIDE` points.
fn is_large(variant: &str) -> bool {
    matches!(variant, "grand" | "fenetre")
}

/// Les deux scènes de la recette, rendues une fois pour tous les cas.
pub(crate) struct FrozenScenes {
    small: Arc<Rgb32f>,
    large: Arc<Rgb32f>,
}

impl FrozenScenes {
    pub(crate) fn new() -> Self {
        Self { small: Arc::new(frozen_scene()), large: Arc::new(frozen_large_scene()) }
    }

    /// La scène d'un cas : la grande pour « grand » et « fenetre », celle de 240 × 160 points sinon.
    fn of(&self, case: &FrozenCase) -> &Arc<Rgb32f> {
        if is_large(case.variant) { &self.large } else { &self.small }
    }
}

/// La courbe d'appareil du brut « courbe » : celle du modèle de
/// `tests::stored_process_numbers_render_with_a_known_process`. Ses nœuds se calculent avec `powi` et
/// `exp`, comme dans le modèle : les écrire autrement donnerait une autre courbe (voir l'en-tête).
fn camera_curve() -> CameraTone {
    let Some(curve) = CameraTone::new(std::array::from_fn(|i| {
        let x = 0.004 * 1.18f32.powi(i as i32);
        [x, 1.0 - (-2.0 * x).exp()]
    })) else {
        panic!("courbe d'appareil du modèle refusée");
    };
    curve
}

fn source_info(source: &str) -> SourceInfo {
    match source {
        "brut" => SourceInfo { raw: true, as_shot_temp: 5200.0, as_shot_tint: 4.0, ..Default::default() },
        "courbe" => SourceInfo { raw: true, relative_wb: true, camera_tone: Some(camera_curve()), ..Default::default() },
        _ => SourceInfo::default(),
    }
}

/// La demande de rendu 8 bits d'un cas : la boîte de son jeu, et pour « fenetre » la fenêtre.
fn frozen_request(case: &FrozenCase) -> RenderRequest {
    match case.variant {
        "grand" => RenderRequest::fit(FROZEN_LARGE_SIDE, FROZEN_LARGE_SIDE),
        "fenetre" => RenderRequest { window: Some(FROZEN_WINDOW), ..RenderRequest::fit(FROZEN_LARGE_SIDE, FROZEN_LARGE_SIDE) },
        _ => RenderRequest::fit(FROZEN_SIDE, FROZEN_SIDE),
    }
}

/// Les réglages d'un cas : ceux d'une photo neuve de ce type, la version du cas, le cran, et pour
/// « retouche » quatre curseurs de la lumière et un mélange de couleur. Les autres jeux sont décrits
/// à leur bloc.
fn settings(case: &FrozenCase) -> DevelopSettings {
    let mut s = match case.source {
        "brut" => DevelopSettings::for_raw(5200.0, 4.0),
        // le blanc de prise de vue du brut « courbe » est celui de `SourceInfo::default`
        "courbe" => DevelopSettings::for_raw(6500.0, 0.0),
        _ => DevelopSettings::default(),
    };
    s.process = case.version.version();
    s.light.exposure = case.step;
    if case.variant == "retouche" {
        let l = &mut s.light;
        (l.contrast, l.highlights, l.whites, l.blacks) = (30.0, -60.0, 15.0, -20.0);
        s.mixer.blue.sat = -30.0;
        s.mixer.orange.lum = 15.0;
    }
    // Les curseurs que le rendu d'Evora reprend ensuite (tons sombres, relief fin, contraste local,
    // atténuation de la brume, vibrance, saturation), montés puis baissés : leurs rendus de V1 à V6
    // restent figés eux aussi.
    if case.variant == "vigueur" {
        s.light.shadows = 40.0;
        (s.effects.texture, s.effects.clarity, s.effects.dehaze) = (40.0, 50.0, 30.0);
        (s.color.vibrance, s.color.saturation) = (40.0, 20.0);
    }
    if case.variant == "douceur" {
        s.light.shadows = -30.0;
        (s.effects.texture, s.effects.clarity, s.effects.dehaze) = (-30.0, -40.0, -35.0);
        (s.color.vibrance, s.color.saturation) = (-30.0, -20.0);
    }
    // Le mélangeur actif (teinte, saturation, luminance) avec vibrance et saturation : le chemin long
    // des outils de couleur. Et un masque visible dont les réglages locaux touchent les tons clairs,
    // le relief fin, le contraste local, l'atténuation de la brume et la saturation.
    if case.variant == "melange" {
        s.mixer.red.hue = 20.0;
        s.mixer.orange.sat = 25.0;
        s.mixer.green.sat = -35.0;
        s.mixer.blue.lum = -20.0;
        (s.color.vibrance, s.color.saturation) = (35.0, -15.0);
        s.masks.push(Mask {
            id: 1,
            components: vec![MaskComponent {
                name: None,
                op: MaskOp::Add,
                invert: false,
                shape: MaskShape::Radial { center: Point::new(0.5, 0.5), rx: 0.3, ry: 0.2, angle: 20.0, feather: 50.0, invert: false },
            }],
            adjust: LocalAdjustments { highlights: -40.0, texture: 30.0, clarity: 25.0, dehaze: 20.0, saturation: 30.0, ..Default::default() },
            ..Default::default()
        });
    }
    // À 480 points, la base des tons clairs et sombres a un sigma de 7,2 points (filtre guidé au pas
    // de 2) et, depuis V2, un médian de rayon 1 ; le contraste local lit cette base.
    if case.variant == "grand" {
        (s.light.highlights, s.light.shadows) = (-50.0, 40.0);
        s.effects.clarity = 30.0;
    }
    // L'atténuation de la brume d'une fenêtre prend son niveau d'atmosphère sur le cadre entier.
    if case.variant == "fenetre" {
        s.light.highlights = -50.0;
        s.effects.dehaze = 30.0;
    }
    // Les plans larges que le brouillon du jeu « cache » reprend : base des tons, contraste local,
    // atténuation de la brume.
    if case.variant == "cache" {
        s.light.highlights = -40.0;
        (s.effects.clarity, s.effects.dehaze) = (40.0, 20.0);
    }
    s
}

/// FNV-1a 64 bits, comme le test d'or de l'amont.
fn fnv(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for b in bytes {
        *hash = (*hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3);
    }
}

/// Les rendus d'un cas pour une demande, dans l'ordre où ils se hachent : un seul, sans cache, sauf
/// pour le jeu « cache », qui en donne deux. Ce jeu passe par un cache d'étapes neuf : un rendu plein,
/// qui n'est pas gardé ; puis, la réduction du bruit de luminance changée, un brouillon de la même
/// boîte, qui garde les plans larges du rendu plein ; puis un rendu plein aux réglages d'origine, à
/// qui la clé des plans laissés par le brouillon ne correspond pas et qui les refait tous. La
/// fonction donne le brouillon, puis ce dernier rendu.
fn rendered(src: &Arc<Rgb32f>, info: &SourceInfo, s: &DevelopSettings, req: &RenderRequest, case: &FrozenCase) -> Vec<Rendered> {
    if case.variant != "cache" {
        return vec![render(src, info, s, req)];
    }
    let cache = StageCache::default();
    render_cached(src, info, s, req, &cache);
    let mut after = s.clone();
    after.detail.nr_luminance = FROZEN_CACHE_NR;
    let draft_req = RenderRequest { quality: Quality::Draft, ..*req };
    let draft = render_cached(src, info, &after, &draft_req, &cache);
    let full = render_cached(src, info, s, req, &cache);
    vec![draft, full]
}

/// L'empreinte d'un cas : les octets de ses rendus 8 bits, puis les échantillons de ses rendus
/// 16 bits (un rendu par profondeur, deux pour le jeu « cache »).
pub(crate) fn fingerprint(scenes: &FrozenScenes, case: &FrozenCase) -> u64 {
    let src = scenes.of(case);
    let info = source_info(case.source);
    let s = settings(case);
    let mut h = 0xCBF2_9CE4_8422_2325_u64;
    for eight in rendered(src, &info, &s, &frozen_request(case), case) {
        fnv(&mut h, eight.image.data.iter().flatten().copied());
    }
    let req = RenderRequest { depth: OutputDepth::U16, ..frozen_request(case) };
    for sixteen in rendered(src, &info, &s, &req, case) {
        let Some(deep) = sixteen.deep else {
            panic!("{} : rendu 16 bits absent", case.key());
        };
        let DeepSamples::U16(v) = deep.samples else {
            panic!("{} : échantillons 16 bits attendus", case.key());
        };
        fnv(&mut h, v.iter().flat_map(|x| x.to_le_bytes()));
    }
    h
}

/// Les lignes d'empreintes de tous les cas : `frozen <clé> <empreinte>`.
pub(crate) fn frozen_lines() -> Vec<String> {
    let scenes = FrozenScenes::new();
    let mut out = Vec::new();
    for case in frozen_cases() {
        out.push(format!("{LINE_PREFIX} {} {:#018x}", case.key(), fingerprint(&scenes, &case)));
    }
    out
}

/// P0 : imprime les empreintes de V1 à V6, la plateforme et leur nombre (432). À jouer par l'étape
/// « Empreintes figées » de la CI d'Evora :
/// `cargo test -p evolight-pipeline --lib -- --ignored --exact tests_frozen::print_frozen_render_fingerprints --nocapture`.
/// Après l'impression, il vérifie seulement la forme : six mots par ligne, et le compte.
#[test]
#[ignore = "P0 : impression des empreintes figées, jouée par la CI d'Evora"]
fn print_frozen_render_fingerprints() {
    let lines = frozen_lines();
    println!();
    println!("{LINE_PREFIX}-platform {} {}", std::env::consts::OS, std::env::consts::ARCH);
    for line in &lines {
        println!("{line}");
    }
    println!("{LINE_PREFIX}-total {}", lines.len());
    for line in &lines {
        assert_eq!(line.split_whitespace().count(), 6, "six mots attendus par ligne d'empreinte : « {line} »");
    }
    assert_eq!(lines.len(), FROZEN_TOTAL, "nombre de lignes d'empreintes");
}

/// Le fichier des constantes de R11 (voir son en-tête).
const FROZEN_FILE: &str = include_str!("frozen_renders.txt");

/// Le contenu du fichier des constantes : plateforme du relevé, commit du relevé, empreintes.
struct FrozenFile {
    platform: Option<String>,
    commit: Option<String>,
    lines: Vec<(String, String)>,
}

/// La plateforme (`frozen-platform <système> <architecture>`, une seule fois), le commit du relevé
/// (`frozen-commit <sha>`, une seule fois) et les empreintes (`frozen v…`, clé puis empreinte) d'un
/// texte au format des lignes imprimées ; les lignes `#` et vides sont ignorées, toute autre ligne
/// fait échouer R11.
fn parse_frozen(text: &str) -> FrozenFile {
    let mut platform = None;
    let mut commit = None;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.first() == Some(&"frozen-platform") && words.len() == 3 {
            assert!(platform.is_none(), "R11 : deux lignes « frozen-platform » dans frozen_renders.txt");
            platform = Some(words[1..].join(" "));
        } else if words.first() == Some(&COMMIT_PREFIX) && words.len() == 2 {
            assert!(commit.is_none(), "R11 : deux lignes « {COMMIT_PREFIX} » dans frozen_renders.txt");
            commit = Some(words[1].to_owned());
        } else if words.first() == Some(&LINE_PREFIX) && words.len() == 6 {
            out.push((words[1..5].join(" "), words[5].to_owned()));
        } else {
            panic!("R11 : ligne illisible dans frozen_renders.txt : « {line} »");
        }
    }
    FrozenFile { platform, commit, lines: out }
}

/// R11 : les rendus de V1 à V6 restent ceux que la CI a relevés sur l'arbre d'EVORA-REV-95, empreinte
/// pour empreinte, cas pour cas (432 lignes). Sans empreinte dans le fichier des constantes, le test
/// ÉCHOUE (il ne passe jamais à vide) ; il échoue aussi sans la ligne du commit du relevé
/// (`frozen-commit`), que la porte « tests Rust » confronte à l'historique. Les empreintes ne se
/// comparent que sur la plateforme de leur relevé ; ailleurs, le test les imprime seulement, comme le
/// test d'or de l'amont, sauf sous `EVORA_FROZEN_STRICT=1` (posée par la porte), où l'écart de
/// plateforme est un échec : sur le runner de la CI, R11 ne peut pas passer sans rien confronter.
/// Sabotage qui le fait rougir : toucher `ToneMap::display_hdr`, ou n'importe quel bras de V1 à V6 de
/// `finish::base_tone` (ses trois sources les rendent tous).
///
/// Limites de cette garde de plateforme :
/// - hors de la plateforme du relevé et sans la variable (poste macOS), R11 passe sans rien
///   confronter : son « ok » n'y prouve rien ;
/// - la plateforme n'est connue que par son système et son architecture. Tout autre Linux x86_64
///   confronte donc aussi, avec ou sans la variable, alors que le dernier bit dépend en plus de la
///   libm et du compilateur du relevé (voir l'en-tête) : R11 peut y rougir sans
///   changement du code (un autre runner, ou le build nix du dépôt, qui joue tous les tests). La
///   variable ne protège que d'un changement de système ou d'architecture du runner.
///
/// Portée de ses empreintes : les jeux et les sources décrits dans l'en-tête de ce fichier, rendus
/// en sRGB, 8 puis 16 bits, dans des boîtes de 120 et de 480 points. R11 ne confronte PAS :
/// - l'épreuve écran (`RenderRequest::proof`) ni le rendu pour un écran profilé
///   (`RenderRequest::display`) ;
/// - un espace de sortie autre que sRGB ;
/// - le rendu HDR (ses cas laissent le réglage HDR éteint) ;
/// - le plafond que le brouillon pose sur la base des tons clairs et sombres, atteint au-delà de
///   1 600 points de bord long (le brouillon du jeu « cache » est rendu à 120 points).
///
/// Y ajouter un cas demande un relevé neuf par la CI, sur un arbre qui ne connaît pas encore la
/// version du rendu d'Evora : les cas imprimés et ceux que R11 lit doivent être les mêmes, et la
/// porte refuse un fichier des constantes dont le commit du relevé, ou le dernier commit à l'avoir
/// touché, connaît cette version. Une fois cette version arrivée, le fichier ne se retouche donc
/// plus, en-tête compris.
#[test]
fn frozen_renders_v1_to_v6() {
    let FrozenFile { platform, commit, lines: expected } = parse_frozen(FROZEN_FILE);
    assert!(
        !expected.is_empty(),
        "R11 : aucune empreinte dans crates/pipeline/src/frozen_renders.txt : y recopier, telles quelles, les lignes \
         de l'étape « Empreintes figées » de la CI d'Evora. Ce test échoue tant qu'il est vide."
    );
    let cases = frozen_cases();
    assert_eq!(cases.len(), FROZEN_TOTAL, "R11 : nombre de cas de la recette");
    assert_eq!(expected.len(), cases.len(), "R11 : {} empreintes pour {} cas", expected.len(), cases.len());
    let Some(platform) = platform else {
        panic!("R11 : ligne « frozen-platform » absente de frozen_renders.txt");
    };
    let Some(commit) = commit else {
        panic!("R11 : ligne « {COMMIT_PREFIX} <commit du relevé> » absente de frozen_renders.txt (voir son en-tête)");
    };
    // Quarante chiffres hexadécimaux en minuscules, comme la porte « tests Rust » l'exige.
    assert!(
        commit.len() == 40 && commit.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "R11 : commit du relevé illisible : « {commit} »"
    );
    let here = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    // La variable ne se pose qu'à 1 : une autre valeur (« true », un blanc de trop) se lirait comme
    // une variable absente, et l'écart de plateforme redeviendrait une impression seule en silence.
    let strict = match std::env::var_os(STRICT_VAR) {
        None => false,
        Some(v) => {
            assert!(v == "1", "R11 : {STRICT_VAR} ne se pose qu'à 1, lu {v:?}");
            true
        }
    };
    assert!(
        platform == here || !strict,
        "R11 : empreintes relevées sur « {platform} », ici « {here} » : sous {STRICT_VAR}=1, un écart de plateforme est un échec"
    );
    let got = frozen_lines();
    if platform != here {
        eprintln!("R11 : empreintes relevées sur « {platform} », ici « {here} » : impression seule");
        for line in &got {
            eprintln!("{line}");
        }
        return;
    }
    let mut changed = Vec::new();
    for (line, (key, fp)) in got.iter().zip(&expected) {
        let want = format!("{LINE_PREFIX} {key} {fp}");
        if *line != want {
            changed.push(format!("attendu « {want} », obtenu « {line} »"));
        }
    }
    assert!(changed.is_empty(), "R11 : {} rendu(s) figé(s) changé(s) :\n{}", changed.len(), changed.join("\n"));
}
