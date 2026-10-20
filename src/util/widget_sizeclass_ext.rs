use gtk::glib;
use gtk::prelude::{Cast, IsA, ObjectExt, WidgetExt};
use std::cell::Cell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

impl SizeClasses {
    pub const REGULAR: Self = Self {
        horizontal: SizeClass::Regular,
        vertical: SizeClass::Regular,
    };
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

impl SizeClass {
    pub fn to_orientation(self) -> gtk::Orientation {
        match self {
            SizeClass::Compact => gtk::Orientation::Vertical,
            SizeClass::Regular => gtk::Orientation::Horizontal,
        }
    }
}

pub enum LayoutRule {
    CollapseSidebar { when: SizeClass },
    HideInspector { when: SizeClass },
}

// ---------------------------------------------------------------------------
// Shared environment
// ---------------------------------------------------------------------------

/// Shared, cheap-to-clone handle to the current size classes.
///
/// Store this in the component's *widget* state (e.g. `additional_fields!`),
/// not in the model, and read it from `#[watch]` expressions. It's updated
/// in place by an `observe_size_classes` call, and a Relm4 message is only
/// emitted when the classification actually changes.
#[derive(Debug, Clone)]
pub struct SizeClassEnv {
    inner: Rc<Cell<SizeClasses>>,
}

impl SizeClassEnv {
    pub fn new(initial: SizeClasses) -> Self {
        Self {
            inner: Rc::new(Cell::new(initial)),
        }
    }

    /// Start optimistic (`Regular`) so nothing collapses before the first
    /// allocation arrives.
    pub fn with_defaults() -> Self {
        Self::new(SizeClasses::REGULAR)
    }

    #[inline]
    pub fn get(&self) -> SizeClasses {
        self.inner.get()
    }

    #[inline]
    pub fn horizontal(&self) -> SizeClass {
        self.inner.get().horizontal
    }

    #[inline]
    pub fn vertical(&self) -> SizeClass {
        self.inner.get().vertical
    }

    /// Internal: set + return whether it changed.
    fn update(&self, next: SizeClasses) -> bool {
        let prev = self.inner.get();
        if prev != next {
            self.inner.set(next);
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Observer handle
// ---------------------------------------------------------------------------

/// Returned by `observe_size_classes`. Keeps the notify handlers alive and
/// lets you disconnect them later. Dropping it *without* calling `disconnect`
/// is fine — the handlers live as long as the widget does.
pub struct SizeClassObserver {
    env: SizeClassEnv,
    widget: gtk::Widget,
    width_handler: glib::SignalHandlerId,
    height_handler: glib::SignalHandlerId,
}

impl SizeClassObserver {
    pub fn env(&self) -> &SizeClassEnv {
        &self.env
    }

    /// Disconnect both notify handlers. After this the env stops updating.
    pub fn disconnect(self) {
        self.widget.disconnect(self.width_handler);
        self.widget.disconnect(self.height_handler);
    }
}

// ---------------------------------------------------------------------------
// Extension trait
// ---------------------------------------------------------------------------

pub trait WidgetSizeClassExt: IsA<gtk::Widget> + 'static {
    // ----- synchronous reads -------------------------------------------------

    /// Classify the widget's *current* allocation. `None` if it hasn't been
    /// allocated yet (width or height is 0).
    fn try_size_classes(&self, thresholds: SizeThresholds) -> Option<SizeClasses> {
        let (w, h) = (self.width(), self.height());
        if w <= 0 || h <= 0 {
            return None;
        }
        Some(thresholds.classify(w, h))
    }

    /// Same as `try_size_classes`, but falls back to `Regular/Regular` if the
    /// widget has no allocation yet. Safe to call any time.
    fn size_classes(&self, thresholds: SizeThresholds) -> SizeClasses {
        self.try_size_classes(thresholds)
            .unwrap_or(SizeClasses::REGULAR)
    }

    fn horizontal_size_class(&self, thresholds: SizeThresholds) -> SizeClass {
        self.size_classes(thresholds).horizontal
    }

    fn vertical_size_class(&self, thresholds: SizeThresholds) -> SizeClass {
        self.size_classes(thresholds).vertical
    }

    // ----- observation ------------------------------------------------------

    /// Observe this widget's size classes and write them into `env` in place.
    /// `on_change` fires only on transitions, on the GTK main thread — feed it
    /// straight into `ComponentSender::input` so `#[watch]` re-evaluates.
    ///
    /// Because `env` is written even when `on_change` is a no-op, you can also
    /// pass `|_| {}` and just read `env` from the view.
    fn observe_size_classes_into(
        &self,
        env: SizeClassEnv,
        thresholds: SizeThresholds,
        on_change: impl Fn(SizeClasses) + 'static,
    ) -> SizeClassObserver {
        let widget: gtk::Widget = self.clone().upcast();
        let on_change = Rc::new(on_change);

        let update = {
            let widget = widget.clone();
            let env = env.clone();
            let on_change = Rc::clone(&on_change);

            move || {
                let (w, h) = (widget.width(), widget.height());
                if w <= 0 || h <= 0 {
                    return;
                }
                let next = thresholds.classify(w, h);
                if env.update(next) {
                    on_change(next);
                }
            }
        };

        let width_handler = widget.connect_notify_local(Some("width"), {
            let update = update.clone();
            move |_, _| update()
        });
        let height_handler = widget.connect_notify_local(Some("height"), {
            let update = update.clone();
            move |_, _| update()
        });

        // Catch the initial allocation if it already happened.
        update();

        SizeClassObserver {
            env,
            widget,
            width_handler,
            height_handler,
        }
    }

    /// Convenience: create a fresh env bound to this widget and return it.
    /// The widget's own allocation is the source of truth.
    fn track_size_classes(
        &self,
        thresholds: SizeThresholds,
        on_change: impl Fn(SizeClasses) + 'static,
    ) -> SizeClassObserver {
        let env = SizeClassEnv::with_defaults();
        self.observe_size_classes_into(env, thresholds, on_change)
    }

    /// Convenience: fire `on_change` only when the *horizontal* class changes.
    /// Useful when you only care about orientation, not vertical breakpoints.
    fn observe_horizontal_size_class(
        &self,
        thresholds: SizeThresholds,
        on_change: impl Fn(SizeClass) + 'static,
    ) {
        let previous: Rc<Cell<Option<SizeClass>>> = Rc::new(Cell::new(None));
        self.observe_size_classes_into(SizeClassEnv::with_defaults(), thresholds, move |classes| {
            if previous.get() != Some(classes.horizontal) {
                previous.set(Some(classes.horizontal));
                on_change(classes.horizontal);
            }
        });
    }

    /// Convenience: fire `on_change` only when the *vertical* class changes.
    fn observe_vertical_size_class(
        &self,
        thresholds: SizeThresholds,
        on_change: impl Fn(SizeClass) + 'static,
    ) {
        let previous: Rc<Cell<Option<SizeClass>>> = Rc::new(Cell::new(None));
        self.observe_size_classes_into(SizeClassEnv::with_defaults(), thresholds, move |classes| {
            if previous.get() != Some(classes.vertical) {
                previous.set(Some(classes.vertical));
                on_change(classes.vertical);
            }
        });
    }
}

impl<T: IsA<gtk::Widget> + 'static> WidgetSizeClassExt for T {}
