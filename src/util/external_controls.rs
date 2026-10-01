use gettextrs::gettext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalControlsMode {
    ForwardBack = 0,
    NextPrevious = 1,
}

impl ExternalControlsMode {
    pub const ALL: [ExternalControlsMode; 2] = [
        ExternalControlsMode::ForwardBack,
        ExternalControlsMode::NextPrevious,
    ];

    pub fn index(&self) -> u32 {
        *self as u32
    }

    pub fn from_index(idx: u32) -> Self {
        match idx {
            1 => ExternalControlsMode::NextPrevious,
            _ => ExternalControlsMode::ForwardBack,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            ExternalControlsMode::ForwardBack => gettext("Forward/Back"),
            ExternalControlsMode::NextPrevious => gettext("Next/Previous"),
        }
    }
}
