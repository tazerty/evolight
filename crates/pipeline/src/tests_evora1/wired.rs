//! Evora (EVORA-REV-112) : le rendu d'Evora branché, au niveau du rendu entier. Sous le prédicat
//! (le rendu SDR d'un fichier rendu sous `Evora1`), l'image passe le pont de scène à la fin de l'étage
//! linéaire, le noyau lit la table de tons d'Evora et la borne au blanc, et les clés du cache de rendu
//! portent la marque d'Evora. Ce que ces tests gardent :
//!
//! - `neutral_rendered_file_is_the_file` : aux réglages neutres, le rendu d'un fichier est ce fichier ;
//! - `render_matches_the_reference_where_the_gamut_return_rests` : exposition, contraste, blancs et
//!   noirs face aux vecteurs d'or de la chaîne par point de la référence ;
//! - `wired_tone_table_is_increasing_and_bounded` : la table du noyau est celle d'Evora, croissante
//!   et bornée, sous le prédicat seulement ;
//! - `exposure_creates_no_white_patch` et `chroma_memory_returns_colour_to_highlights` : deux
//!   contrôles de la référence ;
//! - `version_switch_recomputes_the_linear_stage` : un cache de rendu ne mêle pas les deux versions.
//!
//! Chaque test porte son témoin rouge : il affirme aussi qu'une variante sabordée nommée manque la
//! borne. Les aides (`at`, `retouched`, `bits`, la scène et la boîte de la retouche) sont celles du
//! module père.

use std::sync::Arc;

use evolight_color::luminance_2020;
use evolight_color::perceptual::{oklab_from_2020, oklab_to_2020};
use evolight_color::transfer::linear_to_srgb;
use evolight_develop::{DevelopSettings, Process};
use evolight_raster::{Rgb32f, Rgba8};

use super::{RETOUCH_BOX, at, bits, hdr_on, retouch_render, retouch_scene, retouched};
use crate::evora1::GREY;
use crate::evora1::bridge::Bridge;
use crate::tone::{LUT_N, ToneMap};
use crate::{DeepSamples, OutputDepth, OutputSpace, Quality, RenderRequest, Rendered, SourceInfo, StageCache, plan, render, render_cached};

// Evora (EVORA-REV-113) : les tests du retour de gamme et de la borne finale, au niveau du rendu
// entier, dans `tests_evora1/wired/pushed.rs`. Module fils : il lit les aides de ce fichier.
mod pushed;

// --- Evora (EVORA-REV-112) : le rendu d'Evora branché ---------------------------------------------
//
// Les tableaux, les tolérances et les seuils qui suivent sont écrits par outil (`poser_rev112.py`, à
// côté des vecteurs d'or) : les vecteurs de la chaîne et leurs tolérances depuis `tableaux-rust.txt`
// et `06-fin-rendre.json`, que le générateur des vecteurs tire de la référence ; les triplets, les
// seuils et la mire des contrôles depuis le texte de `controles.py`, dans la référence. Ils ne se
// retouchent pas à la main ; ils se régénèrent.

// >>> or:CHAINE-AU-REPOS (EVORA-REV-112 : écrit par poser_rev112.py depuis tableaux-rust.txt (GOLD_CHAIN) et 06-fin-rendre.json ; ne pas retoucher)
/// Evora (EVORA-REV-112) : ((exposition en EV, contraste, blancs, noirs), source Rec.2020, rendu Rec.2020 d'affichage).
/// Chaîne par point de la référence en double précision, avant vibrance et saturation : les 98 vecteurs de GOLD_CHAIN (sur 150) où le retour de gamme de la référence laisse la couleur telle quelle et où le rendu tient dans 0..1.
#[rustfmt::skip]
const GOLD_CHAIN_AT_REST: [((f64, f64, f64, f64), [f32; 3], [f64; 3]); 98] = [
    ((0.0, 0.0, 0.0, 0.0), [0.697884, 0.26591676, 0.035229906], [0.6978840231895447, 0.26591676473617554, 0.03522990643978119]),
    ((0.0, 0.0, 0.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]),
    ((0.0, 0.0, 0.0, 0.0), [0.9566869, 0.9886377, 0.10440475], [0.9566869139671326, 0.9886376857757568, 0.10440474748611461]),
    ((0.0, 0.0, 0.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.06050226464867592, 0.04249603673815728, 0.898673176765442]),
    ((0.0, 0.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.6553377509117126, 0.46997091174125677, 0.3376247584819795]),
    ((0.0, 0.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.6575288772583007, 0.7708704471588134, 0.4804166555404662]),
    ((0.0, 0.0, 0.0, 0.0), [0.3532253, 0.92266, 0.1541484], [0.353225290775299, 0.9226599931716918, 0.1541483998298645]),
    ((0.0, 0.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.21404114365577706, 0.21404114365577706, 0.21404114365577706]),
    ((0.0, 0.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.024240883067250255, 0.01154856383800507, 0.004949785303324463]),
    ((1.0, 0.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.7982444208344854, 0.9240051291758233, 0.6017255400858414]),
    ((1.0, 0.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.4026816074566738, 0.4026816074566738, 0.4026816074566738]),
    ((1.0, 0.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.05846569220224558, 0.027853555369976808, 0.011938204693637582]),
    ((-1.0, 0.0, 0.0, 0.0), [0.697884, 0.26591676, 0.035229906], [0.360726752682482, 0.13744875629723288, 0.018209859124226374]),
    ((-1.0, 0.0, 0.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.2910700638059084, 0.03205614682139793, 0.007604442987749274]),
    ((-1.0, 0.0, 0.0, 0.0), [0.9566869, 0.9886377, 0.10440475], [0.8523864746541614, 0.8819510249850406, 0.06375641720317016]),
    ((-1.0, 0.0, 0.0, 0.0), [0.3725961, 0.9309027, 0.98360854], [0.24655527660595578, 0.7137516581347422, 0.7578564031433659]),
    ((-1.0, 0.0, 0.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.026068395713752215, 0.018310116297120788, 0.38720811733729094]),
    ((-1.0, 0.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.38105680567333516, 0.2720521604065697, 0.1942262366706494]),
    ((-1.0, 0.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.45932307636348335, 0.5467724778932861, 0.32267102710712026]),
    ((-1.0, 0.0, 0.0, 0.0), [0.3532253, 0.92266, 0.1541484], [0.22476297244054616, 0.666292328520537, 0.0704023563775707]),
    ((-1.0, 0.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.09939289845840966, 0.09939289845840966, 0.09939289845840966]),
    ((-1.0, 0.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.009929166501131905, 0.004730339768497157, 0.002027452642101262]),
    ((2.0, 0.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.6271619369052397, 0.6271619369052397, 0.6271619369052397]),
    ((2.0, 0.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.13704864764608576, 0.0652911468555926, 0.027984186057791058]),
    ((0.0, 50.0, 0.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.05440241799221088, 0.038211580460107246, 0.8080688232857997]),
    ((0.0, 50.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.7272641616200686, 0.523184635419454, 0.3774782060899304]),
    ((0.0, 50.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.7135167131035723, 0.8320311325049441, 0.5283211829534181]),
    ((0.0, 50.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.2213204683323334, 0.2213204683323334, 0.2213204683323334]),
    ((0.0, 50.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.018318065025849914, 0.00872688271103287, 0.0037403954632655584]),
    ((0.0, -50.0, 0.0, 0.0), [0.697884, 0.26591676, 0.035229906], [0.6402889161740428, 0.2439711347843685, 0.03232244593900652]),
    ((0.0, -50.0, 0.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.6107580577897952, 0.06726404535341327, 0.015956552759297193]),
    ((0.0, -50.0, 0.0, 0.0), [0.9566869, 0.9886377, 0.10440475], [0.9267486807617051, 0.9579514402664547, 0.0944196331198569]),
    ((0.0, -50.0, 0.0, 0.0), [0.3725961, 0.9309027, 0.98360854], [0.3366380699553167, 0.8741287930266329, 0.9248695355126715]),
    ((0.0, -50.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.5954651790320032, 0.425930443475982, 0.3048879193919637]),
    ((0.0, -50.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.6057021882075972, 0.7135497960402477, 0.4371750586913854]),
    ((0.0, -50.0, 0.0, 0.0), [0.3532253, 0.92266, 0.1541484], [0.3162058503724181, 0.8584026051796725, 0.12665146639566793]),
    ((0.0, -50.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.2081586453505532, 0.2081586453505532, 0.2081586453505532]),
    ((0.0, -50.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.03053049986755397, 0.01454499102811139, 0.0062340723779874315]),
    ((0.0, 100.0, 0.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.047768937950217485, 0.033552306741953065, 0.7095381217167829]),
    ((0.0, 100.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.8075036886220212, 0.5866704668489986, 0.42900242037258984]),
    ((0.0, 100.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.7701771113164108, 0.8932747106686552, 0.5778197147447468]),
    ((0.0, 100.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.23036465545632548, 0.23036465545632548, 0.23036465545632548]),
    ((0.0, 100.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.01303669208412043, 0.006210791510862159, 0.002661983340414068]),
    ((0.0, -100.0, 0.0, 0.0), [0.697884, 0.26591676, 0.035229906], [0.5945653294191198, 0.22654894448054963, 0.03001427204486129]),
    ((0.0, -100.0, 0.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.5972505813399264, 0.06577643909599923, 0.015603658912268364]),
    ((0.0, -100.0, 0.0, 0.0), [0.9566869, 0.9886377, 0.10440475], [0.895606950011086, 0.926014099418776, 0.08450068538236111]),
    ((0.0, -100.0, 0.0, 0.0), [0.3725961, 0.9309027, 0.98360854], [0.3050400598082143, 0.820991778046958, 0.8696991736291281]),
    ((0.0, -100.0, 0.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.546407320900318, 0.39010270883425746, 0.2785060811083867]),
    ((0.0, -100.0, 0.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.559798632949767, 0.6622986116402201, 0.3996279297573565]),
    ((0.0, -100.0, 0.0, 0.0), [0.3532253, 0.92266, 0.1541484], [0.28468212075007243, 0.8002509433792384, 0.10443697965580045]),
    ((0.0, -100.0, 0.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.20338818919406018, 0.20338818919406018, 0.20338818919406018]),
    ((0.0, -100.0, 0.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.03691617412060373, 0.01758718080954231, 0.007537973580013589]),
    ((0.0, 0.0, 100.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]),
    ((0.0, 0.0, 100.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.06050226464867592, 0.04249603673815728, 0.898673176765442]),
    ((0.0, 0.0, 100.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.7344306958030988, 0.528639186844987, 0.38171045511544976]),
    ((0.0, 0.0, 100.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.7893082995855214, 0.9140272104571879, 0.5944173746221681]),
    ((0.0, 0.0, 100.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.21404114365577706, 0.21404114365577706, 0.21404114365577706]),
    ((0.0, 0.0, 100.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.024240883067250255, 0.01154856383800507, 0.004949785303324463]),
    ((0.0, 0.0, -100.0, 0.0), [0.697884, 0.26591676, 0.035229906], [0.6819577781368651, 0.2598483415912401, 0.034425933136912557]),
    ((0.0, 0.0, -100.0, 0.0), [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]),
    ((0.0, 0.0, -100.0, 0.0), [0.3725961, 0.9309027, 0.98360854], [0.22418163528501117, 0.6644520954377507, 0.7060149524023643]),
    ((0.0, 0.0, -100.0, 0.0), [0.060502265, 0.042496037, 0.8986732], [0.06050226464867592, 0.04249603673815728, 0.898673176765442]),
    ((0.0, 0.0, -100.0, 0.0), [0.65533775, 0.4699709, 0.33762476], [0.575555208963726, 0.4113361856216993, 0.2940889145964476]),
    ((0.0, 0.0, -100.0, 0.0), [0.6575289, 0.77087045, 0.48041666], [0.47686363729448533, 0.5673078421631789, 0.33553178456068233]),
    ((0.0, 0.0, -100.0, 0.0), [0.3532253, 0.92266, 0.1541484], [0.23260965329428723, 0.6865892231248671, 0.07389638742854648]),
    ((0.0, 0.0, -100.0, 0.0), [0.21404114, 0.21404114, 0.21404114], [0.21404114365577706, 0.21404114365577706, 0.21404114365577706]),
    ((0.0, 0.0, -100.0, 0.0), [0.024240883, 0.011548564, 0.0049497853], [0.024240883067250255, 0.01154856383800507, 0.004949785303324463]),
    ((0.0, 0.0, 0.0, 100.0), [0.697884, 0.26591676, 0.035229906], [0.6978840231895447, 0.26591676473617554, 0.03522990643978119]),
    ((0.0, 0.0, 0.0, 100.0), [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]),
    ((0.0, 0.0, 0.0, 100.0), [0.9566869, 0.9886377, 0.10440475], [0.9566869139671326, 0.9886376857757568, 0.10440474748611461]),
    ((0.0, 0.0, 0.0, 100.0), [0.65533775, 0.4699709, 0.33762476], [0.6553377509117126, 0.46997091174125677, 0.3376247584819795]),
    ((0.0, 0.0, 0.0, 100.0), [0.6575289, 0.77087045, 0.48041666], [0.6575288772583007, 0.7708704471588134, 0.4804166555404662]),
    ((0.0, 0.0, 0.0, 100.0), [0.3532253, 0.92266, 0.1541484], [0.353225290775299, 0.9226599931716918, 0.1541483998298645]),
    ((0.0, 0.0, 0.0, 100.0), [0.21404114, 0.21404114, 0.21404114], [0.21404114365577706, 0.21404114365577706, 0.21404114365577706]),
    ((0.0, 0.0, 0.0, 100.0), [0.024240883, 0.011548564, 0.0049497853], [0.06377671355677283, 0.03038377132735792, 0.013022670774074947]),
    ((0.0, 0.0, 0.0, -100.0), [0.697884, 0.26591676, 0.035229906], [0.6978840231895447, 0.26591676473617554, 0.03522990643978119]),
    ((0.0, 0.0, 0.0, -100.0), [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]),
    ((0.0, 0.0, 0.0, -100.0), [0.9566869, 0.9886377, 0.10440475], [0.9566869139671326, 0.9886376857757568, 0.10440474748611461]),
    ((0.0, 0.0, 0.0, -100.0), [0.060502265, 0.042496037, 0.8986732], [0.05525626489412054, 0.038811311883765756, 0.8207514776007448]),
    ((0.0, 0.0, 0.0, -100.0), [0.65533775, 0.4699709, 0.33762476], [0.6553377509117126, 0.46997091174125677, 0.3376247584819795]),
    ((0.0, 0.0, 0.0, -100.0), [0.6575289, 0.77087045, 0.48041666], [0.6575288772583007, 0.7708704471588134, 0.4804166555404662]),
    ((0.0, 0.0, 0.0, -100.0), [0.3532253, 0.92266, 0.1541484], [0.353225290775299, 0.9226599931716918, 0.1541483998298645]),
    ((0.0, 0.0, 0.0, -100.0), [0.21404114, 0.21404114, 0.21404114], [0.21404114365577706, 0.21404114365577706, 0.21404114365577706]),
    ((0.0, 0.0, 0.0, -100.0), [0.024240883, 0.011548564, 0.0049497853], [0.009077366152347768, 0.004324534803476778, 0.001853522144764119]),
    ((1.0, 50.0, 30.0, -20.0), [0.21404114, 0.21404114, 0.21404114], [0.45712140813253477, 0.45712140813253477, 0.45712140813253477]),
    ((1.0, 50.0, 30.0, -20.0), [0.024240883, 0.011548564, 0.0049497853], [0.041339673572174385, 0.019694573748245136, 0.008441215120922187]),
    ((-1.0, -50.0, 100.0, 100.0), [0.697884, 0.26591676, 0.035229906], [0.3577375672261883, 0.1363097783305197, 0.018058961954414254]),
    ((-1.0, -50.0, 100.0, 100.0), [0.6274039, 0.06909729, 0.01639144], [0.3388447904464467, 0.03731767606118422, 0.008852596714872304]),
    ((-1.0, -50.0, 100.0, 100.0), [0.9566869, 0.9886377, 0.10440475], [0.9435552862293476, 0.9751799972818763, 0.09997074274391804]),
    ((-1.0, -50.0, 100.0, 100.0), [0.3725961, 0.9309027, 0.98360854], [0.2606974920911366, 0.7407610250116968, 0.7860804678509513]),
    ((-1.0, -50.0, 100.0, 100.0), [0.060502265, 0.042496037, 0.8986732], [0.04339253200864026, 0.03047837374532389, 0.6445329743364289]),
    ((-1.0, -50.0, 100.0, 100.0), [0.65533775, 0.4699709, 0.33762476], [0.35587519515102767, 0.254073970690196, 0.1813910651890341]),
    ((-1.0, -50.0, 100.0, 100.0), [0.6575289, 0.77087045, 0.48041666], [0.45371039310172756, 0.540196257827165, 0.31856400628040804]),
    ((-1.0, -50.0, 100.0, 100.0), [0.3532253, 0.92266, 0.1541484], [0.2224117985453022, 0.6601793492133468, 0.06936632629530537]),
    ((-1.0, -50.0, 100.0, 100.0), [0.21404114, 0.21404114, 0.21404114], [0.1154201179944645, 0.1154201179944645, 0.1154201179944645]),
    ((-1.0, -50.0, 100.0, 100.0), [0.024240883, 0.011548564, 0.0049497853], [0.045469260565530745, 0.021661944280294863, 0.00928444219939981]),
    ((2.0, 100.0, 100.0, -100.0), [0.21404114, 0.21404114, 0.21404114], [0.8963193298880752, 0.8963193298880752, 0.8963193298880752]),
    ((2.0, 100.0, 100.0, -100.0), [0.024240883, 0.011548564, 0.0049497853], [0.0791455228678063, 0.03770560341368091, 0.016160852920587762]),
];
// <<< or:CHAINE-AU-REPOS

// >>> or:TRIPLETS (EVORA-REV-112 : écrit par poser_rev112.py depuis 04-tons.json et texte de la référence ; ne pas retoucher)
/// Evora (EVORA-REV-112) : les (contraste, blancs, noirs) du contrôle « tons croissants et bornés » de la référence (boucle de `tons`, controles.py).
#[rustfmt::skip]
const GOLD_CONTROL_TRIPLETS: [(f64, f64, f64); 5] = [(0.0, 0.0, 0.0), (100.0, 100.0, -100.0), (-100.0, -100.0, 100.0), (100.0, -100.0, 100.0), (-100.0, 100.0, -100.0)];
// <<< or:TRIPLETS

// >>> or:TOLERANCES-BRANCHEMENT (EVORA-REV-112 : écrit par poser_rev112.py depuis tableaux-rust.txt, tolérances du branchement ; ne pas retoucher)
/// Evora : absolue, par canal Rec.2020 d'affichage, chaîne du moteur en f32 face à la référence en f64. Écart mesuré 5.58e-06. Somme du pont aller en f32, de la lecture de table (jusqu'à 1,7e-6) et du retour de gamme (jusqu'à 1,9e-6), amplifiée sur les canaux bien plus grands que la luminance.
#[rustfmt::skip]
const GOLD_TOL_CHAIN: f64 = 2.5e-05;

/// Evora : absolue, o -> to_scene (f32) -> table lue (f32) -> o. Écart mesuré 6.56e-07. L'interpolation de la table (3,4e-7) et l'arithmétique f32 ; l'erreur de to_scene près du blanc ne revient pas (pente nulle au blanc).
#[rustfmt::skip]
const GOLD_TOL_NEUTRAL_TONE: f64 = 2e-06;

/// Evora : absolue, par canal Rec.2020, chaîne du moteur au neutre face à la source. Écart mesuré 2.62e-06. Mesuré sur le treillis de 5 832 couleurs, primaires comprises (canal bien plus grand que la luminance).
#[rustfmt::skip]
const GOLD_TOL_NEUTRAL_LINEAR: f64 = 1e-05;

/// Evora : fraction de code sur 255 : la marge de la trame du moteur. Écart mesuré 1.78e-03. L'écart au neutre reste très en dessous de la marge de 1/128 de code : les octets attendus sont ceux du fichier.
#[rustfmt::skip]
const NEUTRAL_CODE_TOL: f64 = 0.0078125;

/// Evora : plus forte baisse admise entre deux nœuds en f32 (seuil de la boucle de `tons`, controles.py:293-300). Écart mesuré 0.00e+00. Table calculée en f64 puis arrondie : l'arrondi garde l'ordre, un palier (pas nul) est possible près de W, jamais une baisse.
#[rustfmt::skip]
const GOLD_TOL_TONE_STEP: f64 = 1e-07;
// <<< or:TOLERANCES-BRANCHEMENT

// >>> or:SEUILS-CONTROLE (EVORA-REV-112 : écrit par poser_rev112.py depuis texte de controles.py de la référence ; ne pas retoucher)
/// Evora (EVORA-REV-112) : seuils et mire de deux contrôles de la référence, lus dans le texte de controles.py (SHA-256 7880643fcd58).
/// `SEUIL_BLANC` : aplat blanc créé admis, en pour cent des points de l'image.
#[rustfmt::skip]
const CONTROL_WHITE_PATCH_MAX: f64 = 0.35;
/// `BLANC` : plus petit canal sRGB codé (0..1) d'un point blanc.
#[rustfmt::skip]
const CONTROL_WHITE: f64 = 0.998;
/// `QUASI_BLANC` : plus petit canal sRGB codé (0..1) d'un point déjà quasi blanc dans la source, qui sort du compte.
#[rustfmt::skip]
const CONTROL_NEAR_WHITE: f64 = 0.99;
/// Expositions (EV) du contrôle « aucun aplat blanc créé ».
#[rustfmt::skip]
const CONTROL_WHITE_PATCH_EXPOSURES: [f64; 2] = [1.0, 2.0];
/// `SEUIL_MEMOIRE` : chroma des clairs colorés après le rendu du contrôle « mémoire de chroma », rapportée à leur chroma d'origine.
#[rustfmt::skip]
const CONTROL_CHROMA_MEMORY_MIN: f64 = 0.97;
/// Mire du contrôle « mémoire de chroma » : luminosité OkLab des pastilles, leur chroma OkLab, l'écart de teinte (radians) des deux voisines de la teinte de peau.
#[rustfmt::skip]
const CONTROL_MEMORY_LIGHTNESS: f64 = 0.93;
#[rustfmt::skip]
const CONTROL_MEMORY_CHROMA: f64 = 0.03;
#[rustfmt::skip]
const CONTROL_MEMORY_HUE_STEP: f64 = 2.0;
/// Exposition (EV) du contrôle « mémoire de chroma ».
#[rustfmt::skip]
const CONTROL_MEMORY_EXPOSURE: f64 = -1.0;
// <<< or:SEUILS-CONTROLE

// >>> or:APLAT-BLANC (EVORA-REV-112 : écrit par poser_rev112.py depuis la référence jouée sur la mire de tons du test ; ne pas retoucher)
/// Evora (EVORA-REV-112) : le blanc que la référence elle-même crée (evora1.rendre, compté comme son contrôle « aucun aplat blanc créé » : plus petit canal sRGB codé au moins `BLANC`, hors des points déjà quasi blancs dans la source) sur la mire de tons de ce fichier (`TONE_MIRE_TOP` = 255,
/// 8 familles de `TONE_MIRE_FAMILIES`), à chaque exposition du contrôle (`CONTROL_WHITE_PATCH_EXPOSURES`), en pour cent des 2048 points de la mire (1 puis 8 points). controles.py : SHA-256 7880643fcd58.
#[rustfmt::skip]
const GOLD_WHITE_PATCH_PERCENT: [f64; 2] = [0.048828125, 0.390625];
// <<< or:APLAT-BLANC

// >>> or:SEUIL-IDENTITE (EVORA-REV-113 : écrit par poser_rev113.py depuis texte de controles.py de la référence ; ne pas retoucher)
/// Evora (EVORA-REV-113) : seuil du contrôle « identité au neutre » de la référence, lu dans le texte de controles.py (SHA-256 7880643fcd58).
/// `SEUIL_IDENTITE` : écart admis entre le rendu neutre d'un fichier et ce fichier, en niveau (code) sur 255.
#[rustfmt::skip]
const CONTROL_IDENTITY_MAX: f64 = 0.5;
// <<< or:SEUIL-IDENTITE

/// Evora (EVORA-REV-112) : écart admis, en codes sur 16 bits, entre le rendu neutre d'un fichier sur
/// 8 bits et ce fichier (un code sur 8 bits en vaut `DEEP_PER_CODE` sur 16), hors des points dont un
/// canal touche le blanc (code 255). L'arrondi du rendu en 16 bits tombe à un demi-code près. Mesuré
/// hors moteur à EVORA-REV-113 (`mesures_constats_110_113.py`) : 0 code, y compris quand les
/// puissances, racines cubiques, hypoténuses et logarithmes bougent de deux derniers chiffres.
const NEUTRAL_DEEP_TOL: i32 = 1;
/// Evora (EVORA-REV-113) : le même écart aux points dont un canal touche le blanc, où le retour de
/// gamme s'éveille sur le bruit d'arrondi du pont. Mesuré hors moteur : 0 code ; 2 quand les
/// puissances, racines cubiques, hypoténuses et logarithmes bougent d'un dernier chiffre, 4 quand ils
/// bougent de deux (40 tirages chacun). Deux fois le pire : 8 codes sur 16 bits font 0,031 code sur
/// 8 bits, sous le seuil d'identité de la référence (`CONTROL_IDENTITY_MAX`).
const NEUTRAL_EDGE_DEEP_TOL: i32 = 8;
/// Evora (EVORA-REV-113) : écart admis, en code sur 255, entre le rendu neutre en flottants sRGB et le
/// fichier, aux points dont un canal touche le blanc (`NEUTRAL_CODE_TOL` ailleurs, où le pire mesuré
/// sous perturbation est 0,0012). Mesuré hors moteur : 0,0019 code ; jusqu'à 0,0090 quand les
/// puissances, racines cubiques, hypoténuses et logarithmes bougent d'un dernier chiffre, 0,0102 quand
/// ils bougent de deux (40 tirages chacun). Deux fois et demie le pire, sous le seuil d'identité de la
/// référence (`CONTROL_IDENTITY_MAX`).
const NEUTRAL_EDGE_CODE_TOL: f64 = 0.025;
/// Codes sur 16 bits par code sur 8 bits (65 535 / 255).
const DEEP_PER_CODE: i32 = 257;
/// Evora (EVORA-REV-112) : le treillis de couleurs du test du neutre : `LATTICE_LEVELS` niveaux par
/// canal, de `LATTICE_STEP` codes en `LATTICE_STEP` codes, du noir (0) au blanc (255).
const LATTICE_LEVELS: usize = 18;
const LATTICE_STEP: usize = 15;
const _: () = assert!((LATTICE_LEVELS - 1) * LATTICE_STEP == 255, "le treillis doit finir au code 255");
/// Evora (EVORA-REV-112) : luminances d'affichage où les deux témoins du test du neutre sont lus
/// (une ombre, un ton moyen, un clair ; le gris moyen, que le pont laisse en place, n'y est pas).
const WITNESS_LUMINANCES: [f32; 3] = [0.02, 0.5, 0.9];
/// Evora (EVORA-REV-112) : écart qu'un témoin du test du neutre doit dépasser, en lumière linéaire :
/// un code entier sur 255, soit des milliers de fois la tolérance du neutre.
const WITNESS_GAP: f64 = 1.0 / 255.0;
/// Evora (EVORA-REV-112) : la mire de tons du contrôle du blanc créé. Son plus haut code : le blanc.
/// La part de blanc créé sur une mire dépend de la part de clairs qu'on y met : le contrôle ne la juge
/// pas au seuil de la référence, fait pour sa photo de démonstration, mais au blanc que la référence
/// elle-même crée sur cette mire (`GOLD_WHITE_PATCH_PERCENT`, que l'outil recalcule depuis cette
/// constante et `TONE_MIRE_FAMILIES`).
const TONE_MIRE_TOP: usize = 255;
/// Les familles de couleur de la mire de tons, une par ligne, en huitièmes du ton pour chaque canal :
/// neutre, chaud, froid, rouge, vert, bleu, jaune, peau.
const TONE_MIRE_FAMILIES: [[usize; 3]; 8] = [[8, 8, 8], [8, 6, 4], [4, 6, 8], [8, 2, 2], [2, 8, 2], [2, 2, 8], [8, 8, 2], [8, 5, 4]];
/// Evora (EVORA-REV-112) : la plage de luminances de scène où la table branchée est lue, en EV autour
/// du gris (elle déborde la table des deux côtés), et son nombre de pas.
const WIRED_READ_FROM_EV: f64 = -16.0;
const WIRED_READ_TO_EV: f64 = 10.5;
const WIRED_READ_STEPS: usize = 20_000;
/// Evora (EVORA-REV-112) : contraste local et atténuation de la brume de la retouche du test du
/// cache. Leurs plans larges ne sont clés que par leur rayon : sans la marque d'Evora dans la clé de
/// l'image, un brouillon les reprendrait d'une image de l'autre version.
const CACHE_LOCAL_CONTRAST: f64 = 40.0;
const CACHE_HAZE_REDUCTION: f64 = 30.0;

/// La rampe sRGB sur 8 bits du test de l'amont `tests::unedited_rendered_source_is_passthrough`
/// (256 × 4 points) : trois canaux qui montent, tournent et descendent.
fn srgb_ramp() -> Rgba8 {
    Rgba8::from_fn(256, 4, |x, y| [x as u8, (x as u8).wrapping_add(y as u8 * 40), 255 - x as u8, 255])
}

/// Evora (EVORA-REV-113) : un point d'un fichier sur 8 bits dont un canal touche le blanc (code 255).
fn touches_white(p: &[u8; 4]) -> bool {
    p[..3].contains(&u8::MAX)
}

/// Un treillis de `LATTICE_LEVELS`³ couleurs sRGB sur 8 bits, primaires, noir et blanc compris.
fn srgb_lattice() -> Rgba8 {
    let n = LATTICE_LEVELS;
    Rgba8::from_fn(n * n, n, |x, y| [((x / n) * LATTICE_STEP) as u8, ((x % n) * LATTICE_STEP) as u8, (y * LATTICE_STEP) as u8, 255])
}

/// La mire de tons : 256 tons du noir à `TONE_MIRE_TOP`, une ligne par famille de couleur.
fn tone_mire() -> Rgba8 {
    Rgba8::from_fn(256, TONE_MIRE_FAMILIES.len(), |x, y| {
        let tone = x * TONE_MIRE_TOP / 255;
        let [r, g, b] = TONE_MIRE_FAMILIES[y].map(|eighths| (tone * eighths / 8) as u8);
        [r, g, b, 255]
    })
}

/// Un fichier sRGB sur 8 bits → la source du rendu (Rec.2020 linéaire, lumière d'affichage), comme
/// l'amont décode un fichier rendu.
fn decoded(file: &Rgba8) -> Rgb32f {
    let to_working = evolight_color::SRGB.to_space(&evolight_color::REC2020);
    file.to_linear().map(|c| to_working.apply_f32(c))
}

/// La sortie en flottants linéaires dans l'espace de travail (Rec.2020) : ce que rend la référence.
fn working_request(w: usize, h: usize) -> RenderRequest {
    RenderRequest { depth: OutputDepth::F32Linear, space: OutputSpace::Rec2020, ..RenderRequest::fit(w, h) }
}

/// Les points d'un rendu en flottants, trois canaux par point.
fn float_points(r: Rendered) -> Vec<[f32; 3]> {
    let Some(deep) = r.deep else {
        panic!("rendu en flottants sans échantillons");
    };
    let DeepSamples::F32(v) = deep.samples else {
        panic!("échantillons flottants attendus");
    };
    assert_eq!(v.len(), 3 * deep.width * deep.height, "trois canaux par point");
    v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
}

/// Les réglages par défaut sous `process`, au cran `exposure`, avec ce contraste, ces blancs et ces
/// noirs : les quatre curseurs que le rendu d'Evora reprend à EVORA-REV-112.
fn toned(process: Process, exposure: f64, (contrast, whites, blacks): (f64, f64, f64)) -> DevelopSettings {
    let mut s = at(process, exposure);
    (s.light.contrast, s.light.whites, s.light.blacks) = (contrast, whites, blacks);
    s
}

/// Le plus grand écart entre deux séries de points, canal par canal.
fn worst_gap(a: &[[f32; 3]], b: &[[f32; 3]]) -> f64 {
    assert_eq!(a.len(), b.len(), "deux séries de même longueur");
    let mut worst = 0.0f64;
    for (p, q) in a.iter().zip(b) {
        for c in 0..3 {
            let gap = (f64::from(p[c]) - f64::from(q[c])).abs();
            // un écart qui n'est pas un nombre est le pire des écarts
            worst = if gap.is_nan() { f64::INFINITY } else { worst.max(gap) };
        }
    }
    worst
}

/// Evora (EVORA-REV-112) : aux réglages neutres, sous `Evora1`, le rendu d'un fichier rendu est ce
/// fichier. Le pont aller de l'étage linéaire et le pont de retour de la table de tons se défont.
/// Deux fichiers sRGB sur 8 bits : la rampe du test de l'amont, et un treillis de 5 832 couleurs,
/// primaires, noir et blanc compris. Quatre lectures de chacun :
/// - en 8 bits, les octets du rendu sont ceux du fichier. La trame du moteur laisse `NEUTRAL_CODE_TOL`
///   de code de marge de chaque côté d'un code exact : c'est la tolérance du neutre ;
/// - en flottants sRGB, l'écart au code du fichier tient sous cette même tolérance, sans trame ni
///   table d'encodage, hors des points dont un canal touche le blanc (code 255), qui ont la leur
///   (`NEUTRAL_EDGE_CODE_TOL`) ;
/// - en 16 bits, au plus `NEUTRAL_DEEP_TOL` code d'écart, et `NEUTRAL_EDGE_DEEP_TOL` aux points dont
///   un canal touche le blanc ;
/// - en flottants dans l'espace de travail, l'écart à la source tient sous `GOLD_TOL_NEUTRAL_LINEAR`.
///
/// Evora (EVORA-REV-113) : aux points dont un canal touche le blanc, le retour de gamme s'éveille sur
/// le bruit d'arrondi du pont (la couleur égale sa source à ce bruit près, et le test `mx > ref` est sans
/// marge, comme dans la référence) ; il y ramène la couleur de quelques millièmes de code, un peu plus
/// quand les fonctions mathématiques du runner s'écartent de celles du poste. Leurs tolérances sont
/// mesurées sous cette perturbation et restent sous le seuil d'identité de la référence
/// (`CONTROL_IDENTITY_MAX`).
///
/// Sabotages qui le font rougir : le pont aller non posé à l'étage linéaire, ou posé deux fois ; le
/// bras de `base_tone` resté sur la table de V6 ; un pont de retour d'un autre exposant ou d'un autre
/// blanc que le pont aller ; l'échelle de chroma rendue à l'aller et non reprise au retour. Témoins
/// intégrés, sur la luminance seule : le pont aller suivi de la table de V6, puis la table d'Evora
/// sans pont aller, manquent tous deux le fichier de plus d'un code.
#[test]
fn neutral_rendered_file_is_the_file() {
    let info = SourceInfo::default();
    let neutral = at(Process::Evora1, 0.0);
    assert!(crate::evora1::renders(&neutral, &info), "garde de portée : ce rendu est un rendu d'Evora");
    // les tolérances des points du bord de la gamme restent sous le seuil d'identité de la référence
    assert!(NEUTRAL_EDGE_CODE_TOL <= CONTROL_IDENTITY_MAX, "tolérance en flottants du bord de la gamme au-dessus du seuil d'identité");
    assert!(
        f64::from(NEUTRAL_EDGE_DEEP_TOL) / f64::from(DEEP_PER_CODE) <= CONTROL_IDENTITY_MAX,
        "tolérance sur 16 bits du bord de la gamme au-dessus du seuil d'identité"
    );
    for (what, file) in [("rampe", srgb_ramp()), ("treillis", srgb_lattice())] {
        let (w, h) = (file.width, file.height);
        let src = decoded(&file);

        // 8 bits : les octets du fichier
        let out = render(&src, &info, &neutral, &RenderRequest::fit(w, h)).image;
        assert_eq!((out.width, out.height), (w, h), "{what} : taille du rendu");
        let mut worst = 0i32;
        for (a, b) in file.data.iter().zip(&out.data) {
            for c in 0..3 {
                worst = worst.max((i32::from(a[c]) - i32::from(b[c])).abs());
            }
        }
        assert_eq!(worst, 0, "{what}, 8 bits : le rendu neutre s'écarte du fichier de {worst} code(s)");

        // flottants sRGB : l'écart au code du fichier, sans trame ni table d'encodage ; à part, les
        // points dont un canal touche le blanc (EVORA-REV-113)
        let srgb_floats = RenderRequest { depth: OutputDepth::F32Linear, ..RenderRequest::fit(w, h) };
        let float = float_points(render(&src, &info, &neutral, &srgb_floats));
        assert_eq!(float.len(), file.data.len(), "{what} : un point de rendu par point du fichier");
        let (mut worst_code, mut worst_edge_code) = (0.0f64, 0.0f64);
        for (a, b) in file.data.iter().zip(&float) {
            let worst = if touches_white(a) { &mut worst_edge_code } else { &mut worst_code };
            for c in 0..3 {
                let gap = (f64::from(linear_to_srgb(b[c])) * 255.0 - f64::from(a[c])).abs();
                *worst = if gap.is_nan() { f64::INFINITY } else { (*worst).max(gap) };
            }
        }
        assert!(worst_code <= NEUTRAL_CODE_TOL, "{what}, flottants sRGB : {worst_code} code d'écart au fichier, {NEUTRAL_CODE_TOL} admis");
        assert!(
            worst_edge_code <= NEUTRAL_EDGE_CODE_TOL,
            "{what}, flottants sRGB, un canal au blanc : {worst_edge_code} code d'écart au fichier, {NEUTRAL_EDGE_CODE_TOL} admis"
        );

        // 16 bits : au plus un code, et `NEUTRAL_EDGE_DEEP_TOL` aux points dont un canal touche le blanc
        let (_, deep) = bits(render(&src, &info, &neutral, &RenderRequest { depth: OutputDepth::U16, ..RenderRequest::fit(w, h) }));
        assert_eq!(deep.len(), 3 * file.data.len(), "{what} : trois canaux de 16 bits par point");
        let (mut worst_deep, mut worst_edge_deep) = (0i32, 0i32);
        for (a, b) in file.data.iter().zip(deep.chunks_exact(3)) {
            let worst = if touches_white(a) { &mut worst_edge_deep } else { &mut worst_deep };
            for c in 0..3 {
                *worst = (*worst).max((i32::from(a[c]) * DEEP_PER_CODE - i32::from(b[c])).abs());
            }
        }
        assert!(worst_deep <= NEUTRAL_DEEP_TOL, "{what}, 16 bits : {worst_deep} code(s) d'écart au fichier, {NEUTRAL_DEEP_TOL} admis");
        assert!(
            worst_edge_deep <= NEUTRAL_EDGE_DEEP_TOL,
            "{what}, 16 bits, un canal au blanc : {worst_edge_deep} code(s) d'écart au fichier, {NEUTRAL_EDGE_DEEP_TOL} admis"
        );

        // flottants dans l'espace de travail : la source, à la tolérance du neutre
        let linear = float_points(render(&src, &info, &neutral, &working_request(w, h)));
        let gap = worst_gap(&linear, &src.data);
        assert!(gap <= GOLD_TOL_NEUTRAL_LINEAR, "{what}, flottants Rec.2020 : écart {gap} à la source, {GOLD_TOL_NEUTRAL_LINEAR} admis");
    }

    // témoins rouges, sur la luminance seule : chaque moitié du branchement, seule, manque le fichier
    let bridge = Bridge::default();
    let (evora_table, v6_table) = (crate::evora1::tone::table(&bridge, 0.0, 0.0, 0.0), ToneMap::display(0.0, 0.0, 0.0));
    for o in WITNESS_LUMINANCES {
        let whole = f64::from(evora_table.apply(bridge.to_scene(o)).min(1.0));
        assert!((whole - f64::from(o)).abs() <= GOLD_TOL_NEUTRAL_TONE, "luminance {o} : pont aller puis table d'Evora rendent {whole}");
        let no_return = f64::from(v6_table.apply(bridge.to_scene(o)));
        assert!((no_return - f64::from(o)).abs() > WITNESS_GAP, "témoin : à {o}, le pont aller suivi de la table de V6 rend {no_return}");
        let no_bridge = f64::from(evora_table.apply(o));
        assert!((no_bridge - f64::from(o)).abs() > WITNESS_GAP, "témoin : à {o}, la table d'Evora sans pont aller rend {no_bridge}");
    }
}

/// Evora (EVORA-REV-112) : exposition, contraste, blancs et noirs, rendus sous `Evora1` par le
/// rendu entier, face aux vecteurs d'or de la chaîne par point de la référence. Dix couleurs sous
/// quinze jeux de réglages ; n'entrent ici que les vecteurs où le retour de gamme de la référence
/// laisse la couleur telle quelle et où le rendu tient dans 0..1 (`GOLD_CHAIN_AT_REST`) : ils gardent
/// que le retour de gamme du moteur n'y fait rien (ceux où il travaille sont lus par
/// `pushed::render_matches_the_reference_through_the_gamut_return`), et hors de 0..1 la mise en
/// gamme finale du moteur ramènerait un canal négatif. La sortie est lue en flottants dans l'espace
/// de travail, sans passer par un espace de sortie.
///
/// Sabotages qui le font rougir : l'exposition posée avant le pont aller (en lumière d'affichage) ;
/// la table de tons bâtie sur d'autres curseurs que ceux de la photo ; la luminance d'affichage non
/// reprise par l'échelle de chroma ; une constante du pont ou des tons changée. Témoin intégré : le
/// même rendu sous V6 manque la tolérance sur chaque jeu de réglages qui n'est pas neutre.
#[test]
fn render_matches_the_reference_where_the_gamut_return_rests() {
    let info = SourceInfo::default();
    let chain = &GOLD_CHAIN_AT_REST;
    let (mut start, mut groups, mut witnessed) = (0usize, 0usize, 0usize);
    while start < chain.len() {
        // un jeu de réglages : les vecteurs qui se suivent sous les mêmes quatre curseurs
        let settings = chain[start].0;
        let group: Vec<_> = chain[start..].iter().take_while(|v| v.0 == settings).collect();
        start += group.len();
        groups += 1;
        let (exposure, contrast, whites, blacks) = settings;
        let src = Rgb32f::from_fn(group.len(), 1, |x, _| group[x].1);
        let want: Vec<[f32; 3]> = group.iter().map(|v| v.2.map(|c| c as f32)).collect();
        let req = working_request(group.len(), 1);
        let rendered = |process: Process| float_points(render(&src, &info, &toned(process, exposure, (contrast, whites, blacks)), &req));

        let got = rendered(Process::Evora1);
        assert_eq!(got.len(), group.len(), "réglages {settings:?} : un point de rendu par vecteur");
        for (i, (out, vector)) in got.iter().zip(&group).enumerate() {
            for c in 0..3 {
                let gap = (f64::from(out[c]) - vector.2[c]).abs();
                assert!(
                    gap <= GOLD_TOL_CHAIN,
                    "réglages {settings:?}, source {:?} (vecteur {i} du jeu), canal {c} : rendu {}, référence {}, écart {gap}",
                    vector.1,
                    out[c],
                    vector.2[c]
                );
            }
        }

        // témoin rouge : hors du neutre, le rendu de V6 du même fichier manque la référence
        if settings != (0.0, 0.0, 0.0, 0.0) {
            let v6_gap = worst_gap(&rendered(Process::V6), &want);
            assert!(v6_gap > GOLD_TOL_CHAIN, "témoin : sous {settings:?}, le rendu de V6 passe la tolérance (écart {v6_gap})");
            witnessed += 1;
        }
    }
    assert!(groups >= 2 && witnessed >= 1, "garde de portée : {groups} jeu(x) de réglages lus, {witnessed} témoin(s)");
}

/// Evora (EVORA-REV-112) : la table de tons que reçoit le noyau (`finish::base_tone`) est, sous le
/// prédicat, celle d'Evora, nœud pour nœud et avec ses nœuds de chroma ; elle est croissante et
/// bornée, sur les cinq triplets du contrôle « tons croissants et bornés » de la référence : des
/// nœuds finis, entre 0 et `W`, sans baisse ; lue puis bornée par le noyau, de 0 à 1, sans baisse.
/// Hors du prédicat (un brut, un rendu HDR), la table d'`Evora1` reste celle de V6.
///
/// Sabotages qui le font rougir : le bras d'`Evora1` de `base_tone` branché sans le prédicat (un brut
/// ou un rendu HDR lirait la table d'Evora), ou sur d'autres curseurs ; la table bornée à 1 avant
/// d'être stockée. Témoin intégré : la table de V6 du même fichier n'est pas celle d'Evora, et elle
/// est bornée dès le stockage (son dernier nœud ne passe pas 1, celui d'Evora si).
#[test]
fn wired_tone_table_is_increasing_and_bounded() {
    let (file, raw) = (SourceInfo::default(), SourceInfo { raw: true, ..Default::default() });
    let bridge = Bridge::default();
    for triplet in GOLD_CONTROL_TRIPLETS {
        let (contrast, whites, blacks) = triplet;
        let (evora, v6) = (toned(Process::Evora1, 0.0, triplet), toned(Process::V6, 0.0, triplet));
        let Some(kernel) = crate::evora1::Kernel::for_render(&evora, &file) else {
            panic!("{triplet:?} : fichier rendu sous Evora1 sans part d'Evora dans le noyau");
        };

        // sous le prédicat : la table d'Evora
        let wired = crate::finish::base_tone(&evora, &file, evora.hdr.peak());
        let law = crate::evora1::tone::table(&bridge, contrast, whites, blacks);
        assert!(wired.lut() == law.lut(), "{triplet:?} : la table du noyau n'est pas celle d'Evora");
        assert!(wired.chroma_lut() == law.chroma_lut(), "{triplet:?} : la table du noyau ne porte pas les nœuds de chroma du pont");

        // croissante et bornée, nœud par nœud
        let lut = wired.lut();
        assert!(lut.len() == LUT_N && lut.iter().all(|v| v.is_finite()), "{triplet:?} : table incomplète ou non finie");
        assert!(lut.iter().all(|v| *v > 0.0 && f64::from(*v) < bridge.w()), "{triplet:?} : un nœud sort de 0..W");
        let node_drop = lut.windows(2).fold(0.0f64, |worst, pair| worst.max(f64::from(pair[0]) - f64::from(pair[1])));
        assert!(node_drop <= GOLD_TOL_TONE_STEP, "{triplet:?} : la table baisse de {node_drop} entre deux nœuds");

        // lue puis bornée par le noyau : de 0 à 1, sans baisse
        let mut previous = 0.0f32;
        for i in 0..=WIRED_READ_STEPS {
            let ev = WIRED_READ_FROM_EV + (WIRED_READ_TO_EV - WIRED_READ_FROM_EV) * i as f64 / WIRED_READ_STEPS as f64;
            let y = (GREY * ev.exp2()) as f32;
            let o = kernel.bound(wired.apply(y));
            assert!((0.0..=1.0).contains(&o), "{triplet:?} : à {ev} EV, le noyau lit {o}, hors de 0..1");
            let fall = f64::from(previous) - f64::from(o);
            assert!(fall <= GOLD_TOL_TONE_STEP, "{triplet:?} : à {ev} EV, la lecture baisse de {fall}");
            previous = o;
        }

        // hors du prédicat : la table de V6, pour un brut et pour un rendu HDR
        assert!(
            crate::finish::base_tone(&evora, &raw, 1.0).lut() == crate::finish::base_tone(&v6, &raw, 1.0).lut(),
            "{triplet:?}, brut : table d'Evora1 autre que celle de V6"
        );
        let (evora_hdr, v6_hdr) = (hdr_on(evora.clone()), hdr_on(v6.clone()));
        let peak = evora_hdr.hdr.peak();
        assert!(peak > 1.0, "garde de portée : le rendu HDR de ce test a une marge au-dessus du blanc SDR");
        assert!(
            crate::finish::base_tone(&evora_hdr, &file, peak).lut() == crate::finish::base_tone(&v6_hdr, &file, peak).lut(),
            "{triplet:?}, rendu HDR : table d'Evora1 autre que celle de V6"
        );

        // témoin rouge : la table de V6 du même fichier, bornée dès le stockage
        let v6_table = crate::finish::base_tone(&v6, &file, 1.0);
        assert!(v6_table.lut() != lut, "témoin : sous {triplet:?}, la table de V6 passe pour celle d'Evora");
        let (v6_top, evora_top) = (v6_table.lut()[LUT_N - 1], lut[LUT_N - 1]);
        assert!(v6_top <= 1.0 && evora_top > 1.0, "témoin : sous {triplet:?}, dernier nœud de V6 {v6_top}, d'Evora {evora_top}");
    }
}

/// Part des points de `file`, en pour cent, que le rendu sur 16 bits `deep` a faits blancs : un point
/// est blanc quand son plus petit canal codé atteint `CONTROL_WHITE` ; ceux dont le plus petit canal
/// atteignait déjà `CONTROL_NEAR_WHITE` dans le fichier sortent du compte (contrôle « aucun aplat
/// blanc créé » de la référence).
fn created_white_percent(file: &Rgba8, deep: &[u16]) -> f64 {
    assert_eq!(deep.len(), 3 * file.data.len(), "trois canaux de 16 bits par point");
    let mut created = 0usize;
    for (source, out) in file.data.iter().zip(deep.chunks_exact(3)) {
        let already = f64::from(source[0].min(source[1]).min(source[2])) >= CONTROL_NEAR_WHITE * 255.0;
        let white = f64::from(out[0].min(out[1]).min(out[2])) >= CONTROL_WHITE * 65535.0;
        if white && !already {
            created += 1;
        }
    }
    100.0 * created as f64 / file.data.len() as f64
}

/// Evora (EVORA-REV-112) : monter l'exposition ne fabrique pas plus d'aplat blanc que la référence
/// (contrôle « aucun aplat blanc créé » de la référence, à ses deux expositions). Sur la mire de tons
/// (huit familles de couleur, du noir au blanc), la part des points devenus blancs sous `Evora1` ne
/// dépasse pas celle que la référence elle-même crée sur la même mire (`GOLD_WHITE_PATCH_PERCENT`) :
/// l'exposition est un gain en lumière de scène, et le pont de retour tend vers le blanc sans couper.
/// Mesuré hors moteur : le moteur crée les mêmes points blancs que la référence (1 puis 8 sur
/// 2 048), avec ou sans retour de gamme, et ce compte ne bouge pas quand les puissances, racines
/// cubiques, hypoténuses et logarithmes bougent d'un ou de deux derniers chiffres (40 tirages
/// chacun) : la lecture se fait sans tolérance.
///
/// Sabotages qui le font rougir : l'exposition posée en lumière d'affichage ; une coupure nette à la
/// place du pont de retour ; le blanc de scène du pont de retour plus bas que celui du pont aller.
/// Témoin intégré : sous V6, où l'exposition d'un fichier rendu est un gain en lumière d'affichage
/// coupé au blanc, la même mire passe le seuil de la référence (`CONTROL_WHITE_PATCH_MAX`) aux deux
/// expositions.
#[test]
fn exposure_creates_no_white_patch() {
    let info = SourceInfo::default();
    let file = tone_mire();
    let src = decoded(&file);
    let req = RenderRequest { depth: OutputDepth::U16, ..RenderRequest::fit(file.width, file.height) };
    let created = |process: Process, exposure: f64| {
        let (_, deep) = bits(render(&src, &info, &at(process, exposure), &req));
        created_white_percent(&file, &deep)
    };
    // garde de portée : aux réglages neutres, la mire n'a aucun point blanc
    assert_eq!(created(Process::Evora1, 0.0), 0.0, "la mire de tons a des points blancs au neutre");
    assert_eq!(CONTROL_WHITE_PATCH_EXPOSURES.len(), GOLD_WHITE_PATCH_PERCENT.len(), "un blanc de la référence par exposition du contrôle");
    for (exposure, reference) in CONTROL_WHITE_PATCH_EXPOSURES.into_iter().zip(GOLD_WHITE_PATCH_PERCENT) {
        let evora = created(Process::Evora1, exposure);
        assert!(evora <= reference, "exposition {exposure} : {evora} % de blanc créé sous Evora1, {reference} % par la référence");
        let v6 = created(Process::V6, exposure);
        assert!(v6 > CONTROL_WHITE_PATCH_MAX, "témoin : exposition {exposure}, {v6} % de blanc créé sous V6, sous le seuil de la référence");
    }
}

/// La chroma OkLab moyenne d'une série de points (Rec.2020 linéaire, canaux négatifs ramenés à 0).
fn mean_chroma(points: &[[f32; 3]]) -> f64 {
    let mut sum = 0.0f64;
    for &p in points {
        let lab = oklab_from_2020(p.map(|v| v.max(0.0)));
        sum += f64::from(lab[1]).hypot(f64::from(lab[2]));
    }
    sum / points.len() as f64
}

/// Evora (EVORA-REV-112) : baisser l'exposition rend leur couleur aux clairs (contrôle « mémoire de
/// chroma » de la référence). Trois pastilles claires et peu colorées, à la teinte de peau du moteur
/// et de part et d'autre : après le rendu sous `Evora1` à l'exposition du contrôle, leur chroma OkLab
/// moyenne vaut au moins `CONTROL_CHROMA_MEMORY_MIN` fois celle de la mire. Le pont aller rend aux
/// clairs la chroma que leur a prise le rendu d'origine, et le pont de retour n'en reprend que ce
/// que demande la luminance d'arrivée, plus basse.
///
/// Sabotages qui le font rougir : l'échelle de chroma oubliée à l'aller, ou reprise à la luminance
/// de départ. Témoin intégré : le même chemin sans échelle de chroma (ni rendue à l'aller ni reprise
/// au retour, la luminance seule passant le pont) reste sous le seuil.
#[test]
fn chroma_memory_returns_colour_to_highlights() {
    let info = SourceInfo::default();
    let skin = crate::colorops::ColorOps::new(&DevelopSettings::default()).skin;
    let step = CONTROL_MEMORY_HUE_STEP as f32;
    let hues = [skin, skin + step, skin - step];
    let (lightness, chroma) = (CONTROL_MEMORY_LIGHTNESS as f32, CONTROL_MEMORY_CHROMA as f32);
    let mire = Rgb32f::from_fn(hues.len(), 1, |x, _| {
        let (sin, cos) = hues[x].sin_cos();
        oklab_to_2020([lightness, chroma * cos, chroma * sin]).map(|v| v.max(0.0))
    });
    let before = mean_chroma(&mire.data);
    assert!(before > 0.5 * CONTROL_MEMORY_CHROMA, "garde de portée : la mire a perdu sa chroma ({before})");
    assert!(mire.data.iter().all(|p| luminance_2020(*p) < 1.0), "garde de portée : une pastille passe le blanc d'affichage");

    let s = at(Process::Evora1, CONTROL_MEMORY_EXPOSURE);
    let out = float_points(render(&mire, &info, &s, &working_request(hues.len(), 1)));
    let kept = mean_chroma(&out) / before;
    assert!(kept >= CONTROL_CHROMA_MEMORY_MIN, "chroma des clairs multipliée par {kept} sous Evora1, {CONTROL_CHROMA_MEMORY_MIN} au moins attendu");

    // témoin rouge : sans échelle de chroma, la luminance seule passe le pont, l'exposition et la table
    let bridge = Bridge::default();
    let table = crate::evora1::tone::table(&bridge, 0.0, 0.0, 0.0);
    let gain = (CONTROL_MEMORY_EXPOSURE as f32).exp2();
    let flat: Vec<[f32; 3]> = mire
        .data
        .iter()
        .map(|&c| {
            let o = luminance_2020(c);
            let back = table.apply(bridge.to_scene(o) * gain).min(1.0);
            c.map(|v| v * back / o)
        })
        .collect();
    let witness = mean_chroma(&flat) / before;
    assert!(witness < CONTROL_CHROMA_MEMORY_MIN, "témoin : sans échelle de chroma, la chroma des clairs est multipliée par {witness}");
}

/// La retouche du test du cache : celle de R12 au cran +1, plus un contraste local et une atténuation
/// de la brume (`CACHE_LOCAL_CONTRAST`, `CACHE_HAZE_REDUCTION`).
fn cache_retouch(process: Process) -> DevelopSettings {
    let mut s = retouched(process, 1.0);
    (s.effects.clarity, s.effects.dehaze) = (CACHE_LOCAL_CONTRAST, CACHE_HAZE_REDUCTION);
    s
}

/// Evora (EVORA-REV-112) : passer une photo de V6 à `Evora1`, puis revenir, refait l'étage linéaire.
/// Avec un même cache de rendu, une suite de rendus qui alterne les deux versions, en plein puis en
/// brouillon, rend à chaque pas ce que rend le même rendu sans cache : l'image de scène d'Evora et
/// l'image de V6 ne se prennent jamais l'une pour l'autre, ni leurs plans.
///
/// Les clés le disent d'abord : pour un fichier rendu, les trois clés de l'étage linéaire diffèrent
/// d'une version à l'autre sous le même cadre ; pour un brut, hors du prédicat, elles sont égales.
///
/// Sabotages qui le font rougir : la marque d'Evora absente de la clé de l'image (le rendu de V6
/// reprendrait l'image de scène du cache, ou l'inverse) ; la marque posée sur `lin_key` seulement
/// (un brouillon reprendrait le plan du contraste local et celui de l'atténuation de la brume d'une
/// image de l'autre version) ; la marque posée sans le prédicat (les clés d'un brut changeraient).
#[test]
fn version_switch_recomputes_the_linear_stage() {
    let (file, raw) = (SourceInfo::default(), SourceInfo { raw: true, ..Default::default() });
    let src = Arc::new(retouch_scene());
    let full = RenderRequest::fit(RETOUCH_BOX, RETOUCH_BOX);
    let draft = RenderRequest { quality: Quality::Draft, ..full };
    let (v6, evora) = (cache_retouch(Process::V6), cache_retouch(Process::Evora1));

    // les clés de l'étage linéaire
    {
        let (a, b) = (plan(&src, &file, &v6, &full), plan(&src, &file, &evora, &full));
        assert!(a.evora1.is_none() && b.evora1.is_some(), "fichier rendu : la part d'Evora du plan suit la version");
        assert_eq!(a.geo, b.geo, "fichier rendu : les deux versions partagent le cadre, donc l'entrée du cache");
        assert!(a.pre_nr_key != b.pre_nr_key, "fichier rendu : clé de l'image avant réduction du bruit sans marque d'Evora");
        assert!(a.lin_key != b.lin_key, "fichier rendu : clé de l'image de l'étage linéaire sans marque d'Evora");
        assert!(a.noise_key != b.noise_key, "fichier rendu : clé du modèle de bruit sans marque d'Evora");
        let (a, b) = (plan(&src, &raw, &v6, &full), plan(&src, &raw, &evora, &full));
        assert!(a.evora1.is_none() && b.evora1.is_none(), "brut : aucune part d'Evora dans le plan");
        assert!(
            (a.pre_nr_key, a.lin_key, a.noise_key) == (b.pre_nr_key, b.lin_key, b.noise_key),
            "brut : les clés d'Evora1 ne sont pas celles de V6"
        );
    }

    // garde de portée : les deux versions ne rendent pas la même image
    assert!(
        bits(retouch_render(&src, &file, &v6, &full)) != bits(retouch_render(&src, &file, &evora, &full)),
        "garde de portée : V6 et Evora1 rendent la même image, le cache n'aurait rien à mêler"
    );

    let cache = StageCache::default();
    let sequence = [
        (Process::V6, full),
        (Process::Evora1, full),
        (Process::Evora1, draft),
        (Process::V6, draft),
        (Process::Evora1, draft),
        (Process::V6, full),
        (Process::Evora1, full),
    ];
    for (i, (process, req)) in sequence.iter().enumerate() {
        let s = cache_retouch(*process);
        let cached = bits(render_cached(&src, &file, &s, req, &cache));
        let alone = bits(retouch_render(&src, &file, &s, req));
        assert!(cached == alone, "pas {i}, {process:?}, {:?} : le rendu avec cache diffère du rendu sans cache", req.quality);
    }
}
