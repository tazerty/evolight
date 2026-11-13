//! Evora (EVORA-REV-110) : le pont de scène du rendu d'Evora. Branché depuis EVORA-REV-112 : sous le
//! prédicat du rendu, l'étage linéaire passe le pont aller (`stage::Frame::to_scene`), et la table de
//! tons d'Evora finit par le pont de retour (`super::tone::table`).
//!
//! Un fichier déjà rendu (JPEG, PNG) porte une lumière d'affichage, de 0 à 1. Le pont la mène à une
//! lumière de scène où les curseurs d'Evora agissent comme sur un brut, puis l'en ramène. C'est une
//! courbe en forme close, `d = W·sⁿ / (sⁿ + q)` : un pied en puissance `n`, une épaule qui tend vers
//! le blanc sans jamais couper. Elle est ancrée au gris moyen (0,18 de scène rend 0,18 d'affichage)
//! et rend le blanc d'affichage au blanc de scène ([`BRIDGE_WHITE`]). Aux réglages neutres, le pont
//! aller puis le pont de retour rendent le fichier.
//!
//! Les clairs d'un fichier rendu ont perdu de leur couleur. Le pont la leur rend à l'aller et la
//! reprend au retour par la même échelle de chroma, lue entre huit nœuds comme le fait la table de
//! tons du moteur ([`crate::tone::ToneMap::chroma_scale`]).
//!
//! Référence : pont.py du rendu de référence d'Evora (`Pont`, `noeuds_chroma`, `chroma_affichage`,
//! `image_vers_scene`). Les deux constantes du pont et les nœuds se calculent une fois, en double
//! précision ; le calcul par point est en simple précision, comme le reste du noyau.

use evolight_color::luminance_2020;
use evolight_raster::Rgb32f;

use super::{BRIDGE_N, BRIDGE_WHITE, CHROMA_FROM, CHROMA_MIN, GREY, LUMINANCE_FLOOR, smooth};
use crate::tone::CHROMA_N;

/// Evora (EVORA-REV-110) : [`LUMINANCE_FLOOR`] en simple précision, pour le calcul par point.
const FLOOR: f32 = LUMINANCE_FLOOR as f32;

/// Evora (EVORA-REV-110) : les deux constantes `(q, W)` du pont d'exposant `n` qui rend `grey` à
/// `grey` et `white` à 1 (`Pont.q` et `Pont.w`, pont.py:22-28).
fn constants(grey: f64, n: f64, white: f64) -> (f64, f64) {
    let (a, b) = (grey.powf(n), white.powf(n));
    let q = (1.0 - grey) * a / (grey - a / b);
    (q, 1.0 + q / b)
}

/// Evora (EVORA-REV-110) : le pont de scène (`Pont`, pont.py:16-38) et son échelle de chroma.
///
/// `q` et `W` sont calculés en double précision à la construction, depuis le gris moyen exact
/// ([`GREY`]). Le calcul par point lit leurs arrondis en simple précision.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bridge {
    n: f64,
    q: f64,
    w: f64,
    n32: f32,
    inv_n32: f32,
    q32: f32,
    w32: f32,
    chroma: [f32; CHROMA_N],
}

impl Default for Bridge {
    /// Le pont du rendu d'Evora : exposant [`BRIDGE_N`], blanc de scène [`BRIDGE_WHITE`].
    fn default() -> Self {
        Bridge::new(BRIDGE_N, BRIDGE_WHITE)
    }
}

impl Bridge {
    /// Evora (EVORA-REV-110) : le pont d'exposant `n` et de blanc de scène `white` (`Pont(n, blanc)`).
    /// `white` doit dépasser le gris moyen, sans quoi `q` n'est pas positif.
    pub fn new(n: f64, white: f64) -> Bridge {
        let (q, w) = constants(GREY, n, white);
        Bridge { n, q, w, n32: n as f32, inv_n32: (1.0 / n) as f32, q32: q as f32, w32: w as f32, chroma: chroma_nodes() }
    }

    /// L'exposant du pont, en double précision.
    pub fn n(&self) -> f64 {
        self.n
    }

    /// La constante `q` du pont, en double précision (`Pont.q`).
    pub fn q(&self) -> f64 {
        self.q
    }

    /// La constante `W` du pont, en double précision (`Pont.w`) : la limite de la forme non bornée,
    /// un peu au-dessus de 1.
    pub fn w(&self) -> f64 {
        self.w
    }

    /// Les nœuds de l'échelle de chroma du pont ([`chroma_nodes`]).
    pub fn chroma(&self) -> &[f32; CHROMA_N] {
        &self.chroma
    }

    /// Evora (EVORA-REV-110) : luminance de scène → luminance d'affichage, sans la borne à 1
    /// (`Pont.vers_affichage` sans son minimum, pont.py:32-33). Croissante ; elle passe 1 au-delà du
    /// blanc de scène et tend vers `W`. C'est la valeur que la table de tons d'Evora stockera : bornée
    /// avant d'être stockée, elle y mettrait un coude que l'interpolation de la table arrondit mal.
    ///
    /// Une luminance négative, ou qui n'est pas un nombre, rend 0. La forme n'est pas faite pour
    /// l'infini (elle y rend « pas un nombre ») : la table de tons s'arrête bien avant.
    #[inline]
    pub fn to_display_unbounded(&self, s: f32) -> f32 {
        let sn = s.max(0.0).powf(self.n32);
        self.w32 * sn / (sn + self.q32)
    }

    /// Evora (EVORA-REV-110) : luminance de scène → luminance d'affichage, de 0 à 1
    /// (`Pont.vers_affichage`, pont.py:30-33) : 1 au blanc de scène et au-delà.
    #[inline]
    pub fn to_display(&self, s: f32) -> f32 {
        self.to_display_unbounded(s).min(1.0)
    }

    /// Evora (EVORA-REV-110) : luminance d'affichage → luminance de scène, de 0 au blanc de scène
    /// (`Pont.vers_scene`, pont.py:35-38). L'entrée est bornée à 0..1 avant le calcul : `W − d` reste
    /// alors au-dessus de `W − 1`, sans division par zéro ni base négative sous la puissance. Une
    /// luminance d'affichage au-dessus de 1 se lit donc au blanc de scène, et son excès est perdu.
    #[inline]
    pub fn to_scene(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, 1.0);
        (self.q32 * d / (self.w32 - d)).powf(self.inv_n32)
    }

    /// Evora (EVORA-REV-110) : une couleur d'affichage (Rec.2020 linéaire) → la couleur de scène
    /// (`image_vers_scene`, pont.py:60-68, pour un point). La chroma est rendue d'abord, autour de la
    /// luminance d'affichage `o` ; puis la luminance passe le pont, par un gain commun aux trois canaux.
    ///
    /// À [`LUMINANCE_FLOOR`] et en dessous (noir, luminance négative), le gain vaut 1 et le point passe
    /// tel quel. Un clair saturé peut sortir avec un canal négatif : la chroma rendue l'écarte de sa
    /// luminance avant le gain.
    #[inline]
    pub fn scene_of(&self, display: [f32; 3]) -> [f32; 3] {
        let o = luminance_2020(display);
        let k = chroma_at(&self.chroma, o);
        // sous ce test strict, le plancher que la référence met au diviseur est sans objet
        let gain = if o > FLOOR { self.to_scene(o) / o } else { 1.0 };
        display.map(|v| (o + (v - o) / k) * gain)
    }

    /// Evora (EVORA-REV-110) : le pont aller sur une image entière, en place (`image_vers_scene`,
    /// pont.py:60-68) : [`Bridge::scene_of`] en chaque point.
    pub fn image_to_scene(&self, image: &mut Rgb32f) {
        image.map_in_place(|display| self.scene_of(display));
    }

    /// Evora (EVORA-REV-110) : une couleur de scène → la couleur d'affichage dont elle vient,
    /// l'inverse par point de [`Bridge::scene_of`]. La référence n'a pas cette fonction (elle garde
    /// l'image d'origine à côté de l'image de scène) ; le noyau, qui ne voit que la scène, s'en sert
    /// pour retrouver la source d'un point (retour de gamme, `stage::Kernel::gamut_return`).
    ///
    /// Luminance de scène, pont de retour borné à 1, gain commun aux trois canaux, puis chroma
    /// reprise. À [`LUMINANCE_FLOOR`] et en dessous le gain vaut 1, comme à l'aller. Une source dont
    /// la luminance dépassait 1 n'est pas retrouvée : [`Bridge::to_scene`] a perdu son excès.
    #[inline]
    pub fn source_of(&self, scene: [f32; 3]) -> [f32; 3] {
        let ys = luminance_2020(scene);
        let o = self.to_display(ys);
        let d = if ys > FLOOR { scene.map(|v| v * o / ys) } else { scene };
        let k = chroma_at(&self.chroma, o);
        d.map(|v| o + (v - o) * k)
    }
}

/// Evora (EVORA-REV-110) : les [`CHROMA_N`] nœuds de l'échelle de chroma du pont, régulièrement
/// espacés sur la luminance d'affichage de 0 à 1 (`noeuds_chroma`, pont.py:44-48) : 1 jusqu'à
/// [`CHROMA_FROM`], puis une baisse en douceur jusqu'à [`CHROMA_MIN`] au blanc. Calculés en double
/// précision, arrondis en simple précision.
pub fn chroma_nodes() -> [f32; CHROMA_N] {
    std::array::from_fn(|i| {
        let o = i as f64 / (CHROMA_N - 1) as f64;
        (1.0 - (1.0 - CHROMA_MIN) * smooth(CHROMA_FROM, 1.0, o)) as f32
    })
}

/// Evora (EVORA-REV-110) : l'échelle de chroma à la luminance d'affichage `o`, lue entre les nœuds
/// (`chroma_affichage`, pont.py:51-57). C'est l'arithmétique de
/// [`crate::tone::ToneMap::chroma_scale`], recopiée : la table de tons d'Evora portera ces mêmes
/// nœuds, et les deux lectures doivent rendre le même nombre.
///
/// `o` est borné à 0..1. La baisse commence dès le dernier nœud à 1, par l'interpolation, donc un
/// peu avant [`CHROMA_FROM`]. Si `o` n'est pas un nombre fini, l'échelle vaut 1 (garde du moteur ;
/// la référence y rend « pas un nombre »).
#[inline]
pub fn chroma_at(nodes: &[f32; CHROMA_N], o: f32) -> f32 {
    if !o.is_finite() {
        return 1.0;
    }
    let f = o.clamp(0.0, 1.0) * (CHROMA_N - 1) as f32;
    let i = (f as usize).min(CHROMA_N - 2);
    let t = f - i as f32;
    let (a, b) = (nodes[i], nodes[i + 1]);
    if a == b { a } else { a + (b - a) * t }
}

#[cfg(test)]
mod tests {
    //! Evora (EVORA-REV-110) : le pont de scène face aux vecteurs d'or de la référence.
    //!
    //! Les vecteurs et leurs tolérances sont recopiés par outil (`poser_rev110.py`, à côté des
    //! vecteurs d'or) depuis `tableaux-rust.txt`, que le générateur des vecteurs tire de la
    //! référence : entrées en simple précision, attendus en double précision. Ils ne se retouchent
    //! pas à la main ; ils se régénèrent. Chaque tolérance couvre au moins deux fois l'écart mesuré
    //! entre l'arithmétique du moteur en simple précision et la référence.

    use evolight_color::luminance_2020;
    use evolight_raster::Rgb32f;

    use super::{Bridge, FLOOR, chroma_at, chroma_nodes, constants};
    use crate::evora1::{BRIDGE_N, BRIDGE_WHITE, CHROMA_FROM, CHROMA_MIN, GREY, smooth};
    use crate::tone::CHROMA_N;

    // >>> or:PONT (EVORA-REV-110 : écrit par poser_rev110.py depuis tableaux-rust.txt, section EVORA-REV-110 ; ne pas retoucher)
    /// Evora (EVORA-REV-110) : `q` et `W` du pont de la référence (pont.py:22-28), en double précision.
    #[rustfmt::skip]
    const GOLD_BRIDGE_Q: f64 = 0.4983299769181831;
    #[rustfmt::skip]
    const GOLD_BRIDGE_W: f64 = 1.013556920066046;

    /// Evora (EVORA-REV-110) : (luminance de scène, affichage borné à 1, affichage non borné), pont.py:30-33.
    #[rustfmt::skip]
    const GOLD_BRIDGE_DISPLAY: [(f32, f64, f64); 31] = [
        (4.3945314e-05, 4.4067213412514865e-06, 4.4067213412514865e-06),
        (8.789063e-05, 1.0850551736254877e-05, 1.0850551736254877e-05),
        (0.00017578126, 2.6716774068133877e-05, 2.6716774068133877e-05),
        (0.0003515625, 6.578187870488986e-05, 6.578187870488986e-05),
        (0.000703125, 0.00016195861417366226, 0.00016195861417366226),
        (0.00140625, 0.0003986957258149082, 0.0003986957258149082),
        (0.0028125, 0.0009811396693520445, 0.0009811396693520445),
        (0.005625, 0.002412434397356707, 0.002412434397356707),
        (0.01125, 0.005919507468528532, 0.005919507468528532),
        (0.0225, 0.014452112443910418, 0.014452112443910418),
        (0.045, 0.03485845890216783, 0.03485845890216783),
        (0.09, 0.08172170012134913, 0.08172170012134913),
        (0.18, 0.18000000764701282, 0.18000000764701282),
        (0.36, 0.3518417392042245, 0.3518417392042245),
        (0.72, 0.5746409881610153, 0.5746409881610153),
        (1.44, 0.7735877984491544, 0.7735877984491544),
        (2.88, 0.9001541543436902, 0.9001541543436902),
        (5.76, 0.964223040761149, 0.964223040761149),
        (11.52, 0.9929247409307604, 0.9929247409307604),
        (23.04, 1.0, 1.005075114874536),
        (46.08, 1.0, 1.0100950320609254),
        (0.0, 0.0, 0.0),
        (-0.5, 0.0, 0.0),
        (1e-09, 4.058178207196799e-12, 4.058178207196799e-12),
        (1e-04, 1.2832923852784111e-05, 1.2832923852784111e-05),
        (1.0, 0.6764577467446556, 0.6764577467446556),
        (16.0, 1.0, 1.0),
        (20.0, 1.0, 1.0033793714274415),
        (64.0, 1.0, 1.0112956045310881),
        (184.32, 1.0, 1.0129842882167195),
        (1e+06, 1.0, 1.0135569120609755),
    ];

    /// Evora (EVORA-REV-110) : (luminance d'affichage, luminance de scène), pont.py:35-38. L'entrée est bornée à 0..1.
    #[rustfmt::skip]
    const GOLD_BRIDGE_SCENE: [(f32, f64); 19] = [
        (-0.1, 0.0),
        (0.0, 0.0),
        (1e-09, 6.914253134705425e-08),
        (1e-06, 1.4042289451516797e-05),
        (1e-04, 0.00048521261569528285),
        (0.001, 0.0028540375491636063),
        (0.01, 0.016891688795123067),
        (0.05, 0.06010775387095949),
        (0.1, 0.10673169120094285),
        (0.18, 0.18000000669007343),
        (0.25, 0.24792464234774242),
        (0.5, 0.5733019366985479),
        (0.75, 1.308263639246955),
        (0.9, 2.8766127159554062),
        (0.95, 4.686288168514266),
        (0.99, 10.379583023186518),
        (0.999, 15.13597154865267),
        (1.0, 15.99999999999998),
        (1.2, 15.99999999999998),
    ];

    /// Evora (EVORA-REV-110) : les huit nœuds de l'échelle de chroma (pont.py:44-48).
    #[rustfmt::skip]
    const GOLD_CHROMA_NODES: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 0.9750585390322277, 0.9058411987742399, 0.8334696506057613, 0.8];

    /// Evora (EVORA-REV-110) : (luminance d'affichage, échelle de chroma), pont.py:51-57.
    #[rustfmt::skip]
    const GOLD_CHROMA_AT: [(f32, f64); 21] = [
        (-0.25, 1.0),
        (0.0, 1.0),
        (0.05, 1.0),
        (0.14285715, 1.0),
        (0.3, 1.0),
        (0.42857143, 0.9999999992566866),
        (0.44, 0.9980046835388338),
        (0.45, 0.9962587829361118),
        (0.5, 0.9875292695161139),
        (0.5714286, 0.9750585266552028),
        (0.6, 0.9612150594287402),
        (0.7, 0.9127629385759836),
        (0.71428573, 0.9058411901468789),
        (0.75, 0.8877483117321202),
        (0.8, 0.8624182638340001),
        (0.85714287, 0.8334696486108146),
        (0.9, 0.8234287610098835),
        (0.95, 0.8117143805049417),
        (0.99, 0.8023428733080631),
        (1.0, 0.8),
        (1.5, 0.8),
    ];

    /// Evora (EVORA-REV-110) : (couleur d'affichage Rec.2020, couleur de scène), pont.py:60-68.
    /// Dans l'ordre : gris moyen ; gris clair ; blanc ; noir ; clair saturé ; sombre ; au-dessus de 1 ; canal négatif en sortie ; rouge pur ; bleu pur ; ton moyen coloré ; sous le plancher ; juste au-dessus du plancher ; luminance négative ; canal négatif en entrée.
    #[rustfmt::skip]
    const GOLD_IMAGE_TO_SCENE: [([f32; 3], [f64; 3]); 15] = [
        ([0.18, 0.18, 0.18], [0.18000000669007343, 0.18000000669007343, 0.18000000669007343]),
        ([0.5, 0.5, 0.5], [0.5733019366985479, 0.5733019366985479, 0.5733019366985479]),
        ([1.0, 1.0, 1.0], [15.99999999999998, 15.99999999999998, 15.99999999999998]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([0.95, 0.6, 0.1], [1.397944974447934, 0.8568984067539704, 0.08397462509047489]),
        ([0.02, 0.01, 0.03], [0.031448550450824446, 0.015724275225412223, 0.047172825676236665]),
        ([1.3, 1.1, 0.9], [18.79342058738905, 15.286741622483815, 11.780060567434289]),
        ([1.0, 1.0, 0.0], [4.511481888927509, 4.511481888927509, -0.9574491331521037]),
        ([1.0, 0.0, 0.0], [0.9931544017824733, 0.0, 0.0]),
        ([0.0, 0.0, 1.0], [0.0, 0.0, 1.1644050136964081]),
        ([0.6, 0.5, 0.4], [0.7047039755214286, 0.5855983499675285, 0.4664927599098623]),
        ([1e-10, 1e-10, 1e-10], [1.000000013351432e-10, 1.000000013351432e-10, 1.000000013351432e-10]),
        ([2e-09, 2e-09, 2e-09], [1.1784381963625225e-07, 1.1784381963625225e-07, 1.1784381963625225e-07]),
        ([-0.01, -0.02, 0.0], [-0.009999999776482582, -0.019999999552965164, 0.0]),
        ([0.9, -0.05, 0.2], [0.8928837652076486, -0.0496046556758887, 0.1984186227035548]),
    ];
    // <<< or:PONT

    // >>> or:TOLERANCES-PONT (EVORA-REV-110 : écrit par poser_rev110.py depuis tableaux-rust.txt, tolérances du pont ; ne pas retoucher)
    /// Evora : absolue, sur une luminance d'affichage (0 à W). Écart mesuré 1.84e-07. to_display et to_display_unbounded en f32 face à la référence en f64 ; environ trois derniers chiffres près de 1.
    #[rustfmt::skip]
    const GOLD_TOL_BRIDGE_DISPLAY: f64 = 1e-06;

    /// Evora : relative, sur une luminance de scène (0 à 16). Écart mesuré 2.13e-06. to_scene en f32 : W − d perd des chiffres près du blanc (W − 1 = 0,01356), l'écart relatif y est le plus grand.
    #[rustfmt::skip]
    const GOLD_TOL_BRIDGE_SCENE: f64 = 1e-05;

    /// Evora : absolue, affichage -> scène -> affichage en f32. Écart mesuré 1.19e-07. L'aller-retour par l'affichage est bien conditionné : la pente du pont de retour s'annule au blanc.
    #[rustfmt::skip]
    const GOLD_TOL_BRIDGE_ROUND_TRIP: f64 = 1e-06;

    /// Evora : relative, scène -> affichage -> scène en f32, s <= 16. Écart mesuré 5.97e-06. Sens mal conditionné près du blanc (facteur W / (n·(W − d)), environ 57 à d = 1).
    #[rustfmt::skip]
    const GOLD_TOL_BRIDGE_ROUND_TRIP_SCENE: f64 = 3e-05;

    /// Evora : absolue, sur un nœud arrondi en f32. Écart mesuré 5.96e-08. Nœuds calculés en f64 puis arrondis : au plus un demi-dernier chiffre, un dernier chiffre de plus pour les fonctions du runner.
    #[rustfmt::skip]
    const GOLD_TOL_CHROMA_NODE: f64 = 2.384185791015625e-07;

    /// Evora : absolue, sur une échelle de chroma (0,8 à 1). Écart mesuré 7.27e-08. Arithmétique de ToneMap::chroma_scale en f32 sur 140 001 luminances de −0,2 à 1,2.
    #[rustfmt::skip]
    const GOLD_TOL_CHROMA: f64 = 5e-07;

    /// Evora : relative à la plus grande composante de la couleur de scène attendue (plus 1e-7 absolu). Écart mesuré 3.46e-06. image_to_scene en f32 : l'écart vient de to_scene près du blanc (jusqu'à 3,9e-5 en absolu sur une scène proche de 16).
    #[rustfmt::skip]
    const GOLD_TOL_SCENE: f64 = 2e-05;

    /// Evora : absolue, source_of(image_to_scene(img)) face à img, luminance de 1e-9 à 1. Écart mesuré 2.98e-07. L'aller-retour finit en lumière d'affichage, où le pont de retour est bien conditionné.
    #[rustfmt::skip]
    const GOLD_TOL_SOURCE: f64 = 2e-06;
    // <<< or:TOLERANCES-PONT

    /// Evora (EVORA-REV-110) : écart relatif admis sur `q` et `W`, calculés en double précision des
    /// deux côtés. Seule la puissance peut différer d'un dernier chiffre entre le poste du relevé et
    /// le runner (2e-16) ; depuis le gris en simple précision du moteur, `q` s'écarte de 3e-9 et `W`
    /// de 5e-11 : la tolérance sépare les deux de plus de deux ordres de grandeur.
    const BRIDGE_CONSTANT_TOL: f64 = 1e-13;
    /// Evora (EVORA-REV-110) : la part absolue de `GOLD_TOL_SCENE`, que sa fiche donne en toutes
    /// lettres (« plus 1e-7 absolu ») : elle tient lieu d'échelle aux couleurs presque noires.
    const GOLD_TOL_SCENE_ABS: f64 = 1e-7;
    /// Evora (EVORA-REV-110) : pas des deux grilles d'aller-retour.
    const ROUND_TRIP_STEPS: usize = 4096;
    /// Evora (EVORA-REV-110) : côté du treillis de couleurs d'affichage de l'aller-retour par la scène.
    const LATTICE: usize = 17;

    fn within(got: f64, want: f64, tol: f64) -> bool {
        (got - want).abs() <= tol
    }

    fn within_rel(got: f64, want: f64, tol: f64) -> bool {
        (got - want).abs() <= tol * want.abs()
    }

    /// Une couleur de scène face à la couleur attendue : écart par canal rapporté à la plus grande
    /// composante attendue (`GOLD_TOL_SCENE`), plus `GOLD_TOL_SCENE_ABS`.
    fn scene_close(got: [f32; 3], want: [f64; 3]) -> bool {
        let scale = want.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let tol = GOLD_TOL_SCENE * scale + GOLD_TOL_SCENE_ABS;
        got.iter().zip(want.iter()).all(|(g, w)| within(f64::from(*g), *w, tol))
    }

    /// Une couleur d'affichage retrouvée face à la couleur d'origine, par canal (`GOLD_TOL_SOURCE`).
    fn source_close(got: [f32; 3], want: [f32; 3]) -> bool {
        got.iter().zip(want.iter()).all(|(g, w)| within(f64::from(*g), f64::from(*w), GOLD_TOL_SOURCE))
    }

    /// Evora (EVORA-REV-110) : le pont est ancré au gris moyen et au blanc. Ses deux constantes sont
    /// celles de la référence ; 0,18 de scène rend 0,18 d'affichage et inversement ; le blanc de
    /// scène rend 1, et 1 rend le blanc de scène ; au-delà, la forme bornée reste à 1 et la forme
    /// libre passe 1 sans dépasser `W`.
    ///
    /// Sabotages qui le font rougir : `q` et `W` calculés depuis le gris en simple précision du
    /// moteur (témoin intégré) ; un autre blanc de scène (témoin intégré) ; `W` remplacé par 1 (le
    /// blanc de scène rendrait 0,987) ; la borne à 1 retirée de `to_display`.
    #[test]
    fn bridge_is_anchored_at_grey_and_white() {
        let bridge = Bridge::default();
        assert!(within_rel(bridge.q(), GOLD_BRIDGE_Q, BRIDGE_CONSTANT_TOL), "q = {} pour {GOLD_BRIDGE_Q}", bridge.q());
        assert!(within_rel(bridge.w(), GOLD_BRIDGE_W, BRIDGE_CONSTANT_TOL), "W = {} pour {GOLD_BRIDGE_W}", bridge.w());
        // témoin rouge : depuis le gris du moteur, en simple précision puis élargi, les deux constantes
        // manquent la référence
        let (q_wide, w_wide) = constants(f64::from(crate::tone::GREY), BRIDGE_N, BRIDGE_WHITE);
        assert!(!within_rel(q_wide, GOLD_BRIDGE_Q, BRIDGE_CONSTANT_TOL), "témoin : q depuis le gris en simple précision passe la tolérance");
        assert!(!within_rel(w_wide, GOLD_BRIDGE_W, BRIDGE_CONSTANT_TOL), "témoin : W depuis le gris en simple précision passe la tolérance");

        let (grey, white) = (GREY as f32, BRIDGE_WHITE as f32);
        // le gris moyen ne bouge pas, dans les deux sens
        assert!(within(f64::from(bridge.to_display(grey)), GREY, GOLD_TOL_BRIDGE_DISPLAY), "gris de scène rendu {}", bridge.to_display(grey));
        assert!(within_rel(f64::from(bridge.to_scene(grey)), GREY, GOLD_TOL_BRIDGE_SCENE), "gris d'affichage lu {}", bridge.to_scene(grey));
        // le blanc de scène rend le blanc d'affichage, et inversement
        let at_white = bridge.to_display(white);
        assert!(at_white <= 1.0 && within(f64::from(at_white), 1.0, GOLD_TOL_BRIDGE_DISPLAY), "blanc de scène rendu {at_white}");
        assert!(within_rel(f64::from(bridge.to_scene(1.0)), BRIDGE_WHITE, GOLD_TOL_BRIDGE_SCENE), "blanc d'affichage lu {}", bridge.to_scene(1.0));
        // au-delà du blanc de scène : la forme bornée vaut 1, la forme libre passe 1 et reste sous W
        for s in [2.0 * white, 16.0 * white, 1e6] {
            assert!(bridge.to_display(s) == 1.0, "to_display({s}) = {}", bridge.to_display(s));
            let free = f64::from(bridge.to_display_unbounded(s));
            assert!(free > 1.0 + GOLD_TOL_BRIDGE_DISPLAY && free <= bridge.w() + GOLD_TOL_BRIDGE_DISPLAY, "to_display_unbounded({s}) = {free}");
        }
        assert!(within(f64::from(bridge.to_display_unbounded(1e6)), bridge.w(), GOLD_TOL_BRIDGE_DISPLAY), "la forme libre tend vers W");

        // témoin rouge : un pont ancré à un autre blanc de scène ne rend pas 1 au blanc de la référence ;
        // son gris, lui, reste ancré
        let other = Bridge::new(BRIDGE_N, 2.0 * BRIDGE_WHITE);
        assert!(!within(f64::from(other.to_display(white)), 1.0, GOLD_TOL_BRIDGE_DISPLAY), "témoin : un autre blanc de scène passe la tolérance");
        assert!(within(f64::from(other.to_display(grey)), GREY, GOLD_TOL_BRIDGE_DISPLAY), "le gris d'un autre pont n'est plus ancré");
    }

    /// Evora (EVORA-REV-110) : pont aller puis pont de retour rendent l'entrée. De l'affichage à la
    /// scène et retour, sur toute la plage 0..1 ; de la scène à l'affichage et retour, de −14 EV sous
    /// le gris (le bas de la table de tons du moteur) au blanc de scène. Ce second sens est mal
    /// conditionné près du blanc, d'où sa tolérance relative plus large.
    ///
    /// Sabotages qui le font rougir : l'exposant du retour pris à `n` au lieu de `1/n` ; `q` ou `W`
    /// différents entre les deux sens. Témoin intégré : au-delà du blanc de scène, l'aller-retour ne
    /// tient pas, la borne de `to_display` y ramenant tout au blanc. La borne de l'entrée de
    /// `to_scene` ne se voit pas ici (l'aller-retour ne lui donne que des entrées de 0 à 1) : c'est
    /// `bridge_matches_the_reference` qui l'éprouve, par ses entrées −0,1 et 1,2.
    #[test]
    fn bridge_round_trip_is_the_identity() {
        let bridge = Bridge::default();
        for i in 0..=ROUND_TRIP_STEPS {
            let d = i as f32 / ROUND_TRIP_STEPS as f32;
            let back = bridge.to_display(bridge.to_scene(d));
            assert!(within(f64::from(back), f64::from(d), GOLD_TOL_BRIDGE_ROUND_TRIP), "affichage {d} revenu à {back}");
        }
        let (low, high) = (f64::from(crate::tone::LUT_MIN_EV), (BRIDGE_WHITE / GREY).log2());
        let mut tried = 0;
        for i in 0..=ROUND_TRIP_STEPS {
            let ev = low + (high - low) * i as f64 / ROUND_TRIP_STEPS as f64;
            let s = (GREY * ev.exp2()) as f32;
            if f64::from(s) > BRIDGE_WHITE {
                continue; // l'arrondi du dernier pas peut passer le blanc de scène
            }
            tried += 1;
            let back = bridge.to_scene(bridge.to_display(s));
            assert!(within_rel(f64::from(back), f64::from(s), GOLD_TOL_BRIDGE_ROUND_TRIP_SCENE), "scène {s} revenue à {back}");
        }
        assert!(tried >= ROUND_TRIP_STEPS, "garde de portée : {tried} luminances de scène éprouvées");

        // témoin rouge : au-delà du blanc de scène, l'aller-retour ne tient plus (tout y revient au blanc)
        let beyond = (2.0 * BRIDGE_WHITE) as f32;
        let back = bridge.to_scene(bridge.to_display(beyond));
        assert!(!within_rel(f64::from(back), f64::from(beyond), GOLD_TOL_BRIDGE_ROUND_TRIP_SCENE), "témoin : {beyond} revient à {back}");
    }

    /// Evora (EVORA-REV-110) : les trois fonctions du pont face à la référence, sur ses vecteurs d'or
    /// (de −12 à +8 EV autour du gris, le noir, une luminance négative, le blanc de scène et au-delà).
    ///
    /// Sabotages qui le font rougir : un autre exposant, un autre blanc de scène ; `q` et `W` échangés ;
    /// la forme libre rendue à la place de la forme bornée (témoin intégré) ; la borne de l'entrée de
    /// `to_scene` retirée (1,2 d'affichage ne se lirait plus au blanc de scène).
    #[test]
    fn bridge_matches_the_reference() {
        let bridge = Bridge::default();
        for (s, bounded, free) in GOLD_BRIDGE_DISPLAY {
            let (got, got_free) = (bridge.to_display(s), bridge.to_display_unbounded(s));
            assert!(within(f64::from(got), bounded, GOLD_TOL_BRIDGE_DISPLAY), "to_display({s}) = {got} pour {bounded}");
            assert!(within(f64::from(got_free), free, GOLD_TOL_BRIDGE_DISPLAY), "to_display_unbounded({s}) = {got_free} pour {free}");
        }
        for (d, scene) in GOLD_BRIDGE_SCENE {
            let got = bridge.to_scene(d);
            assert!(within_rel(f64::from(got), scene, GOLD_TOL_BRIDGE_SCENE), "to_scene({d}) = {got} pour {scene}");
        }
        // témoin rouge : au-delà du blanc de scène, la forme libre manque les valeurs bornées de la référence
        let free_misses =
            GOLD_BRIDGE_DISPLAY.iter().any(|&(s, bounded, _)| !within(f64::from(bridge.to_display_unbounded(s)), bounded, GOLD_TOL_BRIDGE_DISPLAY));
        assert!(free_misses, "témoin : la forme libre passe pour la forme bornée sur tous les vecteurs");

        // hors des nombres finis, la forme bornée rend encore une luminance d'affichage de 0 à 1 (la
        // référence y rend « pas un nombre »)
        assert!(bridge.to_display(f32::NAN) == 0.0 && bridge.to_display(f32::NEG_INFINITY) == 0.0 && bridge.to_display(f32::INFINITY) == 1.0);
    }

    /// Evora (EVORA-REV-110) : les huit nœuds de l'échelle de chroma et sa lecture entre les nœuds,
    /// face à la référence. Les nœuds jusqu'à `CHROMA_FROM` valent exactement 1 (les tons sombres et
    /// moyens passent le pont sans retouche de chroma), le dernier vaut `CHROMA_MIN`.
    ///
    /// Sabotages qui le font rougir : `CHROMA_MIN` ou `CHROMA_FROM` changés ; les nœuds espacés sur
    /// huit intervalles au lieu de sept ; l'indice arrondi au plus proche au lieu d'être tronqué ; la
    /// loi lisse lue en continu au lieu de l'être entre les nœuds (témoin intégré).
    #[test]
    fn chroma_nodes_match_the_reference() {
        let nodes = chroma_nodes();
        assert!(Bridge::default().chroma() == &nodes, "le pont ne porte pas les nœuds de chroma_nodes");
        for (i, (node, gold)) in nodes.iter().zip(GOLD_CHROMA_NODES.iter()).enumerate() {
            assert!(within(f64::from(*node), *gold, GOLD_TOL_CHROMA_NODE), "nœud {i} : {node} pour {gold}");
            let o = i as f64 / (CHROMA_N - 1) as f64;
            if o <= CHROMA_FROM {
                assert!(*node == 1.0, "nœud {i}, sous CHROMA_FROM : {node} au lieu de 1");
            }
        }
        assert!(nodes[CHROMA_N - 1] == CHROMA_MIN as f32, "dernier nœud : {} au lieu de CHROMA_MIN", nodes[CHROMA_N - 1]);

        for (o, gold) in GOLD_CHROMA_AT {
            let got = chroma_at(&nodes, o);
            assert!(within(f64::from(got), gold, GOLD_TOL_CHROMA), "chroma_at({o}) = {got} pour {gold}");
        }
        // hors des nombres finis : la garde du moteur
        for o in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(chroma_at(&nodes, o) == 1.0, "chroma_at({o}) = {}", chroma_at(&nodes, o));
        }

        // témoin rouge : la baisse commence au dernier nœud à 1, par l'interpolation, et non à CHROMA_FROM ;
        // la loi lisse lue en continu manque la référence entre les deux
        let law_misses = GOLD_CHROMA_AT.iter().any(|&(o, gold)| {
            let law = 1.0 - (1.0 - CHROMA_MIN) * smooth(CHROMA_FROM, 1.0, f64::from(o));
            !within(law, gold, GOLD_TOL_CHROMA)
        });
        assert!(law_misses, "témoin : la loi lisse lue en continu passe la tolérance sur tous les vecteurs");
    }

    /// Evora (EVORA-REV-110) : le pont aller par point face à la référence, sur ses quinze couleurs
    /// (neutres, clair saturé, sombre, au-dessus de 1, primaires, canal négatif en entrée et en
    /// sortie, autour du plancher de luminance) ; et l'image entière, qui passe par la même fonction.
    ///
    /// Sabotages qui le font rougir : la chroma rendue retirée (témoin intégré) ou multipliée au lieu
    /// d'être divisée ; le gain pris sur la luminance d'après la chroma ; le test du plancher retiré
    /// (le noir diviserait par zéro) ou rendu large.
    #[test]
    fn image_to_scene_matches_the_reference() {
        let bridge = Bridge::default();
        for (display, scene) in GOLD_IMAGE_TO_SCENE {
            let got = bridge.scene_of(display);
            assert!(scene_close(got, scene), "scene_of({display:?}) = {got:?} pour {scene:?}");
        }
        let mut image = Rgb32f::from_fn(GOLD_IMAGE_TO_SCENE.len(), 1, |x, _| GOLD_IMAGE_TO_SCENE[x].0);
        bridge.image_to_scene(&mut image);
        for (point, (display, _)) in image.data.iter().zip(GOLD_IMAGE_TO_SCENE.iter()) {
            assert!(*point == bridge.scene_of(*display), "image_to_scene rend {point:?} pour {display:?}, scene_of {:?}", bridge.scene_of(*display));
        }

        // témoin rouge : le seul gain de luminance, sans la chroma rendue, manque la référence sur les
        // clairs colorés
        let gain_alone_misses = GOLD_IMAGE_TO_SCENE.iter().any(|&(display, scene)| {
            let o = luminance_2020(display);
            let gain = if o > FLOOR { bridge.to_scene(o) / o } else { 1.0 };
            !scene_close(display.map(|v| v * gain), scene)
        });
        assert!(gain_alone_misses, "témoin : le gain de luminance seul passe la tolérance sur tous les vecteurs");
    }

    /// Evora (EVORA-REV-110) : `source_of` défait `scene_of`. Sur les vecteurs d'or dont la luminance
    /// ne dépasse pas le blanc d'affichage (plancher compris : le point y passe tel quel dans les
    /// deux sens), puis sur un treillis de couleurs d'affichage de 0 à 1.
    ///
    /// Sabotages qui le font rougir : la chroma reprise avant le gain, ou divisée au lieu d'être
    /// multipliée ; sous le plancher, 0 rendu au lieu du point ; le pont de retour non borné, que
    /// l'aller-retour ne voit pas (la scène d'une couleur d'affichage ne passe jamais le blanc de
    /// scène) : une couleur de scène au-delà du blanc, passée seule à `source_of`, l'éprouve (sa
    /// source ne passe pas le blanc d'affichage). Témoin intégré : au-dessus du blanc d'affichage
    /// l'excès de la source est perdu, et l'aller-retour ne tient pas.
    #[test]
    fn source_of_inverts_image_to_scene() {
        let bridge = Bridge::default();
        let mut tried = 0;
        for (display, _) in GOLD_IMAGE_TO_SCENE {
            if luminance_2020(display) > 1.0 {
                continue;
            }
            tried += 1;
            let back = bridge.source_of(bridge.scene_of(display));
            assert!(source_close(back, display), "{display:?} revient à {back:?}");
        }
        assert!(tried > 0, "garde de portée : aucun vecteur d'or sous le blanc d'affichage");
        for i in 0..LATTICE.pow(3) {
            let axis = |k: usize| (k % LATTICE) as f32 / (LATTICE - 1) as f32;
            let display = [axis(i), axis(i / LATTICE), axis(i / (LATTICE * LATTICE))];
            let back = bridge.source_of(bridge.scene_of(display));
            assert!(source_close(back, display), "{display:?} revient à {back:?}");
        }

        // le pont de retour est borné : une couleur de scène au-delà du blanc rend au plus le blanc
        let beyond = bridge.source_of([(2.0 * BRIDGE_WHITE) as f32; 3]);
        assert!(
            beyond.iter().all(|v| f64::from(*v) <= 1.0 + GOLD_TOL_SOURCE),
            "au-delà du blanc de scène, la source retrouvée {beyond:?} passe le blanc d'affichage"
        );

        // témoin rouge : au-dessus du blanc d'affichage, la source n'est pas retrouvée
        let above_white_misses = GOLD_IMAGE_TO_SCENE
            .iter()
            .any(|&(display, _)| luminance_2020(display) > 1.0 && !source_close(bridge.source_of(bridge.scene_of(display)), display));
        assert!(above_white_misses, "témoin : une source au-dessus du blanc d'affichage est retrouvée");
    }
}
