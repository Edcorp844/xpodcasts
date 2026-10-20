use gtk::prelude::{WidgetExt, WidgetExtManual};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeClass {
    Compact,
    Regular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeClasses {
    pub horizontal: SizeClass,
    pub vertical: SizeClass,
}

#[derive(Debug, Clone, Copy)]
pub struct SizeThresholds {
    pub regular_width: i32,
    pub regular_height: i32,
}

impl Default for SizeThresholds {
    fn default() -> Self {
        Self {
            regular_width: 760,
            regular_height: 600,
        }
    }
}

impl SizeThresholds {
    pub fn classify(&self, width: i32, height: i32) -> SizeClasses {
        SizeClasses {
            horizontal: if width >= self.regular_width {
                SizeClass::Regular
            } else {
                SizeClass::Compact
            },
            vertical: if height >= self.regular_height {
                SizeClass::Regular
            } else {
                SizeClass::Compact
            },
        }
    }
}

pub enum LayoutRule {
    CollapseSidebar { when: SizeClass },
    HideInspector { when: SizeClass },
}

pub trait HorizontalSizeClass {
    fn horizontal_size_class(&self) -> SizeClass;
}

impl HorizontalSizeClass for gtk::Window {
    fn horizontal_size_class(&self) -> SizeClass {
        // let window = self.clone();
        // let last_width = std::cell::Cell::new(0);
        // self.add_tick_callback(move |window, _| {
        //     let width = window.width();
        //     if width != last_width.get() {
        //         last_width.set(width);
        //     }
        //     gtk::glib::ControlFlow::Continue
        // });
        match self.width() {
            0..=800 => return SizeClass::Compact,
            _ => return SizeClass::Regular,
        }
        // self.add_tick_callback(move |self, _| {
        //     let width = self.width();
        //     if width != last_width.get() {
        //         last_width.set(width);
        //         let columns = match width {
        //             0..=500 => 2,
        //             501..=800 => 3,
        //             801..=1100 => 4,
        //             _ => 5,
        //         };
        //         self.set_min_children_per_line(columns);
        //         self.set_max_children_per_line(columns);
        //     }
        //     gtk::glib::ControlFlow::Continue
        // });

        // SizeClass::Compact
    }
}
