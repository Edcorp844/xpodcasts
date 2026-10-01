use gtk::prelude::*;
use relm4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::util::progress_color::ProgressColor;

#[derive(Debug, Clone)]
pub struct ProgressBar {
    fraction: Rc<RefCell<f64>>,
    interactive: bool,
}

#[derive(Debug)]
pub enum ProgressBarInput {
    SetFraction(f64),
    HandleScrub(f64),
}

#[derive(Debug)]
pub enum ProgressBarOutput {
    FractionChanged(f64),
}

pub struct ProgressBarInit {
    pub initial_fraction: f64,
    pub interactive: bool,
    pub color: ProgressColor,
}

impl Default for ProgressBarInit {
    fn default() -> Self {
        Self {
            initial_fraction: 0.0,
            interactive: false,
            color: ProgressColor::Default,
        }
    }
}

#[relm4::component(pub)]
impl Component for ProgressBar {
    type Init = ProgressBarInit;
    type Input = ProgressBarInput;
    type Output = ProgressBarOutput;
    type CommandOutput = ();

    view! {
        gtk::DrawingArea {
            set_hexpand: true,
            set_vexpand: true,

            add_controller = gtk::GestureClick {
                set_button: gtk::gdk::BUTTON_PRIMARY,
                connect_pressed[sender, model] => move |gesture, _, x, _| {
                    if model.interactive {
                        if let Some(widget) = gesture.widget() {
                            let width = widget.width() as f64;
                            if width > 0.0 {
                                sender.input(ProgressBarInput::HandleScrub(x / width));
                            }
                        }
                    }
                }
            },

            add_controller = gtk::GestureDrag {
                set_button: gtk::gdk::BUTTON_PRIMARY,
                connect_drag_update[sender, model] => move |gesture, offset_x, _| {
                    if model.interactive {
                        if let Some(widget) = gesture.widget() {
                            let width = widget.width() as f64;
                            if let Some((start_x, _)) = gesture.start_point() {
                                let target_x = start_x + offset_x;
                                if width > 0.0 {
                                    sender.input(ProgressBarInput::HandleScrub(target_x / width));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fraction_data = Rc::new(RefCell::new(init.initial_fraction.clamp(0.0, 1.0)));
        let color_config = init.color;

        let model = ProgressBar {
            fraction: fraction_data.clone(),
            interactive: init.interactive,
        };

        let widgets = view_output!();

        root.set_draw_func(move |widget, cr, width, height| {
            let fraction = *fraction_data.borrow();

            let w = width as f64;
            let h = height as f64;
            let radius = h / 2.0;

            if w <= 0.0 || h <= 0.0 {
                return;
            }

            let draw_rounded_rect =
                |cairo_ctx: &gtk::cairo::Context, x: f64, y: f64, rect_w: f64, rect_h: f64| {
                    if rect_w <= 0.0 {
                        return;
                    }
                    cairo_ctx.new_sub_path();
                    cairo_ctx.arc(
                        x + rect_w - radius,
                        y + radius,
                        radius,
                        -std::f64::consts::PI / 2.0,
                        0.0,
                    );
                    cairo_ctx.arc(
                        x + rect_w - radius,
                        y + rect_h - radius,
                        radius,
                        0.0,
                        std::f64::consts::PI / 2.0,
                    );
                    cairo_ctx.arc(
                        x + radius,
                        y + rect_h - radius,
                        radius,
                        std::f64::consts::PI / 2.0,
                        std::f64::consts::PI,
                    );
                    cairo_ctx.arc(
                        x + radius,
                        y + radius,
                        radius,
                        std::f64::consts::PI,
                        3.0 * std::f64::consts::PI / 2.0,
                    );
                    cairo_ctx.close_path();
                };

            // 1. RESOLVE THE PRIMARY COLOR (theme accent, named string, or explicit rgba)
            let primary = color_config.resolve(widget);

            // 2. DRAW BACKGROUND TRACK — primary color at low opacity
            cr.set_source_rgba(
                primary.red() as f64,
                primary.green() as f64,
                primary.blue() as f64,
                0.15,
            );
            draw_rounded_rect(&cr, 0.0, 0.0, w, h);
            let _ = cr.fill();

            // 3. DRAW FOREGROUND FILL — primary color at its own alpha
            if fraction > 0.0 {
                let fill_width = (w * fraction).max(radius * 2.0);

                cr.set_source_rgba(
                    primary.red() as f64,
                    primary.green() as f64,
                    primary.blue() as f64,
                    primary.alpha() as f64,
                );
                draw_rounded_rect(&cr, 0.0, 0.0, fill_width, h);
                let _ = cr.fill();
            }
        });

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            ProgressBarInput::SetFraction(f) => {
                if f.is_nan() || f.is_infinite() {
                    return;
                }
                let clamped = f.clamp(0.0, 1.0);

                if let Ok(mut guard) = self.fraction.try_borrow_mut() {
                    *guard = clamped;
                }
                root.queue_draw();
            }
            ProgressBarInput::HandleScrub(pct) => {
                if pct.is_nan() || pct.is_infinite() {
                    return;
                }
                let clamped = pct.clamp(0.0, 1.0);

                if let Ok(mut guard) = self.fraction.try_borrow_mut() {
                    *guard = clamped;
                }
                root.queue_draw();

                let _ = sender.output(ProgressBarOutput::FractionChanged(clamped));
            }
        }
    }
}
