//! Evora (lot 1 « REV-TONS », P0) : empreintes figées des rendus des versions V1 à V6, recette
//! « éditions figées » côté Rust.
//!
//! `print_frozen_render_fingerprints`, ignoré par défaut, imprime une empreinte par cas : chaque
//! version de V1 à V6, un fichier rendu et un brut de test, les crans −1, 0, +1 et +2, et deux jeux
//! de réglages. L'étape « Empreintes figées » de la CI d'Evora le joue sur l'arbre d'avant la
//! version du rendu d'Evora et dépose ses lignes dans le résumé et les diagnostics ; elles
//! deviennent les constantes du test qui garde ces rendus figés (R11).
//!
//! Les rendus appellent `powf`, `exp2` et `log2` de la plateforme, dont le dernier bit peut changer
//! d'une plateforme à l'autre (même raison que le test d'or de V1 et V2 de l'amont) : une empreinte
//! ne vaut que pour la plateforme où elle a été relevée.

use evolight_develop::{DevelopSettings, Process};
use evolight_raster::Rgb32f;

use crate::{DeepSamples, OutputDepth, RenderRequest, SourceInfo, render};

/// Les versions figées : toutes celles de l'amont connues au moment du relevé.
pub(crate) const FROZEN_VERSIONS: [Process; 6] = [Process::V1, Process::V2, Process::V3, Process::V4, Process::V5, Process::V6];
/// Crans de la recette, en stops.
pub(crate) const FROZEN_STEPS: [f64; 4] = [-1.0, 0.0, 1.0, 2.0];
/// Côté de la boîte de rendu (la scène de démonstration y est réduite de moitié).
const FROZEN_SIDE: usize = 120;
/// Préfixe des lignes d'empreintes, lu par la CI et par le fichier des constantes.
pub(crate) const LINE_PREFIX: &str = "frozen";

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

/// Tous les cas, dans l'ordre où ils s'impriment.
pub(crate) fn frozen_cases() -> Vec<FrozenCase> {
    let mut out = Vec::new();
    for version in FROZEN_VERSIONS {
        for source in ["rendu", "brut"] {
            for variant in ["simple", "retouche"] {
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

fn source_info(source: &str) -> SourceInfo {
    if source == "brut" { SourceInfo { raw: true, as_shot_temp: 5200.0, as_shot_tint: 4.0, ..Default::default() } } else { SourceInfo::default() }
}

/// Les réglages d'un cas : ceux d'une photo neuve de ce type, la version du cas, le cran, et pour
/// « retouche » quatre curseurs de la lumière et un mélange de couleur.
fn settings(case: &FrozenCase) -> DevelopSettings {
    let mut s = if case.source == "brut" { DevelopSettings::for_raw(5200.0, 4.0) } else { DevelopSettings::default() };
    s.process = case.version.version();
    s.light.exposure = case.step;
    if case.variant == "retouche" {
        let l = &mut s.light;
        (l.contrast, l.highlights, l.whites, l.blacks) = (30.0, -60.0, 15.0, -20.0);
        s.mixer.blue.sat = -30.0;
        s.mixer.orange.lum = 15.0;
    }
    s
}

/// FNV-1a 64 bits, comme le test d'or de l'amont.
fn fnv(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for b in bytes {
        *hash = (*hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3);
    }
}

/// L'empreinte d'un cas : les octets du rendu 8 bits, puis les échantillons du rendu 16 bits.
pub(crate) fn fingerprint(src: &Rgb32f, case: &FrozenCase) -> u64 {
    let info = source_info(case.source);
    let s = settings(case);
    let mut h = 0xCBF2_9CE4_8422_2325_u64;
    let eight = render(src, &info, &s, &RenderRequest::fit(FROZEN_SIDE, FROZEN_SIDE));
    fnv(&mut h, eight.image.data.iter().flatten().copied());
    let req = RenderRequest { depth: OutputDepth::U16, ..RenderRequest::fit(FROZEN_SIDE, FROZEN_SIDE) };
    let Some(deep) = render(src, &info, &s, &req).deep else {
        panic!("{} : rendu 16 bits absent", case.key());
    };
    let DeepSamples::U16(v) = deep.samples else {
        panic!("{} : échantillons 16 bits attendus", case.key());
    };
    fnv(&mut h, v.iter().flat_map(|x| x.to_le_bytes()));
    h
}

/// Les lignes d'empreintes de tous les cas : `frozen <clé> <empreinte>`.
pub(crate) fn frozen_lines(src: &Rgb32f) -> Vec<String> {
    let mut out = Vec::new();
    for case in frozen_cases() {
        out.push(format!("{LINE_PREFIX} {} {:#018x}", case.key(), fingerprint(src, &case)));
    }
    out
}

/// P0 : imprime les empreintes de V1 à V6, la plateforme et leur nombre. À jouer par l'étape
/// « Empreintes figées » de la CI d'Evora :
/// `cargo test -p evolight-pipeline --lib -- --ignored --exact tests_frozen::print_frozen_render_fingerprints --nocapture`.
#[test]
#[ignore = "P0 : impression des empreintes figées, jouée par la CI d'Evora"]
fn print_frozen_render_fingerprints() {
    let lines = frozen_lines(&frozen_scene());
    println!();
    println!("{LINE_PREFIX}-platform {} {}", std::env::consts::OS, std::env::consts::ARCH);
    for line in &lines {
        println!("{line}");
    }
    println!("{LINE_PREFIX}-total {}", lines.len());
}
