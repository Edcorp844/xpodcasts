use gtk::prelude::{StyleContextExt, WidgetExt};

/// Where the bar's primary color comes from.
#[derive(Debug, Clone)]
pub enum ProgressColor {
    /// Use the system/theme accent color (previous behavior).
    Default,
    /// Parse a CSS-style color string, e.g. "#ff6600", "rgb(255,102,0)", "orange",
    /// OR a libadwaita/GTK named color reference, e.g. "var(--success-color)"
    /// or "var(--warning-color)".
    Named(String),
    /// Explicit RGBA components in 0.0..=1.0.
    Rgba(f64, f64, f64, f64),
}

impl Default for ProgressColor {
    fn default() -> Self {
        ProgressColor::Default
    }
}

impl ProgressColor {
    pub fn resolve(&self, widget: &gtk::DrawingArea) -> gtk::gdk::RGBA {
        match self {
            ProgressColor::Default => Self::theme_accent(widget),
            ProgressColor::Named(s) => Self::resolve_named(s, widget),
            ProgressColor::Rgba(r, g, b, a) => {
                gtk::gdk::RGBA::new(*r as f32, *g as f32, *b as f32, *a as f32)
            }
        }
    }

    fn resolve_named(s: &str, widget: &gtk::DrawingArea) -> gtk::gdk::RGBA {
        let trimmed = s.trim();

        // Handle CSS-variable-style references: var(--success-color), var(--warning-color), etc.
        if let Some(inner) = trimmed
            .strip_prefix("var(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let name = inner.trim().trim_start_matches("--");
            let gtk_name = name.replace('-', "_");
            return Self::lookup_named_color(widget, &gtk_name);
        }

        // Handle GTK's native "@named_color" syntax too, for convenience.
        if let Some(name) = trimmed.strip_prefix('@') {
            return Self::lookup_named_color(widget, name);
        }

        // Otherwise treat it as a plain CSS color string.
        gtk::gdk::RGBA::parse(trimmed).unwrap_or_else(|_| Self::theme_accent(widget))
    }

    fn lookup_named_color(widget: &gtk::DrawingArea, name: &str) -> gtk::gdk::RGBA {
        let context = widget.style_context();
        context
            .lookup_color(name)
            .unwrap_or_else(|| Self::theme_accent(widget))
    }

    fn theme_accent(widget: &gtk::DrawingArea) -> gtk::gdk::RGBA {
        let context = widget.style_context();
        context
            .lookup_color("accent_color")
            .unwrap_or_else(|| context.color())
    }
}
