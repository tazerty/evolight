//! Evora (EVORA-REV-113) : le retour de gamme du rendu d'Evora.
//!
//! Les tons d'Evora peuvent pousser une couleur vive hors de la gamme de sortie par le haut : un de
//! ses canaux de sortie passe le blanc. La mise en gamme du moteur ([`crate::finish::gamut_map_to`])
//! désature alors vers le gris en lumière linéaire, et la couleur vire : un orange vif y tourne au
//! saumon. Le retour de gamme ramène la couleur avant elle, sans la laisser virer, en deux temps,
//! par point :
//!
//! 1. une part de l'excès ([`GAMUT_KEPT`]) est rendue en luminance, par un gain commun aux trois
//!    canaux : teinte et richesse gardées ;
//! 2. le reste est désaturé comme le fait le moteur, puis la teinte OkLab d'avant la désaturation est
//!    rendue à la couleur ramenée, qui garde sa luminosité et sa chroma.
//!
//! L'excès se juge contre celui que la source avait déjà au même point, et au moins contre le blanc
//! ([`reference_of`]) : une couleur que les tons n'ont pas poussée plus loin que sa source passe
//! telle quelle. Le test est strict (`mx > ref`, pont.py:93), mais la loi est continue à son seuil
//! (EVORA-REV-114, loi de la référence du 2026-11-13) : le gain y vaut 1, et la teinte d'avant la
//! désaturation n'est rendue qu'en proportion de l'excès relatif, de rien à la référence (la couleur
//! reçoit alors la désaturation du moteur, celle que la mise en gamme finale lui aurait donnée) à
//! plein au-delà de la marge ([`GAMUT_MARGIN`]). Aux réglages neutres, où la couleur égale sa source
//! au bruit d'arrondi du pont près, ce bruit peut encore faire passer le seuil : la teinte n'est
//! alors rendue qu'en proportion de l'excès relatif, soit l'arrondi multiplié par l'inverse de la
//! marge (vingt), sur l'écart entre la teinte rendue et la couleur désaturée, qui va jusqu'à 39 codes
//! (mesuré hors moteur : un excès relatif de 1e-6 déplace déjà la couleur de 0,013 code, et la mise
//! en gamme finale seule, sans retour de gamme, de 0,023). Un fichier P3 rendu en sRGB rend ce que
//! rend V6, sa source dans la gamme ou hors d'elle
//! (`tests_evora1::wired::pushed::neutral_wide_file_keeps_the_engine_gamut_map`).
//!
//! Ce qu'il laisse à la mise en gamme du moteur, qui le suit : rien de ce qui sort par le bas (un
//! canal négatif n'est pas son affaire), et par le haut un reste de quelques centièmes, que la
//! teinte rendue fait ressortir de la gamme.
//!
//! Référence : `retour_gamme` (pont.py:71-108) du rendu de référence d'Evora, avec `mise_en_gamme`
//! (couleur.py:87-94), qui est la mise en gamme du moteur, et ses passages par OkLab, qui sont ceux
//! du moteur ([`oklab_from_2020`], [`oklab_to_2020`]). Tout le calcul est par point, en simple
//! précision.

use evolight_color::perceptual::{oklab_from_2020, oklab_to_2020};

use super::stage::DISPLAY_WHITE;
use super::{GAMUT_KEPT, GAMUT_MARGIN};
use crate::finish::gamut_map_to;

/// Evora (EVORA-REV-113) : [`GAMUT_KEPT`] en simple précision, pour le calcul par point : la part de
/// l'excès que le retour de gamme rend en luminance (`gamme_retenue`, déclaré à evora1.py:47, passé
/// au retour de gamme à la ligne 278).
pub const KEPT: f32 = GAMUT_KEPT as f32;
/// Evora (EVORA-REV-114) : [`GAMUT_MARGIN`] en simple précision, pour le calcul par point : l'excès
/// relatif sur la référence de la source au-delà duquel le retour de gamme rend la teinte en plein
/// (`gamme_marge`, déclaré à evora1.py:48, passé au retour de gamme à la ligne 278).
pub const MARGIN: f32 = GAMUT_MARGIN as f32;
/// Evora (EVORA-REV-113) : plancher du plus grand canal de sortie sous le gain du retour de gamme
/// (pont.py:97). Il n'est jamais atteint : une couleur poussée passe le blanc.
const PEAK_FLOOR: f32 = 1e-9;
/// Evora (EVORA-REV-113) : plancher de la chroma OkLab d'avant la désaturation (pont.py:102). Sous
/// lui (une couleur neutre), la teinte n'a pas de sens, et la couleur ramenée reste neutre.
const CHROMA_FLOOR: f32 = 1e-9;

/// Evora (EVORA-REV-113) : le produit d'une matrice (rangée par lignes) et d'une couleur. C'est
/// l'arithmétique de `mul3`, dans `finish`, qui est privée à son module : même ordre des sommes,
/// pour que le plus grand canal de sortie lu ici soit celui que lira la mise en gamme du moteur.
#[inline]
pub(crate) fn mul3(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Evora (EVORA-REV-113) : le plus grand canal de la couleur `d` (Rec.2020 linéaire) une fois dans
/// l'espace de sortie (`mx`, pont.py:91). `to_out` mène du Rec.2020 linéaire à la sortie linéaire.
#[inline]
pub fn peak_of(d: [f32; 3], to_out: &[[f32; 3]; 3]) -> f32 {
    let out = mul3(to_out, d);
    out[0].max(out[1]).max(out[2])
}

/// Evora (EVORA-REV-113) : la référence du retour de gamme (`ref`, pont.py:92) : le plus grand canal
/// de sortie que la source avait déjà au même point, et au moins le blanc ([`DISPLAY_WHITE`]). Une
/// source dans la gamme de sortie donne le blanc.
#[inline]
pub fn reference_of(source: [f32; 3], to_out: &[[f32; 3]; 3]) -> f32 {
    peak_of(source, to_out).max(DISPLAY_WHITE)
}

/// Evora (EVORA-REV-114) : l'excès relatif de la couleur sur la référence de sa source, ramené à
/// 0..1 (`exces`, pont.py:96) : 0 à la référence, 1 dès que le plus grand canal de sortie `peak` la
/// passe de la part `margin` d'elle-même. `reference` vaut au moins le blanc ([`reference_of`]).
#[inline]
pub fn excess_of(peak: f32, reference: f32, margin: f32) -> f32 {
    ((peak - reference) / (margin * reference)).clamp(0.0, 1.0)
}

/// Evora (EVORA-REV-113, loi d'EVORA-REV-114) : la couleur poussée `d` ramenée dans la gamme de
/// sortie (pont.py:97-108). L'appelant a vérifié que son plus grand canal de sortie passe la
/// référence de sa source, et donne le gain `gain` de la loi (`(ref / mx)^part`, pont.py:97, entier
/// dès que la couleur est poussée) et l'excès relatif `excess` ([`excess_of`]).
///
/// Le gain, sous 1, rend en luminance une part de l'excès. La couleur ainsi assombrie est ensuite
/// désaturée par la mise en gamme du moteur, sous les poids de luminance `luma` ; si celle-ci n'a
/// rien eu à faire, la couleur assombrie est rendue. Sinon la couleur rendue mêle, en lumière
/// linéaire et en proportion de `excess`, la couleur désaturée et cette même couleur à la teinte OkLab
/// d'avant la désaturation (sa luminosité et sa chroma gardées) : à `excess` nul, c'est la
/// désaturation du moteur ; à `excess` plein, la teinte est rendue. `from_out` ramène de la sortie
/// linéaire au Rec.2020 linéaire. OkLab se lit sur des canaux ramenés à zéro quand ils sont négatifs ;
/// la couleur désaturée, elle, garde les siens.
#[inline]
pub fn pulled_back(d: [f32; 3], gain: f32, excess: f32, to_out: &[[f32; 3]; 3], from_out: &[[f32; 3]; 3], luma: [f32; 3]) -> [f32; 3] {
    let dimmed = d.map(|v| v * gain);
    let (mapped, t) = gamut_map_to(mul3(to_out, dimmed), luma, DISPLAY_WHITE);
    if t >= 1.0 {
        return dimmed;
    }
    let desaturated = mul3(from_out, mapped);
    let before = oklab_from_2020(dimmed.map(|v| v.max(0.0)));
    let after = oklab_from_2020(desaturated.map(|v| v.max(0.0)));
    let k = after[1].hypot(after[2]) / before[1].hypot(before[2]).max(CHROMA_FLOOR);
    let hued = oklab_to_2020([after[0], before[1] * k, before[2] * k]);
    std::array::from_fn(|c| desaturated[c] + (hued[c] - desaturated[c]) * excess)
}

/// Evora (EVORA-REV-113, loi d'EVORA-REV-114) : le retour de gamme d'une couleur (`retour_gamme`,
/// pont.py:71-108, pour un point). `d` est la couleur d'affichage après les tons, `source` la couleur
/// d'affichage d'origine au même point, toutes deux en Rec.2020 linéaire. `to_out` et `from_out`
/// mènent du Rec.2020 linéaire à la sortie linéaire et en reviennent ; `luma` porte les poids de
/// luminance de la sortie ; `part` est la part de l'excès rendue en luminance ([`KEPT`] dans le
/// rendu), `margin` l'excès relatif au-delà duquel la teinte est rendue en plein ([`MARGIN`]).
///
/// Si le plus grand canal de sortie de `d` ne dépasse pas la référence de sa source (test strict),
/// `d` est rendue telle quelle, au bit près. Une couleur qui n'est pas un nombre n'est jamais
/// poussée. Au-delà, la loi est continue : juste au-dessus de la référence, le gain vaut presque 1 et
/// la couleur ne reçoit que la désaturation du moteur, qu'elle recevrait de toute façon de la mise en
/// gamme finale.
#[inline]
pub fn gamut_return(
    d: [f32; 3],
    source: [f32; 3],
    to_out: &[[f32; 3]; 3],
    from_out: &[[f32; 3]; 3],
    luma: [f32; 3],
    part: f32,
    margin: f32,
) -> [f32; 3] {
    gamut_return_with(d, || source, to_out, from_out, luma, part, margin)
}

/// Evora (EVORA-REV-113) : [`gamut_return`], la seule écriture de la loi, avec la source donnée par
/// `source`, qui n'est appelée que si le plus grand canal de sortie de `d` passe le blanc : la
/// référence vaut au moins le blanc ([`reference_of`]), et en dessous rien n'est poussé. Le noyau
/// s'en sert pour ne retrouver la source d'un point par le pont que lorsqu'elle peut compter
/// ([`super::stage::Kernel::gamut_return`]). Même résultat, au bit près, qu'avec la source donnée
/// d'avance.
#[inline]
pub fn gamut_return_with(
    d: [f32; 3],
    source: impl FnOnce() -> [f32; 3],
    to_out: &[[f32; 3]; 3],
    from_out: &[[f32; 3]; 3],
    luma: [f32; 3],
    part: f32,
    margin: f32,
) -> [f32; 3] {
    let peak = peak_of(d, to_out);
    if peak > DISPLAY_WHITE {
        let reference = reference_of(source(), to_out);
        if peak > reference {
            let excess = excess_of(peak, reference, margin);
            let gain = (reference / peak.max(PEAK_FLOOR)).powf(part);
            return pulled_back(d, gain, excess, to_out, from_out, luma);
        }
    }
    d
}

#[cfg(test)]
mod tests {
    //! Evora (EVORA-REV-113) : le retour de gamme face aux vecteurs d'or de la référence, puis ses
    //! deux invariants (rien dans la gamme ; peu de reste au-dessus du blanc).
    //!
    //! Les vecteurs et leur tolérance sont recopiés par outil (`poser_rev113.py`, à côté des
    //! vecteurs d'or) depuis `tableaux-rust.txt` et `05-retour-gamme.json`, que le générateur des
    //! vecteurs tire de la référence : entrées en simple précision, attendus en double précision.
    //! Ils ne se retouchent pas à la main ; ils se régénèrent. Depuis EVORA-REV-114, ceux dont la
    //! valeur change sous la loi neuve sont reposés par `poser_rev114.py` (vecteurs du dossier `v114`).

    use evolight_develop::{DevelopSettings, Process};

    use super::{KEPT, MARGIN, gamut_return, mul3, peak_of};
    use crate::SourceInfo;
    use crate::evora1::bridge::Bridge;
    use crate::evora1::stage::{DISPLAY_WHITE, Kernel};
    use crate::output::{DisplaySpace, OutputSpace};

    // >>> or:RETOUR-DE-GAMME (EVORA-REV-114 : écrit par poser_rev114.py depuis v114/tableaux-rust.txt, section EVORA-REV-113 ; ne pas retoucher)
    /// Evora (EVORA-REV-113) : Rec.2020 linéaire -> srgb linéaire, de la référence.
    #[rustfmt::skip]
    const GOLD_SRGB_FROM_2020: [[f64; 3]; 3] = [
        [1.6604910021084343, -0.5876411387885495, -0.07284986331988494],
        [-0.12455047452159064, 1.1328998971259598, -0.008349422604369465],
        [-0.018150763354905213, -0.10057889800800741, 1.118729661362913],
    ];

    /// Evora (EVORA-REV-113) : srgb linéaire -> Rec.2020 linéaire, de la référence.
    #[rustfmt::skip]
    const GOLD_SRGB_TO_2020: [[f64; 3]; 3] = [
        [0.6274038959346991, 0.32928303837788386, 0.043313065687417246],
        [0.06909728935823205, 0.919540395075459, 0.011362315566309168],
        [0.016391438875150224, 0.08801330787722576, 0.8955952532476237],
    ];

    /// Evora (EVORA-REV-113) : ligne Y exacte de srgb (les poids de luminance du vecteur d'or).
    #[rustfmt::skip]
    const GOLD_SRGB_LUMA: [f64; 3] = [0.21263900587151027, 0.715168678767756, 0.07219231536073371];

    /// Evora (EVORA-REV-113) : (couleur après les tons, source, couleur ramenée), sortie srgb, pont.py:71-108.
    /// Rec.2020 linéaire d'affichage ; part de l'excès rendue en luminance : `GAMUT_KEPT`.
    #[rustfmt::skip]
    const GOLD_GAMUT_RETURN_SRGB: [([f32; 3], [f32; 3], [f64; 3]); 49] = [
        ([1.1068414, 0.43403324, 0.07472823], [0.697884, 0.26591676, 0.035229906], [0.7340204169413801, 0.39638871501735434, 0.1705098673203804]), // orange vif, exposition +1, contraste +0, blancs +0,
        ([1.1821798, 0.1301959, 0.030885413], [0.6274039, 0.06909729, 0.01639144], [0.6642710641274022, 0.16702492388746693, 0.08756068912885817]), // rouge, exposition +1, contraste +0, blancs +0,
        ([1.0068676, 1.0400351, 0.122131355], [0.9566869, 0.9886377, 0.10440475], [0.9632020656421676, 0.9968133032974521, 0.4149659948944249]), // jaune, exposition +1, contraste +0, blancs +0,
        ([0.4554909, 1.0641431, 1.1216017], [0.3725961, 0.9309027, 0.98360854], [0.558622605842769, 0.9520462183704262, 0.9819902291804854]), // cyan, exposition +1, contraste +0, blancs +0,
        ([0.13052054, 0.091676, 1.9386927], [0.060502265, 0.042496037, 0.8986732], [0.09041078268478983, 0.10194877947978247, 0.9084649039639408]), // bleu, exposition +1, contraste +0, blancs +0,
        ([0.9113677, 0.67119956, 0.49972698], [0.65533775, 0.4699709, 0.33762476], [0.8541135659348913, 0.6517464127573979, 0.5038766783117415]), // peau, exposition +1, contraste +0, blancs +0,
        ([0.7982444, 0.92400515, 0.6017255], [0.6575289, 0.77087045, 0.48041666], [0.7982444167137146, 0.9240051507949829, 0.6017255187034607]), // pastel, exposition +1, contraste +0, blancs +0,
        ([0.45916873, 1.090293, 0.23852484], [0.3532253, 0.92266, 0.1541484], [0.5448055879154876, 0.9427600183143715, 0.3896489425199545]), // vert vif, exposition +1, contraste +0, blancs +0,
        ([0.40268162, 0.40268162, 0.40268162], [0.21404114, 0.21404114, 0.21404114], [0.4026816189289093, 0.4026816189289093, 0.4026816189289093]), // gris moyen, exposition +1, contraste +0, blancs +0,
        ([0.058465693, 0.027853556, 0.011938205], [0.024240883, 0.011548564, 0.0049497853], [0.05846569314599037, 0.027853555977344513, 0.011938204988837242]), // sombre coloré, exposition +1, contraste +0, blancs +0,
        ([1.4044293, 0.59808797, 0.16747126], [0.697884, 0.26591676, 0.035229906], [0.7784283542482707, 0.4999758649782407, 0.3027069919718239]), // orange vif, exposition +2, contraste +0, blancs +0,
        ([1.7824333, 0.22466426, 0.077606104], [0.6274039, 0.06909729, 0.01639144], [0.702804879499907, 0.26311012284473423, 0.17722676226893203]), // rouge, exposition +2, contraste +0, blancs +0,
        ([1.0287831, 1.0624672, 0.1302629], [0.9566869, 0.9886377, 0.10440475], [0.969528252678083, 0.9984583208499344, 0.5470384119778195]), // jaune, exposition +2, contraste +0, blancs +0,
        ([0.49387783, 1.1316844, 1.1918954], [0.3725961, 0.9309027, 0.98360854], [0.6448707227047863, 0.9616216680682698, 0.9833627049607327]), // cyan, exposition +2, contraste +0, blancs +0,
        ([0.2462666, 0.17297459, 3.6579323], [0.060502265, 0.042496037, 0.8986732], [0.131612461624757, 0.16696026843912942, 0.9217528358653689]), // bleu, exposition +2, contraste +0, blancs +0,
        ([1.0797498, 0.8132866, 0.62304014], [0.65533775, 0.4699709, 0.33762476], [0.8906285169124156, 0.7418570406994458, 0.62848574943893]), // peau, exposition +2, contraste +0, blancs +0,
        ([0.8723095, 1.0064011, 0.6627724], [0.6575289, 0.77087045, 0.48041666], [0.8748604863242029, 0.9839560058632546, 0.7035440646997063]), // pastel, exposition +2, contraste +0, blancs +0,
        ([0.5099948, 1.1822175, 0.27498278], [0.3532253, 0.92266, 0.1541484], [0.6342784224642113, 0.9533347739591759, 0.5048111494316191]), // vert vif, exposition +2, contraste +0, blancs +0,
        ([0.6271619, 0.6271619, 0.6271619], [0.21404114, 0.21404114, 0.21404114], [0.6271619200706482, 0.6271619200706482, 0.6271619200706482]), // gris moyen, exposition +2, contraste +0, blancs +0,
        ([0.13704865, 0.065291144, 0.027984185], [0.024240883, 0.011548564, 0.0049497853], [0.1370486468076706, 0.0652911439538002, 0.02798418514430523]), // sombre coloré, exposition +2, contraste +0, blancs +0,
        ([0.85963553, 0.3286595, 0.045098223], [0.697884, 0.26591676, 0.035229906], [0.7086318572127587, 0.3199082126428707, 0.08647837886766381]), // orange vif, exposition +0, contraste +100, blancs +0,
        ([0.673541, 0.074178465, 0.01759681], [0.6274039, 0.06909729, 0.01639144], [0.6307920030768827, 0.07878264156739269, 0.022572423233054206]), // rouge, exposition +0, contraste +100, blancs +0,
        ([1.0057592, 1.0389003, 0.12172638], [0.9566869, 0.9886377, 0.10440475], [0.9629236864533033, 0.9967074507619719, 0.4082294811065398]), // jaune, exposition +0, contraste +100, blancs +0,
        ([0.44447508, 1.0443541, 1.1009845], [0.3725961, 0.9309027, 0.98360854], [0.5329773404143571, 0.9491721103023812, 0.9818432570468236]), // cyan, exposition +0, contraste +100, blancs +0,
        ([0.04776894, 0.033552308, 0.7095381], [0.060502265, 0.042496037, 0.8986732], [0.04776893928647041, 0.03355230763554573, 0.7095381021499634]), // bleu, exposition +0, contraste +100, blancs +0,
        ([0.8075037, 0.58667046, 0.42900243], [0.65533775, 0.4699709, 0.33762476], [0.8075037002563477, 0.586670458316803, 0.42900243401527405]), // peau, exposition +0, contraste +100, blancs +0,
        ([0.7701771, 0.8932747, 0.5778197], [0.6575289, 0.77087045, 0.48041666], [0.7701771259307861, 0.89327472448349, 0.5778197050094604]), // pastel, exposition +0, contraste +100, blancs +0,
        ([0.43915364, 1.056819, 0.22321513], [0.3532253, 0.92266, 0.1541484], [0.5095899561515654, 0.9387431807066864, 0.3450724505471891]), // vert vif, exposition +0, contraste +100, blancs +0,
        ([0.23036465, 0.23036465, 0.23036465], [0.21404114, 0.21404114, 0.21404114], [0.23036465048789978, 0.23036465048789978, 0.23036465048789978]), // gris moyen, exposition +0, contraste +100, blancs +0,
        ([0.0130366925, 0.0062107914, 0.0026619832], [0.024240883, 0.011548564, 0.0049497853], [0.013036692515015602, 0.006210791412740946, 0.00266198324970901]), // sombre coloré, exposition +0, contraste +100, blancs +0,
        ([0.7139701, 0.2720461, 0.03604195], [0.697884, 0.26591676, 0.035229906], [0.6993063551571811, 0.2711641370537058, 0.04102089180328038]), // orange vif, exposition +0, contraste +0, blancs +100,
        ([0.6274039, 0.06909729, 0.01639144], [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]), // rouge, exposition +0, contraste +0, blancs +100,
        ([1.0294493, 1.0631491, 0.13051383], [0.9566869, 0.9886377, 0.10440475], [0.9697423415784606, 0.9984965402443583, 0.5510210897957498]), // jaune, exposition +0, contraste +0, blancs +100,
        ([0.4650409, 1.0811491, 1.1393116], [0.3725961, 0.9309027, 0.98360854], [0.5805215737427892, 0.9544901179679499, 0.9822150708267312]), // cyan, exposition +0, contraste +0, blancs +100,
        ([0.060502265, 0.042496037, 0.8986732], [0.060502265, 0.042496037, 0.8986732], [0.06050226464867592, 0.04249603673815727, 0.8986731767654419]), // bleu, exposition +0, contraste +0, blancs +100,
        ([0.7344307, 0.5286392, 0.38171047], [0.65533775, 0.4699709, 0.33762476], [0.7344306707382202, 0.5286391973495483, 0.3817104697227478]), // peau, exposition +0, contraste +0, blancs +100,
        ([0.7893083, 0.9140272, 0.5944174], [0.6575289, 0.77087045, 0.48041666], [0.7893083095550537, 0.914027214050293, 0.59441739320755]), // pastel, exposition +0, contraste +0, blancs +100,
        ([0.45416617, 1.0810369, 0.23500942], [0.3532253, 0.92266, 0.1541484], [0.5355970020227059, 0.941708757543174, 0.37799206134719143]), // vert vif, exposition +0, contraste +0, blancs +100,
        ([0.21404114, 0.21404114, 0.21404114], [0.21404114, 0.21404114, 0.21404114], [0.21404114365577698, 0.21404114365577698, 0.21404114365577698]), // gris moyen, exposition +0, contraste +0, blancs +100,
        ([0.024240883, 0.011548564, 0.0049497853], [0.024240883, 0.011548564, 0.0049497853], [0.024240883067250252, 0.011548563838005066, 0.004949785303324461]), // sombre coloré, exposition +0, contraste +0, blancs +100,
        ([0.3, 0.4, 0.2], [0.3, 0.4, 0.2], [0.30000001192092896, 0.4000000059604645, 0.20000000298023224]), // dans la gamme : identité,
        ([0.6, 0.5, 0.45], [0.2, 0.15, 0.1], [0.6000000238418579, 0.5, 0.44999998807907104]), // dans la gamme, source autre,
        ([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]), // source déjà hors gamme, d égal à la source : intact,
        ([0.0, 1.2, 0.0], [0.0, 1.0, 0.0], [0.3701228988233708, 0.9209255711844024, 0.3390256324498763]), // source déjà hors gamme, d poussé de 20 %,
        ([0.0, 0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.5, 0.0]), // source déjà hors gamme, d assombri : intact,
        ([1.4, -0.02, 0.3], [0.7, 0.0, 0.15], [0.655423056384543, 0.11344056481188321, 0.22536200789143698]), // canal négatif en entrée,
        ([1.2, 1.2, 1.2], [0.9, 0.9, 0.9], [0.9999999999999977, 1.0000000000000009, 1.0000000000000002]), // neutre au-dessus de 1,
        ([50.0, 10.0, 1.0], [0.9, 0.3, 0.05], [1.0005928084831883, 0.9998182170281947, 0.9988081245326463]), // très au-dessus de 1,
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]), // noir,
    ];

    /// Evora (EVORA-REV-113) : Rec.2020 linéaire -> p3 linéaire, de la référence.
    #[rustfmt::skip]
    const GOLD_P3_FROM_2020: [[f64; 3]; 3] = [
        [1.3435782525843323, -0.28217967052613585, -0.061398582058196295],
        [-0.06529745278911943, 1.0757879158485744, -0.010490463059454967],
        [0.002821787261701049, -0.019598494524494185, 1.0167767072627933],
    ];

    /// Evora (EVORA-REV-113) : p3 linéaire -> Rec.2020 linéaire, de la référence.
    #[rustfmt::skip]
    const GOLD_P3_TO_2020: [[f64; 3]; 3] = [
        [0.7538330343617216, 0.19859736905261643, 0.04756959658566187],
        [0.045743848965358255, 0.9417772198116938, 0.012478931222948129],
        [-0.001210340354518392, 0.017601717301089993, 0.9836086230534282],
    ];

    /// Evora (EVORA-REV-113) : ligne Y exacte de p3 (les poids de luminance du vecteur d'or).
    #[rustfmt::skip]
    const GOLD_P3_LUMA: [f64; 3] = [0.22897456406974873, 0.6917385218365063, 0.079286914093745];

    /// Evora (EVORA-REV-113) : (couleur après les tons, source, couleur ramenée), sortie p3, pont.py:71-108.
    /// Rec.2020 linéaire d'affichage ; part de l'excès rendue en luminance : `GAMUT_KEPT`.
    #[rustfmt::skip]
    const GOLD_GAMUT_RETURN_P3: [([f32; 3], [f32; 3], [f64; 3]); 49] = [
        ([1.1068414, 0.43403324, 0.07472823], [0.697884, 0.26591676, 0.035229906], [0.8245336818572847, 0.41571640067028304, 0.15518470127208725]), // orange vif, exposition +1, contraste +0, blancs +0,
        ([1.1821798, 0.1301959, 0.030885413], [0.6274039, 0.06909729, 0.01639144], [0.7822845402189821, 0.16621708663979604, 0.07813834639451993]), // rouge, exposition +1, contraste +0, blancs +0,
        ([1.0068676, 1.0400351, 0.122131355], [0.9566869, 0.9886377, 0.10440475], [0.9632020676756127, 0.9968133058100557, 0.4149659801570255]), // jaune, exposition +1, contraste +0, blancs +0,
        ([0.4554909, 1.0641431, 1.1216017], [0.3725961, 0.9309027, 0.98360854], [0.5550590890436451, 0.9627290084483693, 0.9941958869763863]), // cyan, exposition +1, contraste +0, blancs +0,
        ([0.13052054, 0.091676, 1.9386927], [0.060502265, 0.042496037, 0.8986732], [0.0941281260824855, 0.10326612132016381, 0.9890954328685719]), // bleu, exposition +1, contraste +0, blancs +0,
        ([0.9113677, 0.67119956, 0.49972698], [0.65533775, 0.4699709, 0.33762476], [0.9078863598532352, 0.6701443148124092, 0.5003805605516235]), // peau, exposition +1, contraste +0, blancs +0,
        ([0.7982444, 0.92400515, 0.6017255], [0.6575289, 0.77087045, 0.48041666], [0.7982444167137146, 0.9240051507949829, 0.6017255187034607]), // pastel, exposition +1, contraste +0, blancs +0,
        ([0.45916873, 1.090293, 0.23852484], [0.3532253, 0.92266, 0.1541484], [0.5409290178509948, 0.9637163837153598, 0.3777309467906373]), // vert vif, exposition +1, contraste +0, blancs +0,
        ([0.40268162, 0.40268162, 0.40268162], [0.21404114, 0.21404114, 0.21404114], [0.4026816189289093, 0.4026816189289093, 0.4026816189289093]), // gris moyen, exposition +1, contraste +0, blancs +0,
        ([0.058465693, 0.027853556, 0.011938205], [0.024240883, 0.011548564, 0.0049497853], [0.05846569314599037, 0.027853555977344513, 0.011938204988837242]), // sombre coloré, exposition +1, contraste +0, blancs +0,
        ([1.4044293, 0.59808797, 0.16747126], [0.697884, 0.26591676, 0.035229906], [0.856075199812302, 0.5305146069316359, 0.305457665063014]), // orange vif, exposition +2, contraste +0, blancs +0,
        ([1.7824333, 0.22466426, 0.077606104], [0.6274039, 0.06909729, 0.01639144], [0.8110715180854826, 0.27680714518042077, 0.17869238249837963]), // rouge, exposition +2, contraste +0, blancs +0,
        ([1.0287831, 1.0624672, 0.1302629], [0.9566869, 0.9886377, 0.10440475], [0.9695282532803906, 0.9984583218307609, 0.5470384043859682]), // jaune, exposition +2, contraste +0, blancs +0,
        ([0.49387783, 1.1316844, 1.1918954], [0.3725961, 0.9309027, 0.98360854], [0.646735553956279, 0.970640725965215, 0.993008881710953]), // cyan, exposition +2, contraste +0, blancs +0,
        ([0.2462666, 0.17297459, 3.6579323], [0.060502265, 0.042496037, 0.8986732], [0.13665731588694963, 0.17217355210296348, 0.997981063600438]), // bleu, exposition +2, contraste +0, blancs +0,
        ([1.0797498, 0.8132866, 0.62304014], [0.65533775, 0.4699709, 0.33762476], [0.9328252966465809, 0.7655887716517463, 0.63936198949783]), // peau, exposition +2, contraste +0, blancs +0,
        ([0.8723095, 1.0064011, 0.6627724], [0.6575289, 0.77087045, 0.48041666], [0.8751382095738158, 0.989402345550203, 0.696079125961854]), // pastel, exposition +2, contraste +0, blancs +0,
        ([0.5099948, 1.1822175, 0.27498278], [0.3532253, 0.92266, 0.1541484], [0.63846989422915, 0.9706781427034908, 0.5041917779083339]), // vert vif, exposition +2, contraste +0, blancs +0,
        ([0.6271619, 0.6271619, 0.6271619], [0.21404114, 0.21404114, 0.21404114], [0.6271619200706482, 0.6271619200706482, 0.6271619200706482]), // gris moyen, exposition +2, contraste +0, blancs +0,
        ([0.13704865, 0.065291144, 0.027984185], [0.024240883, 0.011548564, 0.0049497853], [0.1370486468076706, 0.0652911439538002, 0.02798418514430523]), // sombre coloré, exposition +2, contraste +0, blancs +0,
        ([0.85963553, 0.3286595, 0.045098223], [0.697884, 0.26591676, 0.035229906], [0.8117024284188153, 0.3278660259791548, 0.05930770488297787]), // orange vif, exposition +0, contraste +100, blancs +0,
        ([0.673541, 0.074178465, 0.01759681], [0.6274039, 0.06909729, 0.01639144], [0.6735410094261169, 0.07417846471071243, 0.017596809193491936]), // rouge, exposition +0, contraste +100, blancs +0,
        ([1.0057592, 1.0389003, 0.12172638], [0.9566869, 0.9886377, 0.10440475], [0.9629236889703999, 0.996707453808216, 0.40822946455331616]), // jaune, exposition +0, contraste +100, blancs +0,
        ([0.44447508, 1.0443541, 1.1009845], [0.3725961, 0.9309027, 0.98360854], [0.5277926084459925, 0.9603435094548491, 0.9948575588372628]), // cyan, exposition +0, contraste +100, blancs +0,
        ([0.04776894, 0.033552308, 0.7095381], [0.060502265, 0.042496037, 0.8986732], [0.04776893928647041, 0.03355230763554573, 0.7095381021499634]), // bleu, exposition +0, contraste +100, blancs +0,
        ([0.8075037, 0.58667046, 0.42900243], [0.65533775, 0.4699709, 0.33762476], [0.8075037002563477, 0.586670458316803, 0.42900243401527405]), // peau, exposition +0, contraste +100, blancs +0,
        ([0.7701771, 0.8932747, 0.5778197], [0.6575289, 0.77087045, 0.48041666], [0.7701771259307861, 0.89327472448349, 0.5778197050094604]), // pastel, exposition +0, contraste +100, blancs +0,
        ([0.43915364, 1.056819, 0.22321513], [0.3532253, 0.92266, 0.1541484], [0.5025223062349758, 0.9611646593428, 0.3289602148007695]), // vert vif, exposition +0, contraste +100, blancs +0,
        ([0.23036465, 0.23036465, 0.23036465], [0.21404114, 0.21404114, 0.21404114], [0.23036465048789978, 0.23036465048789978, 0.23036465048789978]), // gris moyen, exposition +0, contraste +100, blancs +0,
        ([0.0130366925, 0.0062107914, 0.0026619832], [0.024240883, 0.011548564, 0.0049497853], [0.013036692515015602, 0.006210791412740946, 0.00266198324970901]), // sombre coloré, exposition +0, contraste +100, blancs +0,
        ([0.7139701, 0.2720461, 0.03604195], [0.697884, 0.26591676, 0.035229906], [0.7139701247215271, 0.2720460891723633, 0.036041948944330215]), // orange vif, exposition +0, contraste +0, blancs +100,
        ([0.6274039, 0.06909729, 0.01639144], [0.6274039, 0.06909729, 0.01639144], [0.6274039149284363, 0.06909728795289993, 0.016391439363360405]), // rouge, exposition +0, contraste +0, blancs +100,
        ([1.0294493, 1.0631491, 0.13051383], [0.9566869, 0.9886377, 0.10440475], [0.9697423421957527, 0.9984965412399794, 0.5510210823144673]), // jaune, exposition +0, contraste +0, blancs +100,
        ([0.4650409, 1.0811491, 1.1393116], [0.3725961, 0.9309027, 0.98360854], [0.5783398779215944, 0.9647533650601633, 0.9937485519872847]), // cyan, exposition +0, contraste +0, blancs +100,
        ([0.060502265, 0.042496037, 0.8986732], [0.060502265, 0.042496037, 0.8986732], [0.06050226464867592, 0.04249603673815727, 0.8986731767654419]), // bleu, exposition +0, contraste +0, blancs +100,
        ([0.7344307, 0.5286392, 0.38171047], [0.65533775, 0.4699709, 0.33762476], [0.7344306707382202, 0.5286391973495483, 0.3817104697227478]), // peau, exposition +0, contraste +0, blancs +100,
        ([0.7893083, 0.9140272, 0.5944174], [0.6575289, 0.77087045, 0.48041666], [0.7893083095550537, 0.914027214050293, 0.59441739320755]), // pastel, exposition +0, contraste +0, blancs +100,
        ([0.45416617, 1.0810369, 0.23500942], [0.3532253, 0.92266, 0.1541484], [0.5308694272714545, 0.9630439732935978, 0.36494274191577997]), // vert vif, exposition +0, contraste +0, blancs +100,
        ([0.21404114, 0.21404114, 0.21404114], [0.21404114, 0.21404114, 0.21404114], [0.21404114365577698, 0.21404114365577698, 0.21404114365577698]), // gris moyen, exposition +0, contraste +0, blancs +100,
        ([0.024240883, 0.011548564, 0.0049497853], [0.024240883, 0.011548564, 0.0049497853], [0.024240883067250252, 0.011548563838005066, 0.004949785303324461]), // sombre coloré, exposition +0, contraste +0, blancs +100,
        ([0.3, 0.4, 0.2], [0.3, 0.4, 0.2], [0.30000001192092896, 0.4000000059604645, 0.20000000298023224]), // dans la gamme : identité,
        ([0.6, 0.5, 0.45], [0.2, 0.15, 0.1], [0.6000000238418579, 0.5, 0.44999998807907104]), // dans la gamme, source autre,
        ([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]), // source déjà hors gamme, d égal à la source : intact,
        ([0.0, 1.2, 0.0], [0.0, 1.0, 0.0], [0.31530333899659657, 0.9468183479907798, 0.2843061070725451]), // source déjà hors gamme, d poussé de 20 %,
        ([0.0, 0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.5, 0.0]), // source déjà hors gamme, d assombri : intact,
        ([1.4, -0.02, 0.3], [0.7, 0.0, 0.15], [0.7754282457557398, 0.08127649755282179, 0.22651745490168537]), // canal négatif en entrée,
        ([1.2, 1.2, 1.2], [0.9, 0.9, 0.9], [0.9999999999999977, 1.0000000000000009, 1.0000000000000002]), // neutre au-dessus de 1,
        ([50.0, 10.0, 1.0], [0.9, 0.3, 0.05], [1.0005928084831883, 0.9998182170281947, 0.9988081245326463]), // très au-dessus de 1,
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]), // noir,
    ];
    // <<< or:RETOUR-DE-GAMME

    // >>> or:RETOUR-POIDS-ARRONDIS (EVORA-REV-114 : écrit par poser_rev114.py depuis v114/05-retour-gamme.json (sortie_poids_arrondis_f64) ; ne pas retoucher)
    /// Evora (EVORA-REV-113) : la couleur ramenée des mêmes vecteurs que `GOLD_GAMUT_RETURN_SRGB`, dans le même ordre, quand la référence
    /// fait le retour de gamme sous les poids de luminance arrondis de la mise en gamme du moteur pour sRGB (0,2126 ; 0,7152 ; 0,0722).
    #[rustfmt::skip]
    const GOLD_GAMUT_RETURN_SRGB_ROUNDED: [[f64; 3]; 49] = [
        [0.7340149571440113, 0.3963728820399072, 0.17049229066211471],
        [0.6642622817367124, 0.16700244794178326, 0.08754242007977306],
        [0.9632006904522152, 0.9968127537855666, 0.4149319815177586],
        [0.5586591751819299, 0.952050321792268, 0.9819903827693977],
        [0.09041386442726809, 0.10195416302573478, 0.9084660313517195],
        [0.8541126195339841, 0.6517439814092402, 0.50387341481421],
        [0.7982444167137146, 0.9240051507949829, 0.6017255187034607],
        [0.5448284731987068, 0.942762583361385, 0.3896776587143891],
        [0.4026816189289093, 0.4026816189289093, 0.4026816189289093],
        [0.05846569314599037, 0.027853555977344513, 0.011938204988837242],
        [0.7784187888062705, 0.4999517506326085, 0.30267742204222436],
        [0.7027888623693517, 0.2630697345117015, 0.17718906038232493],
        [0.9695257096297621, 0.9984578316556912, 0.5469901213646848],
        [0.6449250420332449, 0.9616276761672812, 0.9833637854191302],
        [0.1316201147067884, 0.1669713622364384, 0.9217549556793212],
        [0.8906254495841394, 0.7418495038068772, 0.628475299885492],
        [0.8748623878173766, 0.9839562415779926, 0.7035485172450994],
        [0.6343127673862157, 0.9533388688229497, 0.5048555644219574],
        [0.6271619200706482, 0.6271619200706482, 0.6271619200706482],
        [0.1370486468076706, 0.0652911439538002, 0.02798418514430523],
        [0.7086300620479441, 0.31990107797594924, 0.08647131508534003],
        [0.6307912035898113, 0.07878039316777934, 0.02257096347456301],
        [0.9629223645982247, 0.996706901094679, 0.4081961988608801],
        [0.5330085920436559, 0.9491756343773702, 0.9818432238074272],
        [0.04776893928647041, 0.03355230763554573, 0.7095381021499634],
        [0.8075037002563477, 0.586670458316803, 0.42900243401527405],
        [0.7701771259307861, 0.89327472448349, 0.5778197050094604],
        [0.5096085241023607, 0.9387451925870932, 0.3450954239108159],
        [0.23036465048789978, 0.23036465048789978, 0.23036465048789978],
        [0.013036692515015602, 0.006210791412740946, 0.00266198324970901],
        [0.6993061541647193, 0.2711634080385303, 0.04102008485361001],
        [0.6274039149284363, 0.06909728795289993, 0.016391439363360405],
        [0.969739760187558, 0.998496054288806, 0.5509723703218569],
        [0.5805626695693309, 0.9544947110789209, 0.9822154186704861],
        [0.06050226464867592, 0.04249603673815727, 0.8986731767654419],
        [0.7344306707382202, 0.5286391973495483, 0.3817104697227478],
        [0.7893083095550537, 0.914027214050293, 0.59441739320755],
        [0.5356186936954059, 0.9417111685213994, 0.37801918210613533],
        [0.21404114365577698, 0.21404114365577698, 0.21404114365577698],
        [0.024240883067250252, 0.011548563838005066, 0.004949785303324461],
        [0.30000001192092896, 0.4000000059604645, 0.20000000298023224],
        [0.6000000238418579, 0.5, 0.44999998807907104],
        [0.0, 1.0, 0.0],
        [0.3701988968924403, 0.9209346947561162, 0.3391009839693302],
        [0.0, 0.5, 0.0],
        [0.6554086063614415, 0.11340367462339203, 0.22533081602449181],
        [0.9999999999999977, 1.0000000000000009, 1.0000000000000002],
        [1.0005928084831883, 0.9998182170281947, 0.9988081245326463],
        [0.0, 0.0, 0.0],
    ];
    // <<< or:RETOUR-POIDS-ARRONDIS

    // >>> or:NON-FINIS (EVORA-REV-113 : écrit par poser_rev113.py depuis 05-retour-gamme.json (non_finis) ; ne pas retoucher)
    /// Evora (EVORA-REV-113) : (couleur après les tons, source, couleur rendue) hors des nombres finis, sortie sRGB (05-retour-gamme.json, « non_finis »).
    /// Une couleur dont un canal n'est pas un nombre n'est pas poussée : elle passe telle quelle. Un canal infini donne un gain nul.
    #[rustfmt::skip]
    const GOLD_NON_FINITE_SRGB: [([f32; 3], [f32; 3], [f64; 3]); 2] = [
        ([f32::NAN, 0.5, 0.5], [0.4, 0.4, 0.4], [f64::NAN, 0.5, 0.5]),
        ([f32::INFINITY, 0.5, 0.5], [0.4, 0.4, 0.4], [f64::NAN, 0.0, 0.0]),
    ];
    // <<< or:NON-FINIS

    // >>> or:TOLERANCES-GAMME (EVORA-REV-113 : écrit par poser_rev113.py depuis tableaux-rust.txt, tolérance du retour de gamme ; ne pas retoucher)
    /// Evora : absolue, par canal Rec.2020, gamut_return en f32 face à la référence en f64. Écart mesuré 2.19e-06. Deux passages par OkLab en f32 (racines cubiques, cubes) et une puissance ; mesuré sur 100 000 couples par espace.
    #[rustfmt::skip]
    const GOLD_TOL_GAMUT_RETURN: f64 = 1e-05;
    // <<< or:TOLERANCES-GAMME

    /// Evora (EVORA-REV-113) : écart admis entre un coefficient de matrice du moteur, en simple
    /// précision, et celui de la référence, en double précision : un dernier chiffre de simple
    /// précision d'un nombre sous 2 (le plus grand coefficient vaut 1,66).
    const MATRIX_TOL: f64 = 2.0 * f32::EPSILON as f64;
    /// Evora (EVORA-REV-113) : écart admis à l'identité pour le produit des deux matrices d'un
    /// espace de sortie (aller, puis retour), en simple précision : trois produits et deux sommes de
    /// coefficients sous 2,1, soit quelques derniers chiffres. Mesuré hors moteur : 1,2e-7 au plus.
    const INVERSE_TOL: f64 = 1e-6;
    /// Evora (EVORA-REV-113) : la matrice de retour de l'espace de travail vers lui-même.
    const WORKING_SPACE: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    /// Evora (EVORA-REV-113) : pas du treillis de couleurs des deux invariants, par canal de sortie :
    /// `LATTICE_STEPS + 1` niveaux du noir (0) au blanc (1), primaires et blanc compris.
    const LATTICE_STEPS: usize = 4;
    /// Evora (EVORA-REV-113) : les poussées des deux invariants, en gain sur la source : d'un dixième
    /// au-dessus à seize fois (le blanc de scène du pont).
    const PUSHES: [f32; 5] = [1.1, 1.5, 2.0, 4.0, 16.0];
    /// Evora (EVORA-REV-113) : ce que le retour de gamme peut laisser à la mise en gamme du moteur,
    /// en lumière linéaire de sortie : au-dessus du blanc, et sous le noir. La teinte rendue après la
    /// désaturation fait ressortir la couleur de la gamme, de peu. Mesuré hors moteur sur ce
    /// treillis et ces poussées : 0,0253 au plus au-dessus du blanc (sRGB 0,0243 ; P3 0,0253 ;
    /// Rec.2020 0,0089) et 0,0008 sous le noir ; la borne en vaut le double.
    const RESIDUE_MAX: f32 = 0.05;
    /// Evora (EVORA-REV-113) : les trois espaces de sortie de la référence, ceux où le reste est
    /// mesuré.
    const REFERENCE_SPACES: [OutputSpace; 3] = [OutputSpace::Srgb, OutputSpace::DisplayP3, OutputSpace::Rec2020];
    /// Evora (EVORA-REV-113) : une source dans la gamme, plus sombre que la couleur lue, et une plus
    /// claire, en part de cette couleur.
    const OTHER_SOURCES: [f32; 2] = [0.5, 1.25];
    /// Evora (EVORA-REV-113) : le plus haut niveau du treillis encore franchement dans la gamme : le
    /// niveau du blanc en sort d'un dernier chiffre au passage des deux matrices.
    const INSIDE_STEPS: usize = LATTICE_STEPS - 1;
    /// Evora (EVORA-REV-113) : ce dont le retour de gamme peut ramener, en lumière linéaire, une
    /// couleur égale à sa source à un dernier chiffre près, au bord de la gamme des trois espaces de
    /// la référence. Mesuré hors moteur sur le treillis, sous la loi d'EVORA-REV-114 : 1,8e-7 ;
    /// jusqu'à 1,9e-6 quand les puissances, racines cubiques et hypoténuses bougent d'un dernier
    /// chiffre, 3,9e-6 quand elles bougent de deux (40 tirages chacun ; sous la loi d'EVORA-REV-113 :
    /// 4,8e-7, 2,9e-6 et 5,2e-6). La borne en vaut plus du double. ProPhoto n'est pas tenu : la perte
    /// d'un canal Rec.2020 négatif par OkLab (jusqu'à 0,19 sous la loi d'EVORA-REV-113) n'y joue plus
    /// (6,1e-7), mais la mise en gamme du moteur y est mal conditionnée près du jaune (poids de
    /// luminance du bleu 9e-5) : un canal d'un dernier chiffre au-dessus du blanc fait bouger la couleur
    /// de 1,4e-3 quand les puissances bougent d'un dernier chiffre (2,7e-3 de deux).
    const EDGE_GAP_MAX: f64 = 1e-5;

    /// Un espace de sortie des vecteurs d'or, ses matrices et ses poids dans la référence.
    struct GoldSpace {
        name: &'static str,
        space: OutputSpace,
        to_out: [[f64; 3]; 3],
        from_out: [[f64; 3]; 3],
        luma: [f64; 3],
        vectors: &'static [([f32; 3], [f32; 3], [f64; 3])],
    }

    /// Les deux espaces de sortie des vecteurs d'or du retour de gamme.
    fn gold_spaces() -> [GoldSpace; 2] {
        [
            GoldSpace {
                name: "sRGB",
                space: OutputSpace::Srgb,
                to_out: GOLD_SRGB_FROM_2020,
                from_out: GOLD_SRGB_TO_2020,
                luma: GOLD_SRGB_LUMA,
                vectors: &GOLD_GAMUT_RETURN_SRGB,
            },
            GoldSpace {
                name: "P3",
                space: OutputSpace::DisplayP3,
                to_out: GOLD_P3_FROM_2020,
                from_out: GOLD_P3_TO_2020,
                luma: GOLD_P3_LUMA,
                vectors: &GOLD_GAMUT_RETURN_P3,
            },
        ]
    }

    /// La part d'Evora du noyau pour un fichier rendu sous `Evora1`, dans l'espace de travail.
    fn file_kernel() -> Kernel {
        let s = DevelopSettings { process: Process::Evora1.version(), ..Default::default() };
        let Some(kernel) = Kernel::for_render(&s, &SourceInfo::default()) else {
            panic!("fichier rendu sous Evora1 sans part d'Evora dans le noyau");
        };
        kernel
    }

    /// Le plus grand écart, canal par canal, entre une couleur du moteur et l'attendu de la
    /// référence. Un écart qui n'est pas un nombre est le pire des écarts.
    fn worst_gap(got: [f32; 3], want: [f64; 3]) -> f64 {
        let mut worst = 0.0f64;
        for c in 0..3 {
            let gap = (f64::from(got[c]) - want[c]).abs();
            worst = if gap.is_nan() { f64::INFINITY } else { worst.max(gap) };
        }
        worst
    }

    /// Le plus grand écart entre une matrice du moteur et celle de la référence.
    fn matrix_gap(engine: &[[f32; 3]; 3], gold: &[[f64; 3]; 3]) -> f64 {
        let mut worst = 0.0f64;
        for r in 0..3 {
            for c in 0..3 {
                worst = worst.max((f64::from(engine[r][c]) - gold[r][c]).abs());
            }
        }
        worst
    }

    /// Les couleurs du treillis de l'espace de sortie dont aucun canal ne passe le niveau `top`, en
    /// Rec.2020 linéaire. `from_out` ramène de la sortie au Rec.2020.
    fn lattice(from_out: &[[f32; 3]; 3], top: usize) -> Vec<[f32; 3]> {
        let level = |i: usize| i as f32 / LATTICE_STEPS as f32;
        let mut colours = Vec::new();
        for r in 0..=top {
            for g in 0..=top {
                for b in 0..=top {
                    colours.push(mul3(from_out, [level(r), level(g), level(b)]));
                }
            }
        }
        colours
    }

    /// Evora (EVORA-REV-113) : le retour de gamme rend les vecteurs d'or de la référence, en sRGB et
    /// en P3, sous `GOLD_TOL_GAMUT_RETURN` : couleurs poussées par l'exposition, le contraste et les
    /// blancs, puis cas d'école (source déjà hors de la gamme, canal négatif, neutre et couleur très
    /// au-dessus du blanc, noir). Quatre lectures :
    /// - les matrices du moteur pour ces deux espaces sont celles de la référence, à l'arrondi de la
    ///   simple précision ; pour chaque espace de sortie et pour un écran, la matrice de retour que
    ///   porte le noyau défait celle de l'aller ;
    /// - la fonction, sous la ligne de luminance exacte de la référence, rend l'attendu ;
    /// - l'entrée du noyau, qui retrouve la source depuis la couleur de scène, rend le même attendu
    ///   (hors des vecteurs où la couleur égale sa source : le seuil strict s'y joue sur l'arrondi de
    ///   la source retrouvée) ;
    /// - sous les poids de luminance du moteur pour sRGB, qui sont arrondis, la fonction rend
    ///   l'attendu que la référence donne sous ces mêmes poids.
    ///
    /// Sabotages qui le font rougir : la part de l'excès rendue en luminance changée ; la matrice de
    /// retour prise pour un autre espace que la sortie ; la teinte rendue sans reprendre la chroma de
    /// la couleur ramenée ; OkLab lu sur des canaux négatifs ; la teinte rendue en plein dès le seuil
    /// (marge nulle, la loi d'EVORA-REV-113 : deux vecteurs par espace, poussés de peu, s'écartent
    /// alors de l'attendu de plus que la tolérance, mesuré hors moteur). Témoin intégré : sous les poids
    /// arrondis du moteur, la fonction manque l'attendu de la ligne exacte, de plus que la tolérance.
    #[test]
    fn gamut_return_matches_the_reference() {
        let bridge = Bridge::default();
        let mut pushed = 0usize;
        for gold in gold_spaces() {
            let name = gold.name;
            let (to_out, kernel) = (gold.space.from_working(), file_kernel().in_space(gold.space));
            let from_out = *kernel.from_out();
            // les matrices du moteur sont celles de la référence
            let (gap_to, gap_from) = (matrix_gap(&to_out, &gold.to_out), matrix_gap(&from_out, &gold.from_out));
            assert!(gap_to <= MATRIX_TOL, "{name} : la matrice d'aller du moteur s'écarte de {gap_to} de celle de la référence");
            assert!(gap_from <= MATRIX_TOL, "{name} : la matrice de retour du noyau s'écarte de {gap_from} de celle de la référence");
            let luma = gold.luma.map(|v| v as f32);

            for (i, &(d, source, want)) in gold.vectors.iter().enumerate() {
                let got = gamut_return(d, source, &to_out, &from_out, luma, KEPT, MARGIN);
                let gap = worst_gap(got, want);
                assert!(
                    gap <= GOLD_TOL_GAMUT_RETURN,
                    "{name}, vecteur {i} ({d:?}, source {source:?}) : rendu {got:?}, référence {want:?}, écart {gap}"
                );
                if got != d {
                    pushed += 1;
                }
                // l'entrée du noyau : la source retrouvée depuis la couleur de scène
                if d != source {
                    let by_kernel = kernel.gamut_return(d, bridge.scene_of(source), &to_out, luma);
                    let gap = worst_gap(by_kernel, want);
                    assert!(gap <= GOLD_TOL_GAMUT_RETURN, "{name}, vecteur {i}, par le noyau : rendu {by_kernel:?}, référence {want:?}, écart {gap}");
                }
            }
        }
        assert!(pushed >= 2, "garde de portée : {pushed} vecteur(s) poussé(s) seulement");

        // la matrice de retour du noyau défait celle de l'aller, pour chaque espace et pour un écran
        for space in OutputSpace::ALL {
            let (to_out, kernel) = (space.from_working(), file_kernel().in_space(space));
            for axis in 0..3 {
                let mut unit = [0.0f32; 3];
                unit[axis] = 1.0;
                let back = mul3(kernel.from_out(), mul3(&to_out, unit));
                let gap = worst_gap(back, unit.map(f64::from));
                assert!(gap <= INVERSE_TOL, "{space:?} : aller puis retour s'écartent de {gap} de l'identité sur l'axe {axis}");
            }
        }
        for space in REFERENCE_SPACES {
            let Some(display) = DisplaySpace::of(space, 1) else {
                panic!("{space:?} : écran de test refusé");
            };
            let on_display = file_kernel().in_space(OutputSpace::Srgb).on_display(&display);
            assert!(on_display.from_out() == &display.to_working, "{space:?} : le noyau ne porte pas la matrice de retour de l'écran");
        }
        // tant qu'aucune sortie n'est dite, le noyau est dans l'espace de travail
        assert!(file_kernel().from_out() == &WORKING_SPACE, "sans espace de sortie, la matrice de retour du noyau n'est pas l'identité");

        // les poids de luminance du moteur pour sRGB, arrondis : l'attendu de la référence sous ces poids
        let (to_out, rounded) = (OutputSpace::Srgb.from_working(), OutputSpace::Srgb.luma());
        let from_out = *file_kernel().in_space(OutputSpace::Srgb).from_out();
        let mut witness = 0.0f64;
        for (i, (&(d, source, exact), &want)) in GOLD_GAMUT_RETURN_SRGB.iter().zip(GOLD_GAMUT_RETURN_SRGB_ROUNDED.iter()).enumerate() {
            let got = gamut_return(d, source, &to_out, &from_out, rounded, KEPT, MARGIN);
            let gap = worst_gap(got, want);
            assert!(gap <= GOLD_TOL_GAMUT_RETURN, "sRGB, poids du moteur, vecteur {i} : rendu {got:?}, référence {want:?}, écart {gap}");
            witness = witness.max(worst_gap(got, exact));
        }
        // témoin rouge : ces poids ne sont pas la ligne exacte, et le vecteur d'or le voit
        assert!(witness > GOLD_TOL_GAMUT_RETURN, "témoin : sous les poids arrondis, l'attendu de la ligne exacte est tenu à {witness}");
    }

    /// Evora (EVORA-REV-113) : dans la gamme, le retour de gamme ne fait rien, au bit près. Sur un
    /// treillis de couleurs de chaque espace de sortie :
    /// - une couleur qui égale sa source au bit près passe telle quelle, qu'elle soit dans la gamme,
    ///   à son bord ou hors d'elle (les primaires du Rec.2020 sortent de sRGB) ;
    /// - une couleur franchement dans la gamme passe telle quelle, quelle que soit sa source (noire,
    ///   plus sombre, plus claire) ;
    /// - une couleur plus sombre que sa source hors gamme passe telle quelle ;
    /// - par l'entrée du noyau, une couleur dans la gamme passe telle quelle sans que sa source soit
    ///   recalculée ;
    /// - au bord de la gamme, dans les trois espaces de la référence, une couleur égale à sa source à
    ///   un dernier chiffre près (le bruit du pont aux réglages neutres) peut passer la référence de
    ///   sa source : elle n'est alors ramenée que de `EDGE_GAP_MAX` au plus.
    ///
    /// Ce qui n'est pas un nombre suit la référence : une couleur dont un canal n'est pas un nombre
    /// passe telle quelle ; un canal infini donne un gain nul, donc ce canal sans nombre et les deux
    /// autres à zéro.
    ///
    /// Sabotages qui le font rougir : le test strict devenu large (une couleur égale à sa source hors
    /// gamme serait ramenée) ; la référence prise sans son plancher au blanc (une couleur dans la
    /// gamme, plus claire que sa source, serait ramenée) ; la source lue à la place de la couleur.
    /// Témoin intégré : la même couleur poussée au-delà de sa source n'est pas rendue telle quelle.
    #[test]
    fn gamut_return_is_the_identity_in_gamut() {
        let bridge = Bridge::default();
        let (mut read, mut witnessed) = (0usize, 0usize);
        for space in OutputSpace::ALL {
            let (to_out, luma, kernel) = (space.from_working(), space.luma(), file_kernel().in_space(space));
            let from_out = *kernel.from_out();
            let keeps = |d: [f32; 3], source: [f32; 3]| gamut_return(d, source, &to_out, &from_out, luma, KEPT, MARGIN) == d;

            // une couleur qui égale sa source : le treillis entier, puis les primaires du Rec.2020
            let primaries = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
            for d in lattice(&from_out, LATTICE_STEPS).into_iter().chain(primaries) {
                assert!(keeps(d, d), "{space:?} : la couleur {d:?}, égale à sa source, est touchée");
                // plus sombre que sa source, elle passe aussi telle quelle
                let dark = d.map(|v| v * OTHER_SOURCES[0]);
                assert!(keeps(dark, d), "{space:?} : la couleur {dark:?}, plus sombre que sa source {d:?}, est touchée");
                read += 2;
            }

            // une couleur franchement dans la gamme, quelle que soit sa source
            for d in lattice(&from_out, INSIDE_STEPS) {
                assert!(peak_of(d, &to_out) < DISPLAY_WHITE, "garde de portée : {space:?}, la couleur {d:?} n'est pas dans la gamme");
                for source in [[0.0; 3], d.map(|v| v * OTHER_SOURCES[0]), d.map(|v| v * OTHER_SOURCES[1])] {
                    assert!(keeps(d, source), "{space:?} : la couleur {d:?}, dans la gamme, est touchée (source {source:?})");
                    let by_kernel = kernel.gamut_return(d, bridge.scene_of(source), &to_out, luma);
                    assert!(by_kernel == d, "{space:?} : par le noyau, la couleur {d:?}, dans la gamme, est touchée (source {source:?})");
                    read += 2;
                }
                // témoin rouge : poussée au-delà de sa source, la même couleur est ramenée
                let pushed = d.map(|v| v * PUSHES[PUSHES.len() - 1]);
                if peak_of(pushed, &to_out) > DISPLAY_WHITE {
                    assert!(!keeps(pushed, d), "témoin : {space:?}, la couleur {pushed:?}, poussée hors de la gamme, passe telle quelle");
                    witnessed += 1;
                }
            }
        }
        assert!(read >= 100 && witnessed >= 10, "garde de portée : {read} lecture(s), {witnessed} témoin(s)");

        // au bord de la gamme, dans les trois espaces de la référence : une couleur égale à sa source à
        // un dernier chiffre près peut passer la référence de sa source ; elle n'est ramenée que de peu
        let mut edge_moved = 0usize;
        for space in REFERENCE_SPACES {
            let (to_out, luma) = (space.from_working(), space.luma());
            let from_out = *file_kernel().in_space(space).from_out();
            for source in lattice(&from_out, LATTICE_STEPS) {
                let d = source.map(|v| f32::from_bits(v.to_bits() + 1));
                let got = gamut_return(d, source, &to_out, &from_out, luma, KEPT, MARGIN);
                let gap = worst_gap(got, d.map(f64::from));
                assert!(gap <= EDGE_GAP_MAX, "{space:?} : la couleur {d:?}, à un dernier chiffre de sa source, est ramenée de {gap}");
                if got != d {
                    edge_moved += 1;
                }
            }
        }
        assert!(edge_moved > 0, "garde de portée : aucune couleur du bord de la gamme n'est ramenée");

        // ce qui n'est pas un nombre suit la référence
        let (to_out, luma) = (OutputSpace::Srgb.from_working(), OutputSpace::Srgb.luma());
        let from_out = *file_kernel().in_space(OutputSpace::Srgb).from_out();
        for (i, &(d, source, want)) in GOLD_NON_FINITE_SRGB.iter().enumerate() {
            let got = gamut_return(d, source, &to_out, &from_out, luma, KEPT, MARGIN);
            for c in 0..3 {
                let same = if want[c].is_nan() { got[c].is_nan() } else { f64::from(got[c]) == want[c] };
                assert!(same, "non fini {i} ({d:?}), canal {c} : rendu {}, référence {}", got[c], want[c]);
            }
        }
    }

    /// Evora (EVORA-REV-113) : après le retour de gamme, il reste peu à faire à la mise en gamme du
    /// moteur. Sur le treillis de chacun des trois espaces de sortie de la référence, chaque source
    /// poussée de `PUSHES` : aucun canal de sortie ne passe le blanc de plus de `RESIDUE_MAX`, ni ne
    /// descend sous le noir de plus. Le reste au-dessus du blanc existe (la teinte rendue fait
    /// ressortir la couleur de la gamme) : c'est une borne, pas une égalité.
    ///
    /// Ces sources sont dans la gamme : rien n'y sort par le bas avant le retour de gamme. Une
    /// couleur que les tons rendent avec un canal négatif garde ce canal, que la mise en gamme du
    /// moteur ramène ; ce test ne dit rien d'elle.
    ///
    /// Sabotages qui le font rougir : le gain seul, sans la désaturation ; la teinte rendue à la
    /// chroma d'avant la désaturation ; la désaturation faite sous une crête au-dessus du blanc.
    /// Témoin intégré : sans le retour de gamme, plus de la moitié de ces couleurs passent le blanc
    /// de plus que la borne.
    #[test]
    fn gamut_return_leaves_little_to_the_final_gamut_map() {
        for space in REFERENCE_SPACES {
            let (to_out, luma) = (space.from_working(), space.luma());
            let from_out = *file_kernel().in_space(space).from_out();
            let (mut read, mut over_before, mut worst) = (0usize, 0usize, 0.0f32);
            for source in lattice(&from_out, LATTICE_STEPS) {
                for push in PUSHES {
                    let d = source.map(|v| v * push);
                    if peak_of(d, &to_out) - DISPLAY_WHITE > RESIDUE_MAX {
                        over_before += 1;
                    }
                    let out = mul3(&to_out, gamut_return(d, source, &to_out, &from_out, luma, KEPT, MARGIN));
                    for c in 0..3 {
                        assert!(out[c].is_finite(), "{space:?}, source {source:?} poussée de {push} : canal {c} non fini");
                        assert!(
                            out[c] <= DISPLAY_WHITE + RESIDUE_MAX && out[c] >= -RESIDUE_MAX,
                            "{space:?}, source {source:?} poussée de {push} : canal {c} à {}, hors de 0..1 de plus de {RESIDUE_MAX}",
                            out[c]
                        );
                        worst = worst.max(out[c] - DISPLAY_WHITE);
                    }
                    read += 1;
                }
            }
            // témoin rouge : sans le retour de gamme, la mise en gamme du moteur aurait tout à faire
            assert!(
                2 * over_before > read,
                "témoin : {space:?}, {over_before} couleur(s) sur {read} passent le blanc de plus de {RESIDUE_MAX} avant le retour"
            );
            // le reste existe : la borne n'est pas tenue par un retour qui ramènerait tout au blanc exact
            assert!(worst > 0.0, "garde de portée : {space:?}, aucun reste au-dessus du blanc (pire {worst})");
        }
    }
}
