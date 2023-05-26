use ecolor::Color32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(into = "String")]
#[serde(from = "String")]
pub struct Colour {
    pub rgb: [u8; 3],
}

impl Colour {
    pub fn as_u8(&mut self) -> &mut [u8; 3] {
        &mut self.rgb
    }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { rgb: [r, g, b] }
    }
}

impl From<String> for Colour {
    fn from(value: String) -> Self {
        let values: Vec<u8> = value
            .split_ascii_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        match values[0..3].try_into() {
            Ok(rgb) => Self { rgb },
            Err(e) => panic!(
                "invalid colour value {} (must have 3 elements): {}",
                value, e
            ),
        }
    }
}

impl Into<String> for Colour {
    fn into(self) -> String {
        format!("{} {} {}", self.rgb[0], self.rgb[1], self.rgb[2])
    }
}

impl From<Colour> for Color32 {
    fn from(val: Colour) -> Self {
        Color32::from_rgb(val.rgb[0], val.rgb[1], val.rgb[2])
    }
}

impl Colour {
    pub fn default_moderator_colour() -> Self {
        Self::from_rgb(255, 78, 78)
    }
}
