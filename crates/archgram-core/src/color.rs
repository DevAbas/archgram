//! Colour arithmetic that must give the same answer on every machine
//! (ARCHITECTURE.md, Invariants): no call into the platform's maths library,
//! whose last digits may differ.

use crate::tokens::{Colors, Role};

/// Each 8-bit sRGB channel value in linear light (IEC 61966-2-1), written
/// out so no `powf` runs: `c / 12.92` up to 0.04045, else
/// `((c + 0.055) / 1.055)^2.4`, with `c` the value over 255.
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
const LINEAR: [f64; 256] = [
    0.0,
    0.0003035269835488375,
    0.000607053967097675,
    0.0009105809506465125,
    0.00121410793419535,
    0.0015176349177441874,
    0.001821161901293025,
    0.0021246888848418626,
    0.0024282158683907,
    0.0027317428519395373,
    0.003035269835488375,
    0.003346535763899161,
    0.003676507324047436,
    0.004024717018496307,
    0.004391442037410293,
    0.004776953480693729,
    0.005181516702338386,
    0.005605391624202723,
    0.006048833022857054,
    0.006512090792594475,
    0.006995410187265387,
    0.007499032043226175,
    0.008023192985384994,
    0.008568125618069307,
    0.009134058702220787,
    0.00972121732023785,
    0.010329823029626936,
    0.010960094006488246,
    0.011612245179743885,
    0.012286488356915872,
    0.012983032342173012,
    0.013702083047289686,
    0.014443843596092545,
    0.01520851442291271,
    0.01599629336550963,
    0.016807375752887384,
    0.017641954488384078,
    0.018500220128379697,
    0.019382360956935723,
    0.0202885630566524,
    0.021219010376003555,
    0.02217388479338738,
    0.02315336617811041,
    0.024157632448504756,
    0.02518685962736163,
    0.026241221894849898,
    0.027320891639074894,
    0.028426039504420793,
    0.0295568344378088,
    0.030713443732993635,
    0.03189603307301153,
    0.033104766570885055,
    0.03433980680868217,
    0.03560131487502034,
    0.03688945040110004,
    0.0382043715953465,
    0.03954623527673284,
    0.04091519690685319,
    0.042311410620809675,
    0.043735029256973465,
    0.04518620438567554,
    0.046665086336880095,
    0.04817182422688942,
    0.04970656598412723,
    0.05126945837404324,
    0.052860647023180246,
    0.05448027644244237,
    0.05612849004960009,
    0.05780543019106723,
    0.0595112381629812,
    0.06124605423161761,
    0.06301001765316767,
    0.06480326669290577,
    0.06662593864377289,
    0.06847816984440017,
    0.07036009569659588,
    0.07227185068231748,
    0.07421356838014963,
    0.07618538148130785,
    0.07818742180518633,
    0.08021982031446832,
    0.0822827071298148,
    0.08437621154414882,
    0.08650046203654976,
    0.08865558628577294,
    0.09084171118340768,
    0.09305896284668745,
    0.0953074666309647,
    0.09758734714186246,
    0.09989872824711389,
    0.10224173308810132,
    0.10461648409110419,
    0.10702310297826761,
    0.10946171077829933,
    0.1119324278369056,
    0.11443537382697373,
    0.11697066775851084,
    0.11953842798834562,
    0.12213877222960187,
    0.12477181756095049,
    0.12743768043564743,
    0.1301364766903643,
    0.13286832155381798,
    0.13563332965520566,
    0.13843161503245183,
    0.14126329114027164,
    0.14412847085805777,
    0.14702726649759498,
    0.14995978981060856,
    0.15292615199615017,
    0.1559264637078274,
    0.1589608350608804,
    0.162029375639111,
    0.1651321945016676,
    0.16826940018969075,
    0.1714411007328226,
    0.17464740365558504,
    0.17788841598362912,
    0.18116424424986022,
    0.184474994500441,
    0.18782077230067787,
    0.19120168274079138,
    0.1946178304415758,
    0.19806931955994886,
    0.20155625379439707,
    0.20507873639031693,
    0.20863687014525575,
    0.21223075741405523,
    0.21586050011389926,
    0.2195261997292692,
    0.2232279573168085,
    0.22696587351009836,
    0.23074004852434915,
    0.23455058216100522,
    0.238397573812271,
    0.24228112246555486,
    0.24620132670783548,
    0.25015828472995344,
    0.25415209433082675,
    0.2581828529215958,
    0.26225065752969623,
    0.26635560480286247,
    0.2704977910130658,
    0.27467731206038465,
    0.2788942634768104,
    0.2831487404299921,
    0.2874408377269175,
    0.29177064981753587,
    0.2961382707983211,
    0.3005437944157765,
    0.3049873140698863,
    0.30946892281750854,
    0.31398871337571754,
    0.31854677812509186,
    0.32314320911295075,
    0.3277780980565422,
    0.33245153634617935,
    0.33716361504833037,
    0.3419144249086609,
    0.3467040563550296,
    0.35153259950043936,
    0.3564001441459435,
    0.3613067797835095,
    0.3662525955988395,
    0.3712376804741491,
    0.3762621229909065,
    0.38132601143253014,
    0.386429433787049,
    0.39157247774972326,
    0.39675523072562685,
    0.4019777798321958,
    0.4072402119017367,
    0.41254261348390375,
    0.4178850708481375,
    0.4232676699860717,
    0.4286904966139066,
    0.43415363617474895,
    0.4396571738409188,
    0.44520119451622786,
    0.45078578283822346,
    0.45641102318040466,
    0.4620769996544071,
    0.467783796112159,
    0.47353149614800955,
    0.4793201831008268,
    0.4851499400560704,
    0.4910208498478356,
    0.4969329950608704,
    0.5028864580325687,
    0.5088813208549338,
    0.5149176653765214,
    0.5209955732043543,
    0.5271151257058131,
    0.5332764040105052,
    0.5394794890121072,
    0.5457244613701866,
    0.5520114015120001,
    0.5583403896342679,
    0.5647115057049292,
    0.5711248294648731,
    0.5775804404296506,
    0.5840784178911641,
    0.5906188409193369,
    0.5972017883637634,
    0.6038273388553378,
    0.6104955708078648,
    0.6172065624196511,
    0.6239603916750761,
    0.6307571363461468,
    0.6375968739940326,
    0.6444796819705821,
    0.6514056374198242,
    0.6583748172794485,
    0.665387298282272,
    0.6724431569576875,
    0.6795424696330938,
    0.6866853124353135,
    0.6938717612919899,
    0.7011018919329731,
    0.7083757798916868,
    0.7156935005064807,
    0.7230551289219693,
    0.7304607400903537,
    0.7379104087727308,
    0.7454042095403874,
    0.7529422167760779,
    0.7605245046752924,
    0.768151147247507,
    0.7758222183174236,
    0.7835377915261935,
    0.7912979403326302,
    0.799102738014409,
    0.8069522576692516,
    0.8148465722161012,
    0.8227857543962835,
    0.8307698767746546,
    0.83879901174074,
    0.846873231509858,
    0.8549926081242338,
    0.8631572134541023,
    0.8713671191987972,
    0.8796223968878317,
    0.8879231178819663,
    0.8962693533742664,
    0.9046611743911496,
    0.9130986517934192,
    0.9215818562772946,
    0.9301108583754237,
    0.938685728457888,
    0.9473065367331999,
    0.9559733532492861,
    0.9646862478944651,
    0.9734452903984125,
    0.9822505503331171,
    0.9911020971138298,
    1.0,
];

/// An opaque colour, eight bits a channel: what a role holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    /// An `#rrggbb` or `rrggbb` colour.
    #[must_use]
    pub fn parse(hex: &str) -> Option<Rgb> {
        let h = hex.strip_prefix('#').unwrap_or(hex);
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Rgb([byte(0)?, byte(2)?, byte(4)?]))
    }

    /// Relative luminance (WCAG 2.1), from 0 for black to 1 for white.
    #[must_use]
    pub fn luminance(self) -> f64 {
        let [r, g, b] = self.0.map(|c| LINEAR[usize::from(c)]);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// `#rrggbb`, in lower case.
impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [r, g, b] = self.0;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

/// The contrast ratio of two colours (WCAG 2.1), from 1 to 21.
#[must_use]
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (x, y) = (a.luminance(), b.luminance());
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The colour spaces a DTCG colour may be read from; any other falls back
/// to the value's `hex`.
pub const SPACES: [&str; 7] = [
    "srgb",
    "srgb-linear",
    "hsl",
    "hwb",
    "oklab",
    "oklch",
    "display-p3",
];

/// A resolved DTCG colour value (Design Tokens 2025.10, Color): an object
/// with `colorSpace`, `components` (each a number or `none`, read as 0),
/// an optional `alpha` and an optional `hex`, or an older `#rrggbb` string.
/// Out of sRGB's gamut, each channel is clipped. Role colours are opaque, so
/// an `alpha` below 1 is refused.
///
/// # Errors
///
/// What makes the value unreadable, in words.
pub fn from_dtcg(value: &serde_json::Value) -> Result<Rgb, String> {
    use serde_json::Value;
    if let Value::String(hex) = value {
        return Rgb::parse(hex).ok_or_else(|| format!("`{hex}` is not a `#rrggbb` colour"));
    }
    let Value::Object(o) = value else {
        return Err("a colour is an object with `colorSpace` and `components`".into());
    };
    if let Some(alpha) = o.get("alpha") {
        let a = alpha.as_f64().ok_or("`alpha` is a number")?;
        if a < 1.0 {
            return Err(format!("`alpha` is {a}; a role's colour is opaque"));
        }
    }
    let space = o
        .get("colorSpace")
        .and_then(Value::as_str)
        .ok_or("a colour needs `colorSpace`")?;
    let components: Vec<f64> = o
        .get("components")
        .and_then(Value::as_array)
        .ok_or("a colour needs `components`")?
        .iter()
        .map(|c| match c {
            Value::String(n) if n == "none" => Ok(0.0),
            _ => c.as_f64().ok_or_else(|| format!("`{c}` is not a component")),
        })
        .collect::<Result<_, _>>()?;
    let [x, y, z] = <[f64; 3]>::try_from(components.as_slice())
        .map_err(|_| format!("`{space}` takes three components, not {}", components.len()))?;
    let rgb = match space {
        "srgb" => [x, y, z],
        "srgb-linear" => [x, y, z].map(encode),
        "hsl" => hsl(x, y / 100.0, z / 100.0),
        "hwb" => hwb(x, y / 100.0, z / 100.0),
        "oklab" => oklab(x, y, z),
        "oklch" => {
            let (sin, cos) = crate::math::sin_cos_degrees(z);
            oklab(x, y * cos, y * sin)
        }
        "display-p3" => display_p3([x, y, z]),
        other => {
            return match o.get("hex").and_then(Value::as_str) {
                Some(hex) => Rgb::parse(hex).ok_or_else(|| format!("`{hex}` is not a `#rrggbb` colour")),
                None => Err(format!(
                    "`{other}` is not read, and the colour has no `hex` to fall back on; archgram reads {}",
                    SPACES.join(", ")
                )),
            };
        }
    };
    Ok(Rgb(rgb.map(|c| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let byte = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        byte
    })))
}

/// Linear light to the sRGB curve (IEC 61966-2-1), sign kept past black.
fn encode(c: f64) -> f64 {
    let a = c.abs();
    let v = if a <= 0.003_130_8 {
        12.92 * a
    } else {
        1.055 * crate::math::pow(a, 1.0 / 2.4) - 0.055
    };
    if c < 0.0 { -v } else { v }
}

/// The sRGB curve to linear light, sign kept past black.
fn decode(c: f64) -> f64 {
    let a = c.abs();
    let v = if a <= 0.040_45 {
        a / 12.92
    } else {
        crate::math::pow((a + 0.055) / 1.055, 2.4)
    };
    if c < 0.0 { -v } else { v }
}

/// HSL to sRGB (CSS Color 4, `hslToRgb`): hue in degrees, the others 0 to 1.
fn hsl(hue: f64, saturation: f64, lightness: f64) -> [f64; 3] {
    let h = hue - 360.0 * (hue / 360.0).floor();
    let a = saturation * lightness.min(1.0 - lightness);
    [0.0, 8.0, 4.0].map(|n: f64| {
        let k = (n + h / 30.0) % 12.0;
        lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    })
}

/// HWB to sRGB (CSS Color 4, `hwbToRgb`).
fn hwb(hue: f64, white: f64, black: f64) -> [f64; 3] {
    if white + black >= 1.0 {
        let grey = white / (white + black);
        return [grey; 3];
    }
    hsl(hue, 1.0, 0.5).map(|c| c * (1.0 - white - black) + white)
}

/// `OKLab` to sRGB (Björn Ottosson, "A perceptual color space for image
/// processing", 2020): to LMS, cubed, to linear sRGB, then the curve.
fn oklab(l: f64, a: f64, b: f64) -> [f64; 3] {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
        -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701 * s3,
    ]
    .map(encode)
}

/// Display P3 to sRGB (CSS Color 4): its curve is sRGB's; its primaries go
/// through CIE XYZ (D65) to sRGB's.
fn display_p3(rgb: [f64; 3]) -> [f64; 3] {
    const P3_TO_XYZ: [[f64; 3]; 3] = [
        [
            0.486_570_948_648_216_2,
            0.265_667_693_169_093_06,
            0.198_217_285_234_362_5,
        ],
        [
            0.228_974_564_069_748_8,
            0.691_738_521_836_506_4,
            0.079_286_914_093_745,
        ],
        [0.0, 0.045_113_381_858_902_64, 1.043_944_368_900_976],
    ];
    const XYZ_TO_SRGB: [[f64; 3]; 3] = [
        [
            3.240_969_941_904_522_6,
            -1.537_383_177_570_094,
            -0.498_610_760_293_003_4,
        ],
        [
            -0.969_243_636_280_879_6,
            1.875_967_501_507_720_2,
            0.041_555_057_407_175_59,
        ],
        [
            0.055_630_079_696_993_66,
            -0.203_976_958_888_976_52,
            1.056_971_514_242_878_6,
        ],
    ];
    let times = |m: &[[f64; 3]; 3], v: [f64; 3]| m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2]);
    times(&XYZ_TO_SRGB, times(&P3_TO_XYZ, rgb.map(decode))).map(encode)
}

/// The pairs DESIGN.md holds to WCAG 2.1 AA (Colors): text 4.5:1 on what
/// it sits on, lines and icons 3:1. Each is a foreground role, the role
/// behind it and the least ratio.
pub const PAIRS: [(Role, Role, f64); 14] = [
    (Role::Text, Role::Card, 4.5),
    (Role::TextMuted, Role::Card, 4.5),
    (Role::Text, Role::Canvas, 4.5),
    (Role::TextMuted, Role::Canvas, 4.5),
    (Role::IconCore, Role::Badge, 3.0),
    (Role::IconAi, Role::Badge, 3.0),
    (Role::IconBuild, Role::Badge, 3.0),
    (Role::IconClient, Role::Badge, 3.0),
    (Role::IconCore, Role::Canvas, 3.0),
    (Role::IconAi, Role::Canvas, 3.0),
    (Role::IconBuild, Role::Canvas, 3.0),
    (Role::IconClient, Role::Canvas, 3.0),
    (Role::Connector, Role::Canvas, 3.0),
    (Role::Frame, Role::Canvas, 3.0),
];

/// A pair below its least contrast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shortfall {
    pub foreground: Role,
    pub background: Role,
    pub ratio: f64,
    pub least: f64,
}

impl std::fmt::Display for Shortfall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Rounded down, so a ratio shown as the least is never below it.
        let shown = (self.ratio * 100.0).floor() / 100.0;
        write!(
            f,
            "`{}` on `{}` is {}:1, below {}:1",
            self.foreground.name(),
            self.background.name(),
            crate::render::svg::num(shown),
            crate::render::svg::num(self.least)
        )
    }
}

/// Every pair of `colors` below its least contrast (`PAIRS`).
#[must_use]
pub fn check(colors: &Colors) -> Vec<Shortfall> {
    PAIRS
        .iter()
        .filter_map(|&(foreground, background, least)| {
            let ratio = contrast(colors.get(foreground), colors.get(background));
            (ratio < least).then_some(Shortfall {
                foreground,
                background,
                ratio,
                least,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_the_srgb_curve() {
        for (i, &v) in LINEAR.iter().enumerate() {
            let c = f64::from(u8::try_from(i).unwrap()) / 255.0;
            let want = if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
            assert!((v - want).abs() < 1e-12, "{i}: {v} against {want}");
        }
    }

    fn read(json: &str) -> Result<Rgb, String> {
        from_dtcg(&serde_json::from_str(json).unwrap())
    }

    #[test]
    fn each_space_reads_to_the_colour_css_gives() {
        // CSS Color 4's conversions, computed apart (Python, its own libm)
        // and rounded to eight bits; none lands near a rounding boundary.
        let cases = [
            (
                r#"{ "colorSpace": "srgb", "components": [0.1843, 0.4157, 0.6588] }"#,
                "#2f6aa8",
            ),
            (
                r#"{ "colorSpace": "srgb-linear", "components": [0.5, 0.3, 0.05] }"#,
                "#bc953f",
            ),
            (
                r#"{ "colorSpace": "hsl", "components": [200, 60, 40] }"#,
                "#297aa3",
            ),
            (
                r#"{ "colorSpace": "hsl", "components": [-30, 70, 62] }"#,
                "#e25a9e",
            ),
            (
                r#"{ "colorSpace": "hwb", "components": [40, 12, 33] }"#,
                "#ab7c1f",
            ),
            (
                r#"{ "colorSpace": "hwb", "components": ["none", 70, 60] }"#,
                "#898989",
            ),
            (
                r#"{ "colorSpace": "oklab", "components": [0.627955, 0.224863, 0.125846] }"#,
                "#ff0000",
            ),
            (
                r#"{ "colorSpace": "oklch", "components": [0.519752, 0.1795, 264.052] }"#,
                "#315fce",
            ),
            (
                r#"{ "colorSpace": "oklch", "components": [0.7, 0.12, 145] }"#,
                "#6cb26f",
            ),
            (
                r#"{ "colorSpace": "oklch", "components": [1, 0, "none"] }"#,
                "#ffffff",
            ),
            (
                r#"{ "colorSpace": "display-p3", "components": [0.3, 0.6, 0.8] }"#,
                "#249bd1",
            ),
            (
                r#"{ "colorSpace": "display-p3", "components": [1, 0, 0] }"#,
                "#ff0000",
            ),
            (
                r##"{ "colorSpace": "lab", "components": [50, 0, 0], "hex": "#777777" }"##,
                "#777777",
            ),
            (r##""#ABCDEF""##, "#abcdef"),
        ];
        for (json, want) in cases {
            assert_eq!(read(json).map(|c| c.to_string()), Ok(want.to_owned()), "{json}");
        }
    }

    #[test]
    fn unreadable_colours_say_why() {
        assert!(
            read(r#"{ "colorSpace": "lab", "components": [50, 0, 0] }"#)
                .unwrap_err()
                .contains("no `hex`")
        );
        assert!(
            read(r#"{ "colorSpace": "srgb", "components": [1, 0, 0], "alpha": 0.5 }"#)
                .unwrap_err()
                .contains("opaque")
        );
        assert!(
            read(r#"{ "colorSpace": "srgb", "components": [1, 0] }"#)
                .unwrap_err()
                .contains("three")
        );
    }

    #[test]
    fn black_on_white_is_21_to_1() {
        let (black, white) = (Rgb::parse("#000000").unwrap(), Rgb::parse("ffffff").unwrap());
        assert!((contrast(black, white) - 21.0).abs() < 1e-12);
        assert!((contrast(white, white) - 1.0).abs() < 1e-12);
        assert_eq!(Rgb::parse("#fff"), None);
        assert_eq!(Rgb([0x2f, 0x6a, 0xa8]).to_string(), "#2f6aa8");
    }
}
