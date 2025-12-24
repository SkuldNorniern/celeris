//! CSS value types and parsing

/// CSS value types
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    Color(Color),
    Multiple(Vec<Value>), // For properties like margin: 10px 20px 30px 40px
    Function(String, Vec<Value>), // For functions like calc(), var(), url(), etc.
    Variable(String), // For CSS custom properties (--variable-name)
    String(String), // For string values
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.starts_with('#') {
            let hex = &hex[1..];
            match hex.len() {
                3 => {
                    // Short hex: #RGB
                    let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                    let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                    let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                    Some(Self::new(r, g, b, 255))
                }
                6 => {
                    // Full hex: #RRGGBB
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    Some(Self::new(r, g, b, 255))
                }
                8 => {
                    // Full hex with alpha: #RRGGBBAA
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                    Some(Self::new(r, g, b, a))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn from_named(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "black" => Some(Self::new(0, 0, 0, 255)),
            "white" => Some(Self::new(255, 255, 255, 255)),
            "red" => Some(Self::new(255, 0, 0, 255)),
            "green" => Some(Self::new(0, 255, 0, 255)),
            "blue" => Some(Self::new(0, 0, 255, 255)),
            "yellow" => Some(Self::new(255, 255, 0, 255)),
            "cyan" => Some(Self::new(0, 255, 255, 255)),
            "magenta" => Some(Self::new(255, 0, 255, 255)),
            "gray" | "grey" => Some(Self::new(128, 128, 128, 255)),
            "transparent" => Some(Self::new(0, 0, 0, 0)),
            _ => None,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new(0, 0, 0, 255)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Unit {
    Px,
    Em,
    Rem,
    Percent,
    Vh,
    Vw,
    Vmin,
    Vmax,
}

impl Unit {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "px" => Some(Unit::Px),
            "em" => Some(Unit::Em),
            "rem" => Some(Unit::Rem),
            "%" => Some(Unit::Percent),
            "vh" => Some(Unit::Vh),
            "vw" => Some(Unit::Vw),
            "vmin" => Some(Unit::Vmin),
            "vmax" => Some(Unit::Vmax),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Unit::Px => "px".to_string(),
            Unit::Em => "em".to_string(),
            Unit::Rem => "rem".to_string(),
            Unit::Percent => "%".to_string(),
            Unit::Vh => "vh".to_string(),
            Unit::Vw => "vw".to_string(),
            Unit::Vmin => "vmin".to_string(),
            Unit::Vmax => "vmax".to_string(),
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Unit::Px
    }
}
