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

/// Units lookup table (subset from B24 documentation Appendix B)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DataUnits {
    #[default]
    MvPerV,
    Kg,
    Grams,
    Lbs,
    Oz,
    Newtons,
    KiloNewtons,
    Lbf,
    Kgf,
    Bar,
    Psi,
    Pascal,
    Nm,
    Metres,
    Cm,
    Mm,
    Feet,
    Inches,
    Counts,
    Other(u8),
}

impl DataUnits {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Self::MvPerV,
            0x01 => Self::Kg,
            0x1F => Self::Grams,
            0x32 => Self::Lbs,
            0x31 => Self::Oz,
            0x41 => Self::Newtons,
            0x42 => Self::KiloNewtons,
            0x4E => Self::Lbf,
            0x4A => Self::Kgf,
            0x5F => Self::Bar,
            0x61 => Self::Psi,
            0x6B => Self::Pascal,
            0x78 => Self::Nm,
            0x0F => Self::Metres,
            0x12 => Self::Cm,
            0x23 => Self::Mm,
            0x19 => Self::Feet,
            0x1A => Self::Inches,
            0x96 => Self::Counts,
            other => Self::Other(other),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::MvPerV => 0x00,
            Self::Kg => 0x01,
            Self::Grams => 0x1F,
            Self::Lbs => 0x32,
            Self::Oz => 0x31,
            Self::Newtons => 0x41,
            Self::KiloNewtons => 0x42,
            Self::Lbf => 0x4E,
            Self::Kgf => 0x4A,
            Self::Bar => 0x5F,
            Self::Psi => 0x61,
            Self::Pascal => 0x6B,
            Self::Nm => 0x78,
            Self::Metres => 0x0F,
            Self::Cm => 0x12,
            Self::Mm => 0x23,
            Self::Feet => 0x19,
            Self::Inches => 0x1A,
            Self::Counts => 0x96,
            Self::Other(b) => b,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MvPerV => "mV/V",
            Self::Kg => "kg",
            Self::Grams => "g",
            Self::Lbs => "lbs",
            Self::Oz => "oz",
            Self::Newtons => "N",
            Self::KiloNewtons => "kN",
            Self::Lbf => "lbf",
            Self::Kgf => "kgf",
            Self::Bar => "bar",
            Self::Psi => "psi",
            Self::Pascal => "Pa",
            Self::Nm => "N m",
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
        Self::Nm,
        Self::Bar,
        Self::Psi,
        Self::Counts,
    ];
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

/// Advanced parameter indices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedParam {
    PeakValue,
    TroughValue,
    DisplayMin,
    DisplayMax,
    FilterLevel,
    FilterSteps,
    LinAutoIncrement,
    LinDirection,
    DigitalOutputFunction,
    FastMode,
    FastDataRate,
    FastDuration,
    FastLevel,
}

impl AdvancedParam {
    pub fn index(self) -> u8 {
        match self {
            Self::PeakValue => 5,
            Self::TroughValue => 6,
            Self::DisplayMin => 26,
            Self::DisplayMax => 27,
            Self::FilterLevel => 35,
            Self::FilterSteps => 36,
            Self::LinAutoIncrement => 37,
            Self::LinDirection => 38,
            Self::DigitalOutputFunction => 39,
            Self::FastMode => 42,
            Self::FastDataRate => 43,
            Self::FastDuration => 44,
            Self::FastLevel => 45,
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
            Self::LinAutoIncrement => "Lin Auto-Increment",
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
        Self::LinAutoIncrement,
        Self::LinDirection,
        Self::DigitalOutputFunction,
        Self::FastMode,
        Self::FastDataRate,
        Self::FastDuration,
        Self::FastLevel,
    ];
}

/// Device actions triggered via advanced index/data writes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    Reboot,
    ShuntCalOn,
    ShuntCalOff,
    Tare,
    ResetTare,
    ResetPeakTrough,
    RestoreEepromDefaults,
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
    pub fn command(self) -> (u8, Vec<u8>) {
        match self {
            Self::Reboot => (189, vec![1]),
            Self::ShuntCalOn => (190, vec![1]),
            Self::ShuntCalOff => (190, vec![0]),
            Self::Tare => (191, vec![1]),
            Self::ResetTare => (191, vec![0]),
            Self::ResetPeakTrough => (192, vec![1]),
            Self::RestoreEepromDefaults => (197, vec![1]),
        }
    }
}
