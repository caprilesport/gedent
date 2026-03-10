/// A [chemical element](https://en.wikipedia.org/wiki/Chemical_element).
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    strum::Display,
    strum::EnumString,
    strum::FromRepr,
)]
#[repr(u8)]
#[strum(ascii_case_insensitive)]
pub enum Element {
    /// Dummy, pseudo or virtual element.
    X,
    /// Hydrogen.
    H,
    /// Helium.
    He,
    /// Lithium.
    Li,
    /// Beryllium.
    Be,
    /// Boron.
    B,
    /// Carbon.
    C,
    /// Nitrogen.
    N,
    /// Oxygen.
    O,
    /// Fluorine.
    F,
    /// Neon.
    Ne,
    /// Sodium.
    Na,
    /// Magnesium.
    Mg,
    /// Aluminium.
    Al,
    /// Silicon.
    Si,
    /// Phosphorus.
    P,
    /// Sulfur.
    S,
    /// Chlorine.
    Cl,
    /// Argon.
    Ar,
    /// Potassium.
    K,
    /// Calcium.
    Ca,
    /// Scandium.
    Sc,
    /// Titanium.
    Ti,
    /// Vanadium.
    V,
    /// Chromium.
    Cr,
    /// Manganese.
    Mn,
    /// Iron.
    Fe,
    /// Cobalt.
    Co,
    /// Nickel.
    Ni,
    /// Copper.
    Cu,
    /// Zinc.
    Zn,
    /// Gallium.
    Ga,
    /// Germanium.
    Ge,
    /// Arsenic.
    As,
    /// Selenium.
    Se,
    /// Bromine.
    Br,
    /// Krypton.
    Kr,
    /// Rubidium.
    Rb,
    /// Strontium.
    Sr,
    /// Yttrium.
    Y,
    /// Zirconium.
    Zr,
    /// Niobium.
    Nb,
    /// Molybdenum.
    Mo,
    /// Technetium.
    Tc,
    /// Ruthenium.
    Ru,
    /// Rhodium.
    Rh,
    /// Palladium.
    Pd,
    /// Silver.
    Ag,
    /// Cadmium.
    Cd,
    /// Indium.
    In,
    /// Tin.
    Sn,
    /// Antimony.
    Sb,
    /// Tellurium.
    Te,
    /// Iodine.
    I,
    /// Xenon.
    Xe,
    /// Cesium.
    Cs,
    /// Barium.
    Ba,
    /// Lanthanum.
    La,
    /// Cerium.
    Ce,
    /// Praseodymium.
    Pr,
    /// Neodymium.
    Nd,
    /// Promethium.
    Pm,
    /// Samarium.
    Sm,
    /// Europium.
    Eu,
    /// Gadolinium.
    Gd,
    /// Terbium.
    Tb,
    /// Dysprosium.
    Dy,
    /// Holmium.
    Ho,
    /// Erbium.
    Er,
    /// Thulium.
    Tm,
    /// Ytterbium.
    Yb,
    /// Lutetium.
    Lu,
    /// Hafnium.
    Hf,
    /// Tantalum.
    Ta,
    /// Tungsten.
    W,
    /// Rhenium.
    Re,
    /// Osmium.
    Os,
    /// Iridium.
    Ir,
    /// Platinum.
    Pt,
    /// Gold.
    Au,
    /// Mercury.
    Hg,
    /// Thallium.
    Tl,
    /// Lead.
    Pb,
    /// Bismuth.
    Bi,
    /// Polonium.
    Po,
    /// Astatine.
    At,
    /// Radon.
    Rn,
    /// Francium.
    Fr,
    /// Radium.
    Ra,
    /// Actinium.
    Ac,
    /// Thorium.
    Th,
    /// Protactinium.
    Pa,
    /// Uranium.
    U,
    /// Neptunium.
    Np,
    /// Plutonium.
    Pu,
    /// Americium.
    Am,
    /// Curium.
    Cm,
    /// Berkelium.
    Bk,
    /// Californium.
    Cf,
    /// Einsteinium.
    Es,
    /// Fermium.
    Fm,
    /// Mendelevium.
    Md,
    /// Nobelium.
    No,
    /// Lawrencium.
    Lr,
    /// Rutherfordium.
    Rf,
    /// Dubnium.
    Db,
    /// Seaborgium.
    Sg,
    /// Bohrium.
    Bh,
    /// Hassium.
    Hs,
    /// Meitnerium.
    Mt,
    /// Darmstadtium.
    Ds,
    /// Roentgenium.
    Rg,
    /// Copernicium.
    Cn,
    /// Nihonium.
    Nh,
    /// Flerovium.
    Fl,
    /// Moscovium.
    Mc,
    /// Livermorium.
    Lv,
    /// Tennessine.
    Ts,
    /// Oganesson.
    Og,
}

/// Parsing error.
///
/// Reexported so that users don't have to depend on [`strum`].
#[allow(dead_code)]
pub type ParseElementError = strum::ParseError;

impl serde::Serialize for Element {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

#[allow(dead_code)]
impl Element {
    /// Try to create an [`Element`] from its atomic number.
    ///
    /// This is the same as [`Element::from_repr`], except that this function
    /// will never return a dummy element (see examples below).
    ///
    /// # Examples
    /// ```rust,ignore
    /// use crate::elements::Element as E;
    ///
    /// assert_eq!(E::from_atomic_number(6), Some(E::C));
    ///
    /// // Compare
    /// assert_eq!(E::from_atomic_number(0), None);
    /// assert_eq!(E::from_repr(0), Some(E::X));
    ///
    /// // Compare
    /// assert_eq!(E::from_atomic_number(1), Some(E::H));
    /// assert_eq!(E::from_atomic_number(1), E::from_repr(1));
    ///
    /// // Call me when this gets discovered
    /// assert_eq!(E::from_atomic_number(119), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn from_atomic_number(n: u8) -> Option<Self> {
        match n {
            0 => None,
            n => Self::from_repr(n),
        }
    }

    /// Covalent radius (Alvarez, 2008).
    ///
    /// Values based on a statistical analysis of more than 228000 experimental bond lengths from the Cambridge Structural Database (see the [original publication](https://doi.org/10.1039%2Fb801115j) and [Wikipedia](https://en.wikipedia.org/wiki/Covalent_radius#Average_radii)). Also in agreement with [QCElemental](https://github.com/MolSSI/QCElemental/blob/e942b810f1681b7d38209d0fed55b49954e6e4b5/qcelemental/data/alvarez_2008_covalent_radii.py#L22-L124).
    ///
    /// For carbon, manganese, iron and cobalt, values are averaged out from
    /// different hybridisations and/or spin states.
    #[allow(clippy::too_many_lines, clippy::use_self)]
    pub fn get_radius(self) -> Option<f32> {
        match self {
            Element::H => Some(0.31),
            Element::He => Some(0.28),
            Element::Li => Some(1.28),
            Element::Be => Some(0.96),
            Element::B => Some(0.84),
            Element::C => {
                let (sp3, n_sp3) = (0.76, 10000_f64);
                let (sp2, n_sp2) = (0.73, 10000_f64);
                let (sp, n_sp) = (0.69, 171_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(sp3, n_sp3), (sp2, n_sp2), (sp, n_sp)])
                        .mean() as f32,
                )
            }
            Element::N => Some(0.71),
            Element::O => Some(0.66),
            Element::F => Some(0.57),
            Element::Ne => Some(0.58),
            Element::Na => Some(1.66),
            Element::Mg | Element::Ir => Some(1.41),
            Element::Al => Some(1.21),
            Element::Si => Some(1.11),
            Element::P => Some(1.07),
            Element::S => Some(1.05),
            Element::Cl => Some(1.02),
            Element::Ar => Some(1.06),
            Element::K | Element::Pr => Some(2.03),
            Element::Ca => Some(1.76),
            Element::Sc | Element::Ta => Some(1.70),
            Element::Ti => Some(1.60),
            Element::V => Some(1.53),
            Element::Cr | Element::Pd | Element::Sn | Element::Sb | Element::I => Some(1.39),
            Element::Mn => {
                let (lowspin, n_lowspin) = (1.39, 321_f64);
                let (highspin, n_highspin) = (1.61, 929_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Element::Fe => {
                let (lowspin, n_lowspin) = (1.32, 336_f64);
                let (highspin, n_highspin) = (1.52, 1540_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Element::Co => {
                let (lowspin, n_lowspin) = (1.26, 5733_f64);
                let (highspin, n_highspin) = (1.50, 780_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Element::Ni => Some(1.24),
            Element::Cu | Element::Hg => Some(1.32),
            Element::Zn | Element::Ga => Some(1.22),
            Element::Ge | Element::Se | Element::Br => Some(1.20),
            Element::As => Some(1.19),
            Element::Kr => Some(1.16),
            Element::Rb => Some(2.20),
            Element::Sr => Some(1.95),
            Element::Y | Element::Tm | Element::Np => Some(1.90),
            Element::Zr | Element::Hf => Some(1.75),
            Element::Nb => Some(1.64),
            Element::Mo => Some(1.54),
            Element::Tc => Some(1.47),
            Element::Ru | Element::Pb => Some(1.46),
            Element::Rh | Element::In => Some(1.42),
            Element::Ag | Element::Tl => Some(1.45),
            Element::Cd | Element::Os => Some(1.44),
            Element::Te => Some(1.38),
            Element::Xe | Element::Po => Some(1.40),
            Element::Cs => Some(2.44),
            Element::Ba | Element::Ac => Some(2.15),
            Element::La => Some(2.07),
            Element::Ce => Some(2.04),
            Element::Nd => Some(2.01),
            Element::Pm => Some(1.99),
            Element::Sm | Element::Eu => Some(1.98),
            Element::Gd | Element::U => Some(1.96),
            Element::Tb => Some(1.94),
            Element::Dy | Element::Ho => Some(1.92),
            Element::Er => Some(1.89),
            Element::Yb | Element::Lu | Element::Pu => Some(1.87),
            Element::W => Some(1.62),
            Element::Re => Some(1.51),
            Element::Pt | Element::Au => Some(1.36),
            Element::Bi => Some(1.48),
            Element::At | Element::Rn => Some(1.50),
            Element::Fr => Some(2.60),
            Element::Ra => Some(2.21),
            Element::Th => Some(2.06),
            Element::Pa => Some(2.00),
            Element::Am => Some(1.80),
            Element::Cm => Some(1.69),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_convert_to_u8() {
        assert_eq!(Element::X as u8, 0);
        assert_eq!(Element::H as u8, 1);
        assert_eq!(Element::Ne as u8, 10);
        assert_eq!(Element::Og as u8, 118);
    }

    #[test]
    fn from_atomic_number_works() {
        assert_eq!(Element::from_atomic_number(0), None);
        assert_eq!(Element::from_atomic_number(1), Some(Element::H));
        assert_eq!(Element::from_atomic_number(10), Some(Element::Ne));
        assert_eq!(Element::from_atomic_number(118), Some(Element::Og));
        assert_eq!(Element::from_atomic_number(119), None);
    }

    #[test]
    fn should_parse_from_string() {
        assert_eq!("Fe".parse::<Element>(), Ok(Element::Fe));
        assert_eq!("fe".parse::<Element>(), Ok(Element::Fe));
    }

    #[test]
    fn covalent_alvarez2008_works() {
        approx::assert_relative_eq!(Element::H.get_radius().unwrap(), 0.31);
        approx::assert_relative_eq!(Element::C.get_radius().unwrap(), 0.744_533_7);
        approx::assert_relative_eq!(Element::Mn.get_radius().unwrap(), 1.553_504);
        approx::assert_relative_eq!(Element::Fe.get_radius().unwrap(), 1.484_179_1);
        approx::assert_relative_eq!(Element::Co.get_radius().unwrap(), 1.288_742_5);
        approx::assert_relative_eq!(Element::Cm.get_radius().unwrap(), 1.69);
    }
}
