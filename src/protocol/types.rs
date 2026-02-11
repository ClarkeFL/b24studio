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
    #[default]
    MvPerV,       // 0x00
    Kg,           // 0x2D
    Grams,        // 0x30
    Lbs,          // 0x34
    Oz,           // 0x32
    Newtons,      // 0x41
    KiloNewtons,  // 0x42
    MilliNewtons, // 0x43
    Lbf,          // 0x4D
    Kgf,          // 0x49
    Bar,          // 0x5F
    Psi,          // 0x6E
    Pascal,       // 0x6A
    NewtonMetre,  // 0x96
    Metres,       // 0x0F
    Cm,           // 0x12
    Mm,           // 0x22
    Feet,         // 0x17
    Inches,       // 0x19
    Counts,       // 0xC8
    Other(u8),
}

impl DataUnits {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::MvPerV,
            0x2D => Self::Kg,
            0x30 => Self::Grams,
            0x34 => Self::Lbs,
            0x32 => Self::Oz,
            0x41 => Self::Newtons,
            0x42 => Self::KiloNewtons,
            0x43 => Self::MilliNewtons,
            0x4D => Self::Lbf,
            0x49 => Self::Kgf,
            0x5F => Self::Bar,
            0x6E => Self::Psi,
            0x6A => Self::Pascal,
            0x96 => Self::NewtonMetre,
            0x0F => Self::Metres,
            0x12 => Self::Cm,
            0x22 => Self::Mm,
            0x17 => Self::Feet,
            0x19 => Self::Inches,
            0xC8 => Self::Counts,
            other => Self::Other(other),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::MvPerV => 0x00,
            Self::Kg => 0x2D,
            Self::Grams => 0x30,
            Self::Lbs => 0x34,
            Self::Oz => 0x32,
            Self::Newtons => 0x41,
            Self::KiloNewtons => 0x42,
            Self::MilliNewtons => 0x43,
            Self::Lbf => 0x4D,
            Self::Kgf => 0x49,
            Self::Bar => 0x5F,
            Self::Psi => 0x6E,
            Self::Pascal => 0x6A,
            Self::NewtonMetre => 0x96,
            Self::Metres => 0x0F,
            Self::Cm => 0x12,
            Self::Mm => 0x22,
            Self::Feet => 0x17,
            Self::Inches => 0x19,
            Self::Counts => 0xC8,
            Self::Other(b) => b,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MvPerV => "mV/V",
            Self::Kg => "kg",
            Self::Grams => "g",
            Self::Lbs => "lb",
            Self::Oz => "oz",
            Self::Newtons => "N",
            Self::KiloNewtons => "kN",
            Self::MilliNewtons => "mN",
            Self::Lbf => "lbf",
            Self::Kgf => "kgf",
            Self::Bar => "bar",
            Self::Psi => "psi",
            Self::Pascal => "Pa",
            Self::NewtonMetre => "N m",
            Self::Metres => "m",
            Self::Cm => "cm",
            Self::Mm => "mm",
            Self::Feet => "ft",
            Self::Inches => "in",
            Self::Counts => "counts",
            Self::Other(_) => "?",
        }
    }

    pub const COMMON: &'static [DataUnits] = &[
        Self::MvPerV,
        Self::Kg,
        Self::Grams,
        Self::Lbs,
        Self::Newtons,
        Self::KiloNewtons,
        Self::NewtonMetre,
        Self::Bar,
        Self::Psi,
        Self::Counts,
    ];

    /// All known units for dropdown selection
    pub const ALL: &'static [DataUnits] = &[
        Self::MvPerV,
        Self::Kg,
        Self::Grams,
        Self::Lbs,
        Self::Oz,
        Self::Newtons,
        Self::KiloNewtons,
        Self::MilliNewtons,
        Self::Lbf,
        Self::Kgf,
        Self::Bar,
        Self::Psi,
        Self::Pascal,
        Self::NewtonMetre,
        Self::Metres,
        Self::Cm,
        Self::Mm,
        Self::Feet,
        Self::Inches,
        Self::Counts,
    ];

    /// Display label including the byte value for clarity
    pub fn dropdown_label(self) -> String {
        match self {
            Self::Other(b) => format!("Unknown (0x{b:02X})"),
            _ => format!("{} (0x{:02X})", self.label(), self.to_byte()),
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
    /// Actions are write-only; data value doesn't matter but we send 1
    pub fn command(self) -> (u8, Vec<u8>) {
        match self {
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
