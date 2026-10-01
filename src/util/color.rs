#[derive(Debug, Clone, Copy)]
pub struct RGBAColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl RGBAColor {
    pub fn to_css_rgba(&self) -> String {
        format!(
            "rgba({}, {}, {}, {})",
            (self.red * 255.0).round() as u8,
            (self.green * 255.0).round() as u8,
            (self.blue * 255.0).round() as u8,
            self.alpha
        )
    }
}