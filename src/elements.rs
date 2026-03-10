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
    /// This is the same as [`Self::from_repr`], except that this function
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
    #[allow(clippy::too_many_lines)]
    pub fn get_radius(self) -> Option<f32> {
        match self {
            Self::H => Some(0.31),
            Self::He => Some(0.28),
            Self::Li => Some(1.28),
            Self::Be => Some(0.96),
            Self::B => Some(0.84),
            Self::C => {
                let (sp3, n_sp3) = (0.76, 10000_f64);
                let (sp2, n_sp2) = (0.73, 10000_f64);
                let (sp, n_sp) = (0.69, 171_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(sp3, n_sp3), (sp2, n_sp2), (sp, n_sp)])
                        .mean() as f32,
                )
            }
            Self::N => Some(0.71),
            Self::O => Some(0.66),
            Self::F => Some(0.57),
            Self::Ne => Some(0.58),
            Self::Na => Some(1.66),
            Self::Mg | Self::Ir => Some(1.41),
            Self::Al => Some(1.21),
            Self::Si => Some(1.11),
            Self::P => Some(1.07),
            Self::S => Some(1.05),
            Self::Cl => Some(1.02),
            Self::Ar => Some(1.06),
            Self::K | Self::Pr => Some(2.03),
            Self::Ca => Some(1.76),
            Self::Sc | Self::Ta => Some(1.70),
            Self::Ti => Some(1.60),
            Self::V => Some(1.53),
            Self::Cr | Self::Pd | Self::Sn | Self::Sb | Self::I => Some(1.39),
            Self::Mn => {
                let (lowspin, n_lowspin) = (1.39, 321_f64);
                let (highspin, n_highspin) = (1.61, 929_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Self::Fe => {
                let (lowspin, n_lowspin) = (1.32, 336_f64);
                let (highspin, n_highspin) = (1.52, 1540_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Self::Co => {
                let (lowspin, n_lowspin) = (1.26, 5733_f64);
                let (highspin, n_highspin) = (1.50, 780_f64);
                #[allow(clippy::cast_possible_truncation)]
                Some(
                    average::WeightedMean::from_iter([(lowspin, n_lowspin), (highspin, n_highspin)])
                        .mean() as f32,
                )
            }
            Self::Ni => Some(1.24),
            Self::Cu | Self::Hg => Some(1.32),
            Self::Zn | Self::Ga => Some(1.22),
            Self::Ge | Self::Se | Self::Br => Some(1.20),
            Self::As => Some(1.19),
            Self::Kr => Some(1.16),
            Self::Rb => Some(2.20),
            Self::Sr => Some(1.95),
            Self::Y | Self::Tm | Self::Np => Some(1.90),
            Self::Zr | Self::Hf => Some(1.75),
            Self::Nb => Some(1.64),
            Self::Mo => Some(1.54),
            Self::Tc => Some(1.47),
            Self::Ru | Self::Pb => Some(1.46),
            Self::Rh | Self::In => Some(1.42),
            Self::Ag | Self::Tl => Some(1.45),
            Self::Cd | Self::Os => Some(1.44),
            Self::Te => Some(1.38),
            Self::Xe | Self::Po => Some(1.40),
            Self::Cs => Some(2.44),
            Self::Ba | Self::Ac => Some(2.15),
            Self::La => Some(2.07),
            Self::Ce => Some(2.04),
            Self::Nd => Some(2.01),
            Self::Pm => Some(1.99),
            Self::Sm | Self::Eu => Some(1.98),
            Self::Gd | Self::U => Some(1.96),
            Self::Tb => Some(1.94),
            Self::Dy | Self::Ho => Some(1.92),
            Self::Er => Some(1.89),
            Self::Yb | Self::Lu | Self::Pu => Some(1.87),
            Self::W => Some(1.62),
            Self::Re => Some(1.51),
            Self::Pt | Self::Au => Some(1.36),
            Self::Bi => Some(1.48),
            Self::At | Self::Rn => Some(1.50),
            Self::Fr => Some(2.60),
            Self::Ra => Some(2.21),
            Self::Th => Some(2.06),
            Self::Pa => Some(2.00),
            Self::Am => Some(1.80),
            Self::Cm => Some(1.69),
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
