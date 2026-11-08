//! The Evo App Suite as releases know it: each app's id, name and repository.
//!
//! An app that updates itself takes its own entry (`suite::EVOPHOTO`); the launcher takes all
//! of them. Installers use the list to tell a Evo App's package from anyone else's.

use crate::app::App;

pub const EVOPHOTO: App = App::new("evophoto", "EvoPhoto", "Image editing", "tazerty/evophoto");
pub const EVOVECTOR: App = App::new("evovector", "EvoVector", "Vector illustration", "tazerty/evovector");
pub const EVOFILM: App = App::new("evofilm", "EvoFilm", "Video editing, color and sound", "tazerty/evofilm");
pub const EVOLIGHT: App = App::new("evolight", "EvoLight", "Photo library and raw development", "tazerty/evolight");
pub const EVOPDF: App = App {
    former_ids: &["evoprint"],
    former_names: &["EvoPrint"],
    ..App::new("evopdf", "EvoPdf", "Reading, organizing and protecting PDFs", "tazerty/evopdf")
};
pub const EVOEFFECT: App = App::new("evoeffect", "EvoEffect", "Motion graphics and visual effects", "tazerty/evoeffect");
pub const EVODESIGN: App = App::new("evodesign", "EvoDesign", "Page layout and publishing", "tazerty/evodesign");
pub const EVOFIG: App = App::new("evofig", "EvoFig", "Interface design and prototyping", "tazerty/evofig");
pub const EVOWORD: App = App::new("evoword", "EvoWord", "Writing and document design", "tazerty/evoword");
pub const EVOSOUND: App = App::new("evosound", "EvoSound", "Recording, editing and mixing audio", "tazerty/evosound");
pub const EVODECK: App = App::new("evodeck", "EvoDeck", "Presentations and slide shows", "tazerty/evodeck");
pub const EVOCAD: App = App::new("evocad", "EvoCAD", "Computer-aided design and drafting", "storytold/evocad");
pub const EVOGRID: App = App::new("evogrid", "EvoGrid", "Spreadsheets and data", "tazerty/evogrid");

/// Upstream Launcher, described like a Evo App so it updates itself with the same machinery
/// (asset names `upstream-launcher-<ver>-…`, bundle id `ai.storyteller.upstream-launcher`,
/// `Upstream Launcher.app`). Not part of [`SUITE`].
pub const LAUNCHER: App = App::new("upstream-launcher", "Upstream Launcher", "Installs and updates the Evo App Suite", "amont/evo-launcher");

/// The Evo App Suite, in upstream-site.com's display order.
pub const SUITE: [App; 13] = [
    EVOPHOTO,
    EVOVECTOR,
    EVOFILM,
    EVOLIGHT,
    EVOPDF,
    EVOEFFECT,
    EVODESIGN,
    EVOFIG,
    EVOWORD,
    EVOSOUND,
    EVODECK,
    EVOCAD,
    EVOGRID,
];

/// The suite's app with this id.
pub fn by_id(id: &str) -> Option<&'static App> {
    SUITE.iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirteen_apps_with_unique_ids_in_their_own_repos() {
        let mut ids: Vec<_> = SUITE.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), SUITE.len());
        for a in &SUITE {
            assert_eq!(a.repo, format!("storytold/{}", a.id));
            assert_eq!(a.name.to_lowercase(), a.id);
        }
        assert!(by_id(LAUNCHER.id).is_none());
        assert_eq!(by_id("evopdf").map(|a| a.former_ids), Some(&["evoprint"][..]));
    }
}
