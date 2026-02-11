use serde::{Deserialize, Serialize};

/// Resolution setting for the B24 ADC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    Bits8,
    Bits16,
    Bits32,
    Bits48,
    Bits64,
}

impl Resolution {
    pub fn to_byte(self) -> u8 {
        match self {
            Self::Bits8 => 8,
            Self::Bits16 => 16,
            Self::Bits32 => 32,
            Self::Bits48 => 48,
            Self::Bits64 => 64,
        }
    }

    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            8 => Some(Self::Bits8),
            16 => Some(Self::Bits16),
            32 => Some(Self::Bits32),
            48 => Some(Self::Bits48),
            64 => Some(Self::Bits64),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Bits8 => "8 (14.25 bits, max battery)",
            Self::Bits16 => "16 (15.25 bits, 75% battery)",
            Self::Bits32 => "32 (16 bits, 50% battery)",
            Self::Bits48 => "48 (16.5 bits, 37% battery)",
            Self::Bits64 => "64 (16.75 bits, 30% battery)",
        }
    }

    pub const ALL: [Resolution; 5] = [
        Self::Bits8,
        Self::Bits16,
        Self::Bits32,
        Self::Bits48,
        Self::Bits64,
    ];
}

/// Sensitivity range selector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitivityRange {
    Range6mV,   // 0 = ±6 mV/V
    Range12mV,  // 1 = ±12 mV/V
    Range24mV,  // 2 = ±24 mV/V
    Range48mV,  // 3 = ±48 mV/V
}

impl SensitivityRange {
    pub fn to_byte(self) -> u8 {
        match self {
            Self::Range6mV => 0,
            Self::Range12mV => 1,
            Self::Range24mV => 2,
            Self::Range48mV => 3,
        }
    }

    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Range6mV),
            1 => Some(Self::Range12mV),
            2 => Some(Self::Range24mV),
            3 => Some(Self::Range48mV),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Range6mV => "0: ±6 mV/V",
            Self::Range12mV => "1: ±12 mV/V",
            Self::Range24mV => "2: ±24 mV/V",
            Self::Range48mV => "3: ±48 mV/V",
        }
    }

    pub const ALL: [SensitivityRange; 4] = [
        Self::Range6mV,
        Self::Range12mV,
        Self::Range24mV,
        Self::Range48mV,
    ];
}

/// Decoded status byte from the Data Service
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StatusByte {
    pub shunt_cal: bool,
    pub integrity: bool,
    pub not_gross: bool,
    pub over_range: bool,
    pub fast_mode: bool,
    pub battery_low: bool,
    pub digital_input: bool,
}

impl StatusByte {
    pub fn from_byte(b: u8) -> Self {
        Self {
            shunt_cal: b & 0x01 != 0,
            integrity: b & 0x02 != 0,
            not_gross: b & 0x04 != 0,
            over_range: b & 0x08 != 0,
            fast_mode: b & 0x10 != 0,
            battery_low: b & 0x20 != 0,
            digital_input: b & 0x40 != 0,
        }
    }

    pub fn to_byte(self) -> u8 {
        let mut b = 0u8;
        if self.shunt_cal { b |= 0x01; }
        if self.integrity { b |= 0x02; }
        if self.not_gross { b |= 0x04; }
        if self.over_range { b |= 0x08; }
        if self.fast_mode { b |= 0x10; }
        if self.battery_low { b |= 0x20; }
        if self.digital_input { b |= 0x40; }
        b
    }

    pub fn description(self) -> String {
        let mut parts = Vec::new();
        if self.shunt_cal { parts.push("ShuntCal"); }
        if self.integrity { parts.push("Integrity"); }
        if self.not_gross { parts.push("Tare"); }
        if self.over_range { parts.push("OverRange"); }
        if self.fast_mode { parts.push("Fast"); }
        if self.battery_low { parts.push("BattLow"); }
        if self.digital_input { parts.push("DigIn"); }
        if parts.is_empty() {
            "OK".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Units lookup table from B24 Technical Manual Appendix B
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DataUnits {
    // Ratio (0)
    #[default]
    MvPerV,           // 0
    // Angle (1–7)
    Radians,          // 1
    Degrees,          // 2
    Circumference,    // 3
    Grade,            // 4
    ArcMinutes,       // 5
    ArcSeconds,       // 6
    Revolutions,      // 7
    // Length (15–38)
    Metres,           // 15
    Angstrom,         // 16
    AstronomicalUnit, // 17
    Cm,               // 18
    ChainsGunters,    // 19
    Ell,              // 20
    Em,               // 21
    Fathoms,          // 22
    Feet,             // 23
    Furlongs,         // 24
    Inches,           // 25
    Km,               // 26
    League,           // 27
    Leagues,          // 28
    LightYears,       // 29
    Lines,            // 30
    Microns,          // 31
    NauticalMiles,    // 32
    Miles,            // 33
    Mm,               // 34
    Mils,             // 35
    Nanometers,       // 36
    Parsec,           // 37
    Yards,            // 38
    // Mass (45–59)
    Kg,               // 45
    Drams,            // 46
    Grains,           // 47
    Grams,            // 48
    Milligrams,       // 49
    Oz,               // 50
    Pennyweights,     // 51
    Lbs,              // 52
    Kilopounds,       // 53
    Scruples,         // 54
    Slug,             // 55
    TonsLong,         // 56
    TonsMetric,       // 57
    Tonnes,           // 58
    TonsShort,        // 59
    // Force (65–81)
    Newtons,          // 65
    KiloNewtons,      // 66
    MilliNewtons,     // 67
    MegaNewtons,      // 68
    Crinals,          // 69
    Dynes,            // 70
    GramsForce,       // 71
    JoulesPerCm,      // 72
    Kgf,              // 73
    KgfKp,            // 74
    KgMsSquared,      // 75
    OuncesForce,      // 76
    Lbf,              // 77
    Poundals,         // 78
    TonsForceLong,    // 79
    TonsForceShort,   // 80
    TonsForceMetric,  // 81
    // Pressure (95–111)
    Bar,              // 95
    AtmosphereTech,   // 96
    AtmospherePhys,   // 97
    DynePerCmSq,      // 98
    FtWater,          // 99
    InWater,          // 100
    GigaPascal,       // 101
    HectoPascal,      // 102
    KgfPerCmSq,       // 103
    KgfPerMSq,        // 104
    Microbar,         // 105
    Pascal,           // 106
    NewtonPerMSq,     // 107
    OzPerInSq,        // 108
    LbPerFtSq,        // 109
    Psi,              // 110
    TonnePerCmSq,     // 111
    // Speed (120–135)
    MetresPerSec,     // 120
    CmPerSec,         // 121
    FeetPerMin,       // 122
    FeetPerSec,       // 123
    KmPerHr,          // 124
    KmPerMin,         // 125
    KmPerSec,         // 126
    Knots,            // 127
    MetresPerHr,      // 128
    MetresPerMin,     // 129
    MilesPerHr,       // 130
    MilesPerMin,      // 131
    MilesPerSec,      // 132
    NautMilesPerHr,   // 133
    NautMilesPerMin,  // 134
    NautMilesPerSec,  // 135
    // Torque (150–154)
    NewtonMetre,      // 150
    MetreKg,          // 151
    FootPound,        // 152
    FootPoundal,      // 153
    InchPound,        // 154
    // Arbitrary (200)
    Counts,           // 200
    // Undefined (255)
    Undefined,        // 255
    // Unknown
    Other(u8),
}

impl DataUnits {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::MvPerV,
            1 => Self::Radians,
            2 => Self::Degrees,
            3 => Self::Circumference,
            4 => Self::Grade,
            5 => Self::ArcMinutes,
            6 => Self::ArcSeconds,
            7 => Self::Revolutions,
            15 => Self::Metres,
            16 => Self::Angstrom,
            17 => Self::AstronomicalUnit,
            18 => Self::Cm,
            19 => Self::ChainsGunters,
            20 => Self::Ell,
            21 => Self::Em,
            22 => Self::Fathoms,
            23 => Self::Feet,
            24 => Self::Furlongs,
            25 => Self::Inches,
            26 => Self::Km,
            27 => Self::League,
            28 => Self::Leagues,
            29 => Self::LightYears,
            30 => Self::Lines,
            31 => Self::Microns,
            32 => Self::NauticalMiles,
            33 => Self::Miles,
            34 => Self::Mm,
            35 => Self::Mils,
            36 => Self::Nanometers,
            37 => Self::Parsec,
            38 => Self::Yards,
            45 => Self::Kg,
            46 => Self::Drams,
            47 => Self::Grains,
            48 => Self::Grams,
            49 => Self::Milligrams,
            50 => Self::Oz,
            51 => Self::Pennyweights,
            52 => Self::Lbs,
            53 => Self::Kilopounds,
            54 => Self::Scruples,
            55 => Self::Slug,
            56 => Self::TonsLong,
            57 => Self::TonsMetric,
            58 => Self::Tonnes,
            59 => Self::TonsShort,
            65 => Self::Newtons,
            66 => Self::KiloNewtons,
            67 => Self::MilliNewtons,
            68 => Self::MegaNewtons,
            69 => Self::Crinals,
            70 => Self::Dynes,
            71 => Self::GramsForce,
            72 => Self::JoulesPerCm,
            73 => Self::Kgf,
            74 => Self::KgfKp,
            75 => Self::KgMsSquared,
            76 => Self::OuncesForce,
            77 => Self::Lbf,
            78 => Self::Poundals,
            79 => Self::TonsForceLong,
            80 => Self::TonsForceShort,
            81 => Self::TonsForceMetric,
            95 => Self::Bar,
            96 => Self::AtmosphereTech,
            97 => Self::AtmospherePhys,
            98 => Self::DynePerCmSq,
            99 => Self::FtWater,
            100 => Self::InWater,
            101 => Self::GigaPascal,
            102 => Self::HectoPascal,
            103 => Self::KgfPerCmSq,
            104 => Self::KgfPerMSq,
            105 => Self::Microbar,
            106 => Self::Pascal,
            107 => Self::NewtonPerMSq,
            108 => Self::OzPerInSq,
            109 => Self::LbPerFtSq,
            110 => Self::Psi,
            111 => Self::TonnePerCmSq,
            120 => Self::MetresPerSec,
            121 => Self::CmPerSec,
            122 => Self::FeetPerMin,
            123 => Self::FeetPerSec,
            124 => Self::KmPerHr,
            125 => Self::KmPerMin,
            126 => Self::KmPerSec,
            127 => Self::Knots,
            128 => Self::MetresPerHr,
            129 => Self::MetresPerMin,
            130 => Self::MilesPerHr,
            131 => Self::MilesPerMin,
            132 => Self::MilesPerSec,
            133 => Self::NautMilesPerHr,
            134 => Self::NautMilesPerMin,
            135 => Self::NautMilesPerSec,
            150 => Self::NewtonMetre,
            151 => Self::MetreKg,
            152 => Self::FootPound,
            153 => Self::FootPoundal,
            154 => Self::InchPound,
            200 => Self::Counts,
            255 => Self::Undefined,
            other => Self::Other(other),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::MvPerV => 0,
            Self::Radians => 1,
            Self::Degrees => 2,
            Self::Circumference => 3,
            Self::Grade => 4,
            Self::ArcMinutes => 5,
            Self::ArcSeconds => 6,
            Self::Revolutions => 7,
            Self::Metres => 15,
            Self::Angstrom => 16,
            Self::AstronomicalUnit => 17,
            Self::Cm => 18,
            Self::ChainsGunters => 19,
            Self::Ell => 20,
            Self::Em => 21,
            Self::Fathoms => 22,
            Self::Feet => 23,
            Self::Furlongs => 24,
            Self::Inches => 25,
            Self::Km => 26,
            Self::League => 27,
            Self::Leagues => 28,
            Self::LightYears => 29,
            Self::Lines => 30,
            Self::Microns => 31,
            Self::NauticalMiles => 32,
            Self::Miles => 33,
            Self::Mm => 34,
            Self::Mils => 35,
            Self::Nanometers => 36,
            Self::Parsec => 37,
            Self::Yards => 38,
            Self::Kg => 45,
            Self::Drams => 46,
            Self::Grains => 47,
            Self::Grams => 48,
            Self::Milligrams => 49,
            Self::Oz => 50,
            Self::Pennyweights => 51,
            Self::Lbs => 52,
            Self::Kilopounds => 53,
            Self::Scruples => 54,
            Self::Slug => 55,
            Self::TonsLong => 56,
            Self::TonsMetric => 57,
            Self::Tonnes => 58,
            Self::TonsShort => 59,
            Self::Newtons => 65,
            Self::KiloNewtons => 66,
            Self::MilliNewtons => 67,
            Self::MegaNewtons => 68,
            Self::Crinals => 69,
            Self::Dynes => 70,
            Self::GramsForce => 71,
            Self::JoulesPerCm => 72,
            Self::Kgf => 73,
            Self::KgfKp => 74,
            Self::KgMsSquared => 75,
            Self::OuncesForce => 76,
            Self::Lbf => 77,
            Self::Poundals => 78,
            Self::TonsForceLong => 79,
            Self::TonsForceShort => 80,
            Self::TonsForceMetric => 81,
            Self::Bar => 95,
            Self::AtmosphereTech => 96,
            Self::AtmospherePhys => 97,
            Self::DynePerCmSq => 98,
            Self::FtWater => 99,
            Self::InWater => 100,
            Self::GigaPascal => 101,
            Self::HectoPascal => 102,
            Self::KgfPerCmSq => 103,
            Self::KgfPerMSq => 104,
            Self::Microbar => 105,
            Self::Pascal => 106,
            Self::NewtonPerMSq => 107,
            Self::OzPerInSq => 108,
            Self::LbPerFtSq => 109,
            Self::Psi => 110,
            Self::TonnePerCmSq => 111,
            Self::MetresPerSec => 120,
            Self::CmPerSec => 121,
            Self::FeetPerMin => 122,
            Self::FeetPerSec => 123,
            Self::KmPerHr => 124,
            Self::KmPerMin => 125,
            Self::KmPerSec => 126,
            Self::Knots => 127,
            Self::MetresPerHr => 128,
            Self::MetresPerMin => 129,
            Self::MilesPerHr => 130,
            Self::MilesPerMin => 131,
            Self::MilesPerSec => 132,
            Self::NautMilesPerHr => 133,
            Self::NautMilesPerMin => 134,
            Self::NautMilesPerSec => 135,
            Self::NewtonMetre => 150,
            Self::MetreKg => 151,
            Self::FootPound => 152,
            Self::FootPoundal => 153,
            Self::InchPound => 154,
            Self::Counts => 200,
            Self::Undefined => 255,
            Self::Other(b) => b,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MvPerV => "mV/V",
            Self::Radians => "rad",
            Self::Degrees => "\u{00B0}",
            Self::Circumference => "circ",
            Self::Grade => "grade",
            Self::ArcMinutes => "'",
            Self::ArcSeconds => "\"",
            Self::Revolutions => "rev",
            Self::Metres => "m",
            Self::Angstrom => "\u{00C5}",
            Self::AstronomicalUnit => "AU",
            Self::Cm => "cm",
            Self::ChainsGunters => "ch",
            Self::Ell => "ell",
            Self::Em => "em",
            Self::Fathoms => "fm",
            Self::Feet => "ft",
            Self::Furlongs => "fur",
            Self::Inches => "in",
            Self::Km => "km",
            Self::League => "lea",
            Self::Leagues => "league",
            Self::LightYears => "ly",
            Self::Lines => "ln",
            Self::Microns => "\u{00B5}",
            Self::NauticalMiles => "mi n",
            Self::Miles => "mi",
            Self::Mm => "mm",
            Self::Mils => "mil",
            Self::Nanometers => "nm",
            Self::Parsec => "pc",
            Self::Yards => "yd",
            Self::Kg => "kg",
            Self::Drams => "dr av",
            Self::Grains => "gr",
            Self::Grams => "g",
            Self::Milligrams => "mg",
            Self::Oz => "oz",
            Self::Pennyweights => "pwt",
            Self::Lbs => "lb",
            Self::Kilopounds => "klb",
            Self::Scruples => "s ap",
            Self::Slug => "slug",
            Self::TonsLong => "ton",
            Self::TonsMetric => "T",
            Self::Tonnes => "tonne",
            Self::TonsShort => "sh tn",
            Self::Newtons => "N",
            Self::KiloNewtons => "kN",
            Self::MilliNewtons => "mN",
            Self::MegaNewtons => "MN",
            Self::Crinals => "crinal",
            Self::Dynes => "dyn",
            Self::GramsForce => "gf",
            Self::JoulesPerCm => "J/cm",
            Self::Kgf => "kgf",
            Self::KgfKp => "kp",
            Self::KgMsSquared => "kg m/s\u{00B2}",
            Self::OuncesForce => "ozf",
            Self::Lbf => "lbf",
            Self::Poundals => "pdl",
            Self::TonsForceLong => "tonf l",
            Self::TonsForceShort => "tonf s",
            Self::TonsForceMetric => "tonf m",
            Self::Bar => "bar",
            Self::AtmosphereTech => "at",
            Self::AtmospherePhys => "atm",
            Self::DynePerCmSq => "dyn/cm\u{00B2}",
            Self::FtWater => "ftH\u{2082}O",
            Self::InWater => "inH\u{2082}O",
            Self::GigaPascal => "GPa",
            Self::HectoPascal => "hPa",
            Self::KgfPerCmSq => "kgf/cm\u{00B2}",
            Self::KgfPerMSq => "kgf/m\u{00B2}",
            Self::Microbar => "\u{00B5}bar",
            Self::Pascal => "Pa",
            Self::NewtonPerMSq => "N/m\u{00B2}",
            Self::OzPerInSq => "oz/in\u{00B2}",
            Self::LbPerFtSq => "lb/ft\u{00B2}",
            Self::Psi => "psi",
            Self::TonnePerCmSq => "T/cm\u{00B2}",
            Self::MetresPerSec => "m/s",
            Self::CmPerSec => "cm/s",
            Self::FeetPerMin => "ft/min",
            Self::FeetPerSec => "ft/s",
            Self::KmPerHr => "km/h",
            Self::KmPerMin => "km/min",
            Self::KmPerSec => "km/s",
            Self::Knots => "kn",
            Self::MetresPerHr => "m/h",
            Self::MetresPerMin => "m/min",
            Self::MilesPerHr => "mph",
            Self::MilesPerMin => "mi/min",
            Self::MilesPerSec => "mi/s",
            Self::NautMilesPerHr => "n mi/h",
            Self::NautMilesPerMin => "n mi/min",
            Self::NautMilesPerSec => "n mi/s",
            Self::NewtonMetre => "N m",
            Self::MetreKg => "m kg",
            Self::FootPound => "ft lbf",
            Self::FootPoundal => "ft pdl",
            Self::InchPound => "in lbf",
            Self::Counts => "counts",
            Self::Undefined => "undefined",
            Self::Other(_) => "?",
        }
    }

    /// All known units for dropdown selection, grouped by category
    pub const ALL: &'static [DataUnits] = &[
        // Ratio
        Self::MvPerV,
        // Angle
        Self::Radians, Self::Degrees, Self::Circumference, Self::Grade,
        Self::ArcMinutes, Self::ArcSeconds, Self::Revolutions,
        // Length
        Self::Metres, Self::Angstrom, Self::AstronomicalUnit, Self::Cm,
        Self::ChainsGunters, Self::Ell, Self::Em, Self::Fathoms, Self::Feet,
        Self::Furlongs, Self::Inches, Self::Km, Self::League, Self::Leagues,
        Self::LightYears, Self::Lines, Self::Microns, Self::NauticalMiles,
        Self::Miles, Self::Mm, Self::Mils, Self::Nanometers, Self::Parsec, Self::Yards,
        // Mass
        Self::Kg, Self::Drams, Self::Grains, Self::Grams, Self::Milligrams,
        Self::Oz, Self::Pennyweights, Self::Lbs, Self::Kilopounds, Self::Scruples,
        Self::Slug, Self::TonsLong, Self::TonsMetric, Self::Tonnes, Self::TonsShort,
        // Force
        Self::Newtons, Self::KiloNewtons, Self::MilliNewtons, Self::MegaNewtons,
        Self::Crinals, Self::Dynes, Self::GramsForce, Self::JoulesPerCm, Self::Kgf,
        Self::KgfKp, Self::KgMsSquared, Self::OuncesForce, Self::Lbf, Self::Poundals,
        Self::TonsForceLong, Self::TonsForceShort, Self::TonsForceMetric,
        // Pressure
        Self::Bar, Self::AtmosphereTech, Self::AtmospherePhys, Self::DynePerCmSq,
        Self::FtWater, Self::InWater, Self::GigaPascal, Self::HectoPascal,
        Self::KgfPerCmSq, Self::KgfPerMSq, Self::Microbar, Self::Pascal,
        Self::NewtonPerMSq, Self::OzPerInSq, Self::LbPerFtSq, Self::Psi, Self::TonnePerCmSq,
        // Speed
        Self::MetresPerSec, Self::CmPerSec, Self::FeetPerMin, Self::FeetPerSec,
        Self::KmPerHr, Self::KmPerMin, Self::KmPerSec, Self::Knots,
        Self::MetresPerHr, Self::MetresPerMin, Self::MilesPerHr, Self::MilesPerMin,
        Self::MilesPerSec, Self::NautMilesPerHr, Self::NautMilesPerMin, Self::NautMilesPerSec,
        // Torque
        Self::NewtonMetre, Self::MetreKg, Self::FootPound, Self::FootPoundal, Self::InchPound,
        // Arbitrary
        Self::Counts,
        // Undefined
        Self::Undefined,
    ];

    /// Display label with decimal byte value for dropdown
    pub fn dropdown_label(self) -> String {
        match self {
            Self::Other(b) => format!("Unknown ({b})"),
            _ => format!("{} ({})", self.label(), self.to_byte()),
        }
    }

    /// Full name as used by the B24 Toolkit mobile app (lowercase)
    pub fn app_name(self) -> &'static str {
        match self {
            // Ratio
            Self::MvPerV => "millivolt per volt",
            // Angle
            Self::Radians => "radian",
            Self::Degrees => "degree",
            Self::Circumference => "circumference",
            Self::Grade => "grade",
            Self::ArcMinutes => "arc minute",
            Self::ArcSeconds => "arc second",
            Self::Revolutions => "revolution",
            // Length
            Self::Metres => "metre",
            Self::Angstrom => "angstrom",
            Self::AstronomicalUnit => "astronomical unit",
            Self::Cm => "centimetre",
            Self::ChainsGunters => "chain",
            Self::Ell => "ell",
            Self::Em => "em",
            Self::Fathoms => "fathom",
            Self::Feet => "foot",
            Self::Furlongs => "furlong",
            Self::Inches => "inch",
            Self::Km => "kilometre",
            Self::League => "league",
            Self::Leagues => "league",
            Self::LightYears => "light year",
            Self::Lines => "line",
            Self::Microns => "micron",
            Self::NauticalMiles => "nautical mile",
            Self::Miles => "mile",
            Self::Mm => "millimetre",
            Self::Mils => "mil",
            Self::Nanometers => "nanometre",
            Self::Parsec => "parsec",
            Self::Yards => "yard",
            // Mass
            Self::Kg => "kilogram",
            Self::Drams => "dram",
            Self::Grains => "grain",
            Self::Grams => "gram",
            Self::Milligrams => "milligram",
            Self::Oz => "ounce",
            Self::Pennyweights => "pennyweight",
            Self::Lbs => "pound",
            Self::Kilopounds => "kilopound",
            Self::Scruples => "scruple",
            Self::Slug => "slug",
            Self::TonsLong => "ton long",
            Self::TonsMetric => "ton metric",
            Self::Tonnes => "tonne",
            Self::TonsShort => "ton short",
            // Force
            Self::Newtons => "newton",
            Self::KiloNewtons => "kilonewton",
            Self::MilliNewtons => "millinewton",
            Self::MegaNewtons => "meganewton",
            Self::Crinals => "crinal",
            Self::Dynes => "dyne",
            Self::GramsForce => "gram-force",
            Self::JoulesPerCm => "joule per centimetre",
            Self::Kgf => "kilogram-force",
            Self::KgfKp => "kilopond",
            Self::KgMsSquared => "kilogram metre per second squared",
            Self::OuncesForce => "ounce-force",
            Self::Lbf => "pound-force",
            Self::Poundals => "poundal",
            Self::TonsForceLong => "ton-force long",
            Self::TonsForceShort => "ton-force short",
            Self::TonsForceMetric => "ton-force metric",
            // Pressure
            Self::Bar => "bar",
            Self::AtmosphereTech => "atmosphere technical",
            Self::AtmospherePhys => "atmosphere",
            Self::DynePerCmSq => "dyne per square centimetre",
            Self::FtWater => "foot of water",
            Self::InWater => "inch of water",
            Self::GigaPascal => "gigapascal",
            Self::HectoPascal => "hectopascal",
            Self::KgfPerCmSq => "kilogram-force per square centimetre",
            Self::KgfPerMSq => "kilogram-force per square metre",
            Self::Microbar => "microbar",
            Self::Pascal => "pascal",
            Self::NewtonPerMSq => "newton per square metre",
            Self::OzPerInSq => "ounce per square inch",
            Self::LbPerFtSq => "pound per square foot",
            Self::Psi => "pound per square inch",
            Self::TonnePerCmSq => "tonne per square centimetre",
            // Speed
            Self::MetresPerSec => "metre per second",
            Self::CmPerSec => "centimetre per second",
            Self::FeetPerMin => "foot per minute",
            Self::FeetPerSec => "foot per second",
            Self::KmPerHr => "kilometre per hour",
            Self::KmPerMin => "kilometre per minute",
            Self::KmPerSec => "kilometre per second",
            Self::Knots => "knot",
            Self::MetresPerHr => "metre per hour",
            Self::MetresPerMin => "metre per minute",
            Self::MilesPerHr => "mile per hour",
            Self::MilesPerMin => "mile per minute",
            Self::MilesPerSec => "mile per second",
            Self::NautMilesPerHr => "nautical mile per hour",
            Self::NautMilesPerMin => "nautical mile per minute",
            Self::NautMilesPerSec => "nautical mile per second",
            // Torque
            Self::NewtonMetre => "newton meter",
            Self::MetreKg => "metre kilogram",
            Self::FootPound => "foot pound",
            Self::FootPoundal => "foot poundal",
            Self::InchPound => "inch pound",
            // Arbitrary
            Self::Counts => "counts",
            // Undefined / Unknown
            Self::Undefined => "undefined",
            Self::Other(_) => "unknown",
        }
    }

    /// Unit group category as used by the B24 Toolkit mobile app
    pub fn app_group(self) -> &'static str {
        match self {
            Self::MvPerV => "ratio",
            Self::Radians | Self::Degrees | Self::Circumference | Self::Grade
            | Self::ArcMinutes | Self::ArcSeconds | Self::Revolutions => "angle",
            Self::Metres | Self::Angstrom | Self::AstronomicalUnit | Self::Cm
            | Self::ChainsGunters | Self::Ell | Self::Em | Self::Fathoms | Self::Feet
            | Self::Furlongs | Self::Inches | Self::Km | Self::League | Self::Leagues
            | Self::LightYears | Self::Lines | Self::Microns | Self::NauticalMiles
            | Self::Miles | Self::Mm | Self::Mils | Self::Nanometers | Self::Parsec
            | Self::Yards => "length",
            Self::Kg | Self::Drams | Self::Grains | Self::Grams | Self::Milligrams
            | Self::Oz | Self::Pennyweights | Self::Lbs | Self::Kilopounds | Self::Scruples
            | Self::Slug | Self::TonsLong | Self::TonsMetric | Self::Tonnes
            | Self::TonsShort => "mass",
            Self::Newtons | Self::KiloNewtons | Self::MilliNewtons | Self::MegaNewtons
            | Self::Crinals | Self::Dynes | Self::GramsForce | Self::JoulesPerCm | Self::Kgf
            | Self::KgfKp | Self::KgMsSquared | Self::OuncesForce | Self::Lbf | Self::Poundals
            | Self::TonsForceLong | Self::TonsForceShort | Self::TonsForceMetric => "force",
            Self::Bar | Self::AtmosphereTech | Self::AtmospherePhys | Self::DynePerCmSq
            | Self::FtWater | Self::InWater | Self::GigaPascal | Self::HectoPascal
            | Self::KgfPerCmSq | Self::KgfPerMSq | Self::Microbar | Self::Pascal
            | Self::NewtonPerMSq | Self::OzPerInSq | Self::LbPerFtSq | Self::Psi
            | Self::TonnePerCmSq => "pressure",
            Self::MetresPerSec | Self::CmPerSec | Self::FeetPerMin | Self::FeetPerSec
            | Self::KmPerHr | Self::KmPerMin | Self::KmPerSec | Self::Knots
            | Self::MetresPerHr | Self::MetresPerMin | Self::MilesPerHr | Self::MilesPerMin
            | Self::MilesPerSec | Self::NautMilesPerHr | Self::NautMilesPerMin
            | Self::NautMilesPerSec => "speed",
            Self::NewtonMetre | Self::MetreKg | Self::FootPound | Self::FootPoundal
            | Self::InchPound => "torque",
            Self::Counts => "arbitrary",
            Self::Undefined | Self::Other(_) => "undefined",
        }
    }
}

/// A single row of the linearisation table
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinearisationEntry {
    pub index: u8,
    pub valid_from: f32,
    pub gain: f32,
    pub offset: f32,
    pub valid_to: f32,
}

/// Data type for advanced parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Float,
    Uint32,
    Uint8,
}

/// Advanced parameter indices — from Appendix C of B24 Technical Manual
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedParam {
    PeakValue,             // 5  - Read only, FLOAT
    TroughValue,           // 6  - Read only, FLOAT
    DisplayMin,            // 26 - R/W, FLOAT
    DisplayMax,            // 27 - R/W, FLOAT
    FilterLevel,           // 28 - R/W, FLOAT
    FilterSteps,           // 29 - R/W, UINT32
    LinDirection,          // 35 - R/W, UINT8
    DigitalOutputFunction, // 39 - R/W, UINT32
    FastMode,              // 40 - R/W, UINT8
    FastDataRate,          // 41 - R/W, UINT32
    FastDuration,          // 42 - R/W, UINT32
    FastLevel,             // 43 - R/W, FLOAT
}

impl AdvancedParam {
    pub fn index(self) -> u8 {
        match self {
            Self::PeakValue => 5,
            Self::TroughValue => 6,
            Self::DisplayMin => 26,
            Self::DisplayMax => 27,
            Self::FilterLevel => 28,
            Self::FilterSteps => 29,
            Self::LinDirection => 35,
            Self::DigitalOutputFunction => 39,
            Self::FastMode => 40,
            Self::FastDataRate => 41,
            Self::FastDuration => 42,
            Self::FastLevel => 43,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::PeakValue => "Peak Value",
            Self::TroughValue => "Trough Value",
            Self::DisplayMin => "Display Min",
            Self::DisplayMax => "Display Max",
            Self::FilterLevel => "Filter Level",
            Self::FilterSteps => "Filter Steps",
            Self::LinDirection => "Lin Direction",
            Self::DigitalOutputFunction => "Digital Output Func",
            Self::FastMode => "Fast Mode",
            Self::FastDataRate => "Fast Data Rate",
            Self::FastDuration => "Fast Duration",
            Self::FastLevel => "Fast Level",
        }
    }

    pub fn data_type(self) -> ParamType {
        match self {
            Self::PeakValue | Self::TroughValue |
            Self::DisplayMin | Self::DisplayMax |
            Self::FilterLevel | Self::FastLevel => ParamType::Float,
            Self::FilterSteps | Self::DigitalOutputFunction |
            Self::FastDataRate | Self::FastDuration => ParamType::Uint32,
            Self::LinDirection | Self::FastMode => ParamType::Uint8,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::PeakValue => "Highest recorded value since last reset. Read-only.",
            Self::TroughValue => "Lowest recorded value since last reset. Read-only.",
            Self::DisplayMin => "Lower limit of the display range in engineering units. Used for bar graph scaling.",
            Self::DisplayMax => "Upper limit of the display range in engineering units. Used for bar graph scaling.",
            Self::FilterLevel => "Digital filter cutoff in engineering units. Set to 0 to disable. Filters out readings that change faster than this value per sample.",
            Self::FilterSteps => "Number of filter averaging steps (1-255). Higher = smoother but slower response. Set to 1 for no filtering.",
            Self::LinDirection => "Linearisation direction. 0 = ascending (normal), 1 = descending. Must match the direction of your calibration points.",
            Self::DigitalOutputFunction => "Digital output mode. 0 = disabled, 1 = over Display Max, 2 = under Display Min, 3 = outside range, 4 = within range.",
            Self::FastMode => "Fast transmission mode. 0 = disabled, 1 = timed burst, 2 = level triggered, 3 = continuous fast.",
            Self::FastDataRate => "Data rate in ms during fast mode (e.g. 100 = 10 readings/sec). Minimum 50ms.",
            Self::FastDuration => "Duration of fast mode burst in ms (e.g. 5000 = 5 seconds). Only used with timed burst mode.",
            Self::FastLevel => "Threshold in engineering units that triggers fast mode when exceeded. Only used with level-triggered mode.",
        }
    }

    pub const ALL: &'static [AdvancedParam] = &[
        Self::PeakValue,
        Self::TroughValue,
        Self::DisplayMin,
        Self::DisplayMax,
        Self::FilterLevel,
        Self::FilterSteps,
        Self::LinDirection,
        Self::DigitalOutputFunction,
        Self::FastMode,
        Self::FastDataRate,
        Self::FastDuration,
        Self::FastLevel,
    ];
}

/// Device actions triggered via advanced index/data writes — from Appendix C
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    CalculateCoefficients, // Index 38 — recalculate live coefficients after calibration
    Reboot,              // Index 189
    ShuntCalOn,          // Index 192
    ShuntCalOff,         // Index 193
    Tare,                // Index 194
    ResetTare,           // Index 195
    ResetPeakTrough,     // Index 196
    RestoreEepromDefaults, // Index 197
}

impl DeviceAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::CalculateCoefficients => "Calculate Coefficients",
            Self::Reboot => "Reboot Device",
            Self::ShuntCalOn => "Shunt Calibration On",
            Self::ShuntCalOff => "Shunt Calibration Off",
            Self::Tare => "Tare",
            Self::ResetTare => "Reset Tare",
            Self::ResetPeakTrough => "Reset Peak & Trough",
            Self::RestoreEepromDefaults => "Restore EEPROM Defaults",
        }
    }

    /// Returns (advanced_index, data_to_write)
    /// Actions are write-only; data value doesn't matter but we send 0
    pub fn command(self) -> (u8, Vec<u8>) {
        match self {
            Self::CalculateCoefficients => (38, vec![0]),
            Self::Reboot => (189, vec![0]),
            Self::ShuntCalOn => (192, vec![0]),
            Self::ShuntCalOff => (193, vec![0]),
            Self::Tare => (194, vec![0]),
            Self::ResetTare => (195, vec![0]),
            Self::ResetPeakTrough => (196, vec![0]),
            Self::RestoreEepromDefaults => (197, vec![0]),
        }
    }
}
