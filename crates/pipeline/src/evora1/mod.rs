//! Evora (EVORA-REV-99) : le rendu d'Evora, celui de la version du rendu `Evora1` (numéro 1001).
//!
//! À ce commit le module ne porte que le prédicat du rendu, [`renders`], et rien du rendu lui-même.
//! Le chemin principal ne le lit pas encore : sous `Evora1` il rend toujours comme V6, au bit près
//! (bras recopiés d'EVORA-REV-98). Seul le chemin graphique le lit, pour laisser au chemin principal
//! les rendus qui seront ceux d'Evora (`run`, dans `evolight_gpu`).
//!
//! Règle d'écriture du rendu d'Evora : chaque branche neuve du pipeline se ferme par ce prédicat, et
//! par lui seul. Le code d'Evora ne range jamais deux versions du rendu par leur ordre.

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
}
