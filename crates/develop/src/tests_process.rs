//! Process versions: settings saved before they existed stay on V1, new settings get the latest,
//! and numbers this build doesn't know load without harm.

use serde_json::{Value, json};

use crate::*;

/// Complete settings as EvoLight wrote them before process versions existed (main at 8f924a9:
/// `serde_json::to_string` of a raw photo with tone, colour, curve, grading, effects and a radial
/// mask), one top-level key per line. There is no `process` field.
const LEGACY: &str = r#"
{
  "version": 1,
  "profile": {"id": "lc.color", "amount": 100.0},
  "treatment": "color",
  "wb": {"mode": "asShot", "temp": 5200.0, "tint": 4.0},
  "light": {"exposure": 0.4, "contrast": 30.0, "highlights": -60.0, "shadows": 40.0, "whites": 15.0, "blacks": -20.0},
  "curve": {"highlights": -10.0, "lights": 0.0, "darks": 10.0, "shadows": 0.0, "split_shadows": 25.0, "split_mid": 50.0, "split_highlights": 75.0, "master": [{"x": 0.0, "y": 0.05}, {"x": 0.5, "y": 0.55}, {"x": 1.0, "y": 0.95}], "red": [], "green": [], "blue": [], "refine_saturation": 100.0},
  "color": {"vibrance": 20.0, "saturation": -5.0},
  "mixer": {"red": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "orange": {"hue": 0.0, "sat": 0.0, "lum": 15.0}, "yellow": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "green": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "aqua": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "blue": {"hue": 0.0, "sat": -30.0, "lum": 0.0}, "purple": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "magenta": {"hue": 0.0, "sat": 0.0, "lum": 0.0}},
  "point_colors": [],
  "bw_mix": {"red": 0.0, "orange": 0.0, "yellow": 0.0, "green": 0.0, "aqua": 0.0, "blue": 0.0, "purple": 0.0, "magenta": 0.0},
  "grading": {"shadows": {"hue": 220.0, "sat": 20.0, "lum": -5.0}, "midtones": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "highlights": {"hue": 40.0, "sat": 15.0, "lum": 5.0}, "global": {"hue": 0.0, "sat": 0.0, "lum": 0.0}, "blending": 50.0, "balance": 0.0},
  "effects": {"texture": 20.0, "clarity": 25.0, "dehaze": 15.0},
  "vignette": {"amount": -25.0, "midpoint": 50.0, "roundness": 0.0, "feather": 50.0, "highlights": 0.0, "style": "highlightPriority"},
  "grain": {"amount": 20.0, "size": 25.0, "roughness": 50.0, "seed": 0},
  "detail": {"sharpen_amount": 40.0, "sharpen_radius": 1.0, "sharpen_detail": 25.0, "sharpen_masking": 0.0, "nr_luminance": 20.0, "nr_detail": 50.0, "nr_contrast": 0.0, "nr_color": 25.0, "nr_color_detail": 50.0, "nr_color_smoothness": 50.0},
  "optics": {"remove_ca": false, "lens_profile": false, "profile_distortion": 100.0, "profile_vignetting": 100.0, "defringe_purple_amount": 0.0, "defringe_purple_hue_lo": 30.0, "defringe_purple_hue_hi": 70.0, "defringe_green_amount": 0.0, "defringe_green_hue_lo": 40.0, "defringe_green_hue_hi": 60.0, "distortion": 0.0, "vignetting": 0.0, "vignetting_midpoint": 50.0, "ca_red": 0.0, "ca_blue": 0.0},
  "geometry": {"upright": "off", "guides": [], "upright_transform": null, "vertical": 0.0, "horizontal": 0.0, "rotate": 0.0, "aspect": 0.0, "scale": 100.0, "offset_x": 0.0, "offset_y": 0.0, "constrain_crop": false},
  "crop": {"geometry": {"rect": {"x0": 0.0, "y0": 0.0, "x1": 1.0, "y1": 1.0}, "angle": 0.0}, "aspect": null, "flip_h": false, "flip_v": false},
  "orientation": "Normal",
  "masks": [{"id": 1, "name": "Mask 1", "visible": true, "invert": false, "components": [{"op": "add", "invert": false, "shape": {"kind": "radial", "center": {"x": 0.4, "y": 0.5}, "rx": 0.3, "ry": 0.2, "angle": 15.0, "feather": 60.0, "invert": false}}], "adjust": {"temp": 10.0, "tint": 0.0, "exposure": 0.5, "contrast": 0.0, "highlights": 0.0, "shadows": 0.0, "whites": 0.0, "blacks": 0.0, "texture": 0.0, "clarity": 30.0, "dehaze": 0.0, "hue": 0.0, "saturation": 0.0, "sharpness": 0.0, "noise": 0.0, "moire": 0.0, "defringe": 0.0, "color_hue": 0.0, "color_sat": 0.0, "amount": 100.0}}],
  "spots": [],
  "red_eye": [],
  "lens_blur": {"enabled": false, "amount": 0.0, "focal_distance": 0.0, "focal_range": 0.0},
  "enhance": {"denoise": 0.0, "raw_details": false, "super_resolution": false},
  "calibration": {"shadows_tint": 0.0, "red_hue": 0.0, "red_sat": 0.0, "green_hue": 0.0, "green_sat": 0.0, "blue_hue": 0.0, "blue_sat": 0.0},
  "disabled_sections": []
}
"#;

fn legacy() -> Value {
    serde_json::from_str(LEGACY).unwrap()
}

#[test]
fn settings_saved_before_process_versions_load_as_v1() {
    let v = legacy();
    assert!(v.get("process").is_none(), "the fixture predates the field");
    let s = DevelopSettings::from_json(&v).unwrap();
    // V1 as a number, not `LATEST`: once a newer process is the latest, this is what keeps every
    // existing catalog, sidecar and preset on the rendering it was made with
    assert_eq!(s.process, ProcessVersion(1));
    assert_eq!(s.process, ProcessVersion::legacy());
    assert_eq!((s.light.exposure, s.light.highlights, s.masks.len(), s.detail.sharpen_amount), (0.4, -60.0, 1, 40.0));
    // the smallest legacy file too, and settings nested in other records (history, versions)
    assert_eq!(DevelopSettings::from_json(&json!({})).unwrap().process, ProcessVersion(1));
    let nested: Vec<DevelopSettings> = serde_json::from_value(json!([{}, {"light": {"exposure": 1.0}}])).unwrap();
    assert!(nested.iter().all(|s| s.process == ProcessVersion(1)));
    // saved again, they gain no field: files and preview-cache keys (`hash64`) stay as they were
    assert!(s.to_json().get("process").is_none());
}

#[test]
fn new_settings_get_the_latest_process() {
    assert_eq!(DevelopSettings::default().process, ProcessVersion::LATEST);
    assert_eq!(DevelopSettings::for_raw(5200.0, 4.0).process, ProcessVersion::LATEST);
    assert_eq!(ProcessVersion::LATEST, Process::LATEST.version());
    assert_eq!(ProcessVersion::V1, ProcessVersion(1));
    // the processes are listed oldest first, the latest last, each with its own number
    let versions: Vec<u32> = Process::ALL.iter().map(|p| p.version().0).collect();
    assert!(versions.windows(2).all(|w| w[0] < w[1]), "{versions:?}");
    // Evora (EVORA-REV-98) : la plus récente est dans la liste, sans en être forcément la dernière
    // (`Evora1` est déclarée après V6 avant d'être la version des photos neuves) : le « the latest
    // last » du commentaire de l'amont ci-dessus ne vaut plus. L'ordre strict des numéros qu'il
    // précède ne tient que tant qu'aucune version de l'amont n'est déclarée après `Evora1` ; à la
    // montée qui en apportera une (le cas que R15 modélise), cette assertion se lira par plage.
    assert!(Process::ALL.contains(&Process::LATEST));
    assert!(Process::ALL.iter().all(|p| p.version().process() == *p && p.version().is_known()));
}

#[test]
fn v1_is_left_out_when_saving_and_other_processes_are_kept() {
    let mut s = DevelopSettings::from_json(&legacy()).unwrap();
    assert!(s.to_json().get("process").is_none());
    let before = s.hash64();
    s.process = ProcessVersion(2);
    let v = s.to_json();
    assert_eq!(v["process"], json!(2));
    assert_eq!(DevelopSettings::from_json(&v).unwrap().process, ProcessVersion(2));
    assert_ne!(s.hash64(), before, "another process is another preview");
}

#[test]
fn unknown_process_numbers_load_and_render_with_the_newest_known() {
    // written by a newer EvoLight: kept as it is, rendered with the latest process this build has
    // Evora (EVORA-REV-98) : 7, écrit par une build plus récente de l'amont, se rend avec la plus
    // récente version de l'amont connue ici, nommée (V6) : la résolution se fait par plage, et la
    // plus récente version sera un jour celle d'Evora, d'une autre plage
    let s = DevelopSettings::from_json(&json!({"process": 7, "light": {"exposure": 0.5}})).unwrap();
    assert_eq!(s.process, ProcessVersion(7));
    assert_eq!(s.process.process(), Process::V6);
    assert!(!s.process.is_known());
    assert!(!s.process.is_outdated(), "never downgraded by Update to Current Process");
    assert_eq!(s.to_json()["process"], json!(7));
    assert_eq!(s.merged(&json!({"light": {"exposure": 1.0}})).unwrap().process, ProcessVersion(7));
    // below V1 (no EvoLight writes it): rendered as V1, and updatable
    let zero = DevelopSettings::from_json(&json!({"process": 0})).unwrap().process;
    assert_eq!((zero.process(), zero.is_known(), zero.is_outdated()), (Process::V1, false, true));
    // Evora (EVORA-REV-98) : le plus grand numéro est dans la plage d'Evora, il se rend avec la plus
    // récente version d'Evora connue (vrai avant comme après la bascule)
    let max = DevelopSettings::from_json(&json!({"process": u32::MAX})).unwrap().process;
    assert_eq!(max.process(), Process::Evora1);
    // Evora (EVORA-REV-98) : 1002, écrit par une build plus récente d'Evora, se rend avec la plus
    // récente version d'Evora connue et n'est jamais rétrogradé ; 1000, que nul n'écrit, avec la plus
    // récente de l'amont. Tant que la version des photos neuves est V6, ni l'un ni l'autre ne se met
    // à jour : la mise à jour ne ramène pas un numéro d'Evora à V6
    let next = DevelopSettings::from_json(&json!({"process": EVORA_BASE + 2})).unwrap().process;
    assert_eq!((next.process(), next.is_known(), next.is_outdated()), (Process::Evora1, false, false));
    let base = DevelopSettings::from_json(&json!({"process": EVORA_BASE})).unwrap().process;
    assert_eq!((base.process(), base.is_known(), base.is_outdated()), (Process::V6, false, false));
    // tant que la version des photos neuves est V6 : V1 à V5 se mettent à jour, V6 et 1001 non
    for p in Process::ALL {
        let older = matches!(p, Process::V1 | Process::V2 | Process::V3 | Process::V4 | Process::V5);
        assert_eq!(p.version().is_outdated(), older, "{p:?}");
    }
    // a whole number written as a float is that number
    assert_eq!(DevelopSettings::from_json(&json!({"process": 2.0})).unwrap().process, ProcessVersion(2));
}

/// R15 (Evora, EVORA-REV-98, exception L1-REV-I) : la résolution par plage et la règle de mise à
/// jour, sur une liste fictive où une version future de l'amont (7) est déclarée après la version
/// d'Evora (1001), comme elle le sera à une montée, puis sur une liste où une version d'Evora fondée
/// sur 7 (1002) la suit, puis sur la liste de cette build.
/// Sabotages qui le font rougir : l'ancienne résolution (la dernière version de la liste qui ne
/// dépasse pas le numéro), qui lit 1001 comme 7 ; la règle de mise à jour qui met un numéro de l'amont
/// en regard de 1001 par ordre (`n < 1001`), qui rétrograderait une édition V7 vers 1001, fondée sur
/// V6 ; une règle qui, la plus récente restant V6, mettrait à jour tout autre numéro connu, et
/// ramènerait 1001 à V6.
#[test]
fn version_numbers_resolve_within_their_range() {
    let known = [1, 2, 3, 4, 5, 6, EVORA_BASE + 1, 7];
    let cases = [
        (EVORA_BASE + 1, Some(EVORA_BASE + 1)),
        (EVORA_BASE + 2, Some(EVORA_BASE + 1)),
        (u32::MAX, Some(EVORA_BASE + 1)),
        (7, Some(7)),
        (8, Some(7)),
        (EVORA_BASE - 1, Some(7)),
        (6, Some(6)),
        (1, Some(1)),
        // 1000 : aucune version d'Evora ne le précède, repli sur la plus récente de l'amont
        (EVORA_BASE, Some(7)),
        // sous V1 : aucune (V1 pour `ProcessVersion::process`)
        (0, None),
    ];
    for (n, want) in cases {
        assert_eq!(resolve_in_range(n, &known), want, "{n}");
    }
    for (n, newer) in [(8, true), (7, false), (0, false), (EVORA_BASE + 2, true), (EVORA_BASE + 1, false), (EVORA_BASE, false)] {
        assert_eq!(newer_than_its_range(n, &known), newer, "{n}");
    }
    assert!(in_evora_range(EVORA_BASE) && !in_evora_range(EVORA_BASE - 1));
    // la mise à jour, sur la même liste (plus récente : 1001, fondée sur V6) : 7, déclarée après 1001
    // sans version d'Evora fondée sur elle, n'est jamais rétrogradée vers 1001
    let (latest, base) = (EVORA_BASE + 1, 6);
    for (n, outdated) in [(6, true), (7, false), (8, false), (0, true), (EVORA_BASE, true), (EVORA_BASE + 1, false), (EVORA_BASE + 2, false)] {
        assert_eq!(outdated_in_range(n, &known, latest, base), outdated, "{n}");
    }
    // une version d'Evora fondée sur 7 (1002) devenue la plus récente : 7 et 1001 se mettent à jour vers elle
    let next = [1, 2, 3, 4, 5, 6, EVORA_BASE + 1, 7, EVORA_BASE + 2];
    for (n, outdated) in [(6, true), (7, true), (8, false), (EVORA_BASE + 1, true), (EVORA_BASE + 2, false), (EVORA_BASE + 3, false)] {
        assert_eq!(outdated_in_range(n, &next, EVORA_BASE + 2, 7), outdated, "{n}");
    }
    // une plus récente de l'amont (avant toute version d'Evora) : la règle d'avant
    for (n, outdated) in [(0, true), (5, true), (6, false), (7, false), (EVORA_BASE + 1, false)] {
        assert_eq!(outdated_in_range(n, &[1, 2, 3, 4, 5, 6], 6, 6), outdated, "{n}");
    }
    // la liste de cette build, où la plus récente reste V6 alors que 1001 est connue : aucun numéro de
    // la plage d'Evora ne se met à jour (1001 n'est pas ramené à V6), et pour un numéro de l'amont la
    // règle d'avant tient (plus ancien que V6)
    let this = Process::NUMBERS;
    let unchanged = [6, 7, EVORA_BASE - 1, EVORA_BASE, EVORA_BASE + 1, EVORA_BASE + 2, u32::MAX];
    for n in [0, 1, 5] {
        assert!(outdated_in_range(n, &this, 6, 6), "{n}");
    }
    for n in unchanged {
        assert!(!outdated_in_range(n, &this, 6, 6), "{n}");
    }
    // chaque numéro connu se résout vers lui-même, et vers sa version
    assert!(this.iter().all(|&n| resolve_in_range(n, &this) == Some(n)));
    assert_eq!(this.len(), Process::ALL.len());
    assert!(Process::ALL.iter().zip(this).all(|(p, n)| p.version().0 == n));
    let rendered_by = [
        (0, Process::V1),
        (1, Process::V1),
        (6, Process::V6),
        (7, Process::V6),
        (EVORA_BASE - 1, Process::V6),
        (EVORA_BASE, Process::V6),
        (EVORA_BASE + 1, Process::Evora1),
        (EVORA_BASE + 2, Process::Evora1),
        (u32::MAX, Process::Evora1),
    ];
    for (n, want) in rendered_by {
        assert_eq!(ProcessVersion(n).process(), want, "{n}");
    }
    assert_eq!(Process::Evora1.version(), ProcessVersion(EVORA_BASE + 1));
    assert_eq!(Process::Evora1.version().0, 1001);
    assert!(Process::Evora1.is_evora() && !Process::V6.is_evora());
    // une version d'Evora est numérotée dans sa plage, et elle seule
    assert!(Process::ALL.iter().all(|p| p.is_evora() == in_evora_range(p.version().0)));
    // la version de l'amont de chaque version : elle-même pour l'amont, V6 pour Evora1
    assert!(Process::ALL.iter().all(|p| p.is_evora() || p.upstream_base() == *p));
    assert_eq!(Process::Evora1.upstream_base(), Process::V6);
    assert!(!Process::LATEST.upstream_base().is_evora());
    // les réglages neufs épinglés à la version de l'amont : ceux d'une photo neuve, à la version près
    let pinned = Process::LATEST.upstream_base().version();
    assert_eq!(DevelopSettings::upstream_default(), DevelopSettings { process: pinned, ..DevelopSettings::default() });
    assert_eq!(DevelopSettings::upstream_for_raw(5200.0, 4.0), DevelopSettings { process: pinned, ..DevelopSettings::for_raw(5200.0, 4.0) });
}

/// Evora (EVORA-REV-98, exception L1-REV-I) : le numéro d'`Evora1` (1001) survit à l'enregistrement
/// et à la relecture, en valeur JSON comme en texte (sidecars, journal et catalogue l'écrivent
/// ainsi), et nomme une autre empreinte des réglages (`hash64`, clé des aperçus) que V6 aux mêmes
/// curseurs : aucun aperçu n'est partagé entre les deux versions. La version est posée ici en
/// toutes lettres : elle n'est pas encore celle des photos neuves. Les tests du moteur sur la mise
/// à jour et la clé de rendu n'ont pas de course dans la CI d'Evora : ce test porte ce que le schéma
/// des réglages peut prouver seul.
/// Sabotages qui le font rougir : 1001 omis à l'enregistrement (comme V1), ou relu comme V6.
#[test]
fn evora_process_number_survives_a_round_trip_and_keys_the_settings() {
    let mut s = DevelopSettings { process: Process::Evora1.version(), ..Default::default() };
    (s.light.exposure, s.light.contrast) = (0.7, 20.0);
    assert_eq!(s.process, ProcessVersion(EVORA_BASE + 1));
    let v = s.to_json();
    assert_eq!(v["process"], json!(EVORA_BASE + 1));
    let back = DevelopSettings::from_json(&v).unwrap();
    assert_eq!(back, s);
    assert_eq!((back.process.process(), back.process.is_known(), back.process.is_outdated()), (Process::Evora1, true, false));
    let text = serde_json::to_string(&s).unwrap();
    let again: DevelopSettings = serde_json::from_str(&text).unwrap();
    assert_eq!(again, s);
    let v6 = DevelopSettings { process: Process::V6.version(), ..s.clone() };
    assert_ne!(s.hash64(), v6.hash64(), "Evora1 et V6 aux mêmes curseurs : deux aperçus distincts");
    assert_eq!(DevelopSettings::from_json(&v6.to_json()).unwrap().process.process(), Process::V6);
    // une fusion partielle, la remise à zéro d'une section et le test « non retouché » gardent 1001
    assert_eq!(s.merged(&json!({"light": {"exposure": 1.0}})).unwrap().process, ProcessVersion(EVORA_BASE + 1));
    let mut reset = s.clone();
    reset.reset_section(Section::Light);
    assert_eq!(reset.process, ProcessVersion(EVORA_BASE + 1));
    assert!(reset.is_unedited(), "la version du rendu n'est pas une retouche");
}

#[test]
fn a_damaged_process_value_reads_as_v1_and_keeps_the_edits() {
    let too_big = u64::from(u32::MAX) + 1;
    for bad in [
        json!(-1),
        json!(1.5),
        json!(-0.5),
        json!(1e300),
        json!(too_big),
        json!(u64::MAX),
        json!("2"),
        json!(null),
        json!(true),
        json!([2]),
        json!({"v": 2}),
    ] {
        let mut v = legacy();
        v["process"] = bad.clone();
        let s = DevelopSettings::from_json(&v).unwrap_or_else(|e| panic!("{bad}: {e}"));
        assert_eq!(s.process, ProcessVersion::V1, "{bad}");
        assert_eq!((s.light.exposure, s.masks.len(), s.curve.master.len()), (0.4, 1, 3), "{bad}: the rest of the settings load");
        // nested in other records too (history, versions, the catalog)
        let nested: Vec<DevelopSettings> = serde_json::from_value(json!([v])).unwrap();
        assert_eq!(nested[0].process, ProcessVersion::V1);
    }
}

#[test]
fn the_process_is_not_an_edit_nor_part_of_copied_settings() {
    let mut s = DevelopSettings { process: ProcessVersion(LATEST_PLUS_ONE), ..Default::default() };
    assert!(s.is_unedited(), "a photo on another process is not edited for that");
    s.light.exposure = 1.0;
    assert!(!s.is_unedited());
    // resetting a section or a slider leaves the process alone
    s.reset_section(Section::Light);
    assert_eq!(s.process, ProcessVersion(LATEST_PLUS_ONE));
    // copy / paste / sync / presets: no group carries the process, so the target keeps its own
    assert!(SettingsGroup::ALL.iter().all(|g| !g.keys().contains(&"process")));
    s.light.exposure = 1.0;
    let copied = extract_groups(&s, &SettingsGroup::ALL);
    assert!(copied.get("process").is_none());
    let target = DevelopSettings::from_json(&legacy()).unwrap();
    let pasted = apply_partial(&target, &copied, 1.0);
    assert_eq!((pasted.process, pasted.light.exposure), (ProcessVersion(1), 1.0));
    // (at another amount, without Grain: its integer seed doesn't survive scaling, a separate issue)
    let groups: Vec<SettingsGroup> = SettingsGroup::ALL.into_iter().filter(|g| *g != SettingsGroup::Grain).collect();
    let preset = Preset::from_settings("p", "P", "User", &s, &groups);
    let half = preset.apply(&target, 0.5);
    assert_eq!((half.process, half.light.exposure), (ProcessVersion(1), 0.7));
    // a partial that names a process explicitly (Apply Settings JSON) sets it
    assert_eq!(target.merged(&json!({"process": LATEST_PLUS_ONE})).unwrap().process, ProcessVersion(LATEST_PLUS_ONE));
}

/// A process other than the latest: a newer EvoLight's number (no older one than V1 exists).
const LATEST_PLUS_ONE: u32 = ProcessVersion::LATEST.0 + 1;
