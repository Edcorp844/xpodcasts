use gtk::prelude::*;
use relm4::prelude::*;
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

#[derive(Debug)]
pub struct CircularProgress {
    fraction: Rc<RefCell<f64>>,
}

#[derive(Debug)]
pub enum CircularProgressInput {
    SetFraction(f64),
}

#[relm4::component(pub)]
impl Component for CircularProgress {
    type Init = f64;
    type Input = CircularProgressInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::DrawingArea {
            set_hexpand: true,
            set_vexpand: true,
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fraction_data = Rc::new(RefCell::new(init));
        let model = CircularProgress {
            fraction: fraction_data.clone(),
        };

        let widgets = view_output!();
        root.set_draw_func(move |widget, cr, width, height| {
            let fraction = *fraction_data.borrow();

            let size = width.min(height) as f64;
            let center_x = width as f64 / 2.0;
            let center_y = height as f64 / 2.0;

            let line_width = size * 0.10;
            let radius = (size - line_width) / 2.0;

            if radius <= 0.0 {
                return;
            }

            cr.set_line_width(line_width);
            cr.set_line_cap(gtk::gdk::cairo::LineCap::Round);

            // 1. LOOK UP SYSTEM ACCENT COLOR (via libadwaita, not deprecated StyleContext)
            let accent_gdk_rgba = adw::StyleManager::default().accent_color_rgba();

            // 2. DRAW BACKGROUND RING (Muted track color)
            cr.set_source_rgba(
                accent_gdk_rgba.red() as f64,
                accent_gdk_rgba.green() as f64,
                accent_gdk_rgba.blue() as f64,
                0.15,
            );
            cr.new_sub_path();
            cr.arc(center_x, center_y, radius, 0.0, 2.0 * PI);
            let _ = cr.stroke();

            // 3. DRAW FOREGROUND ACTIVE ARC (Dynamic system accent color)
            if fraction > 0.0 {
                let start_angle = -PI / 2.0;
                let end_angle = start_angle + (fraction * 2.0 * PI);

                cr.set_source_rgba(
                    accent_gdk_rgba.red() as f64,
                    accent_gdk_rgba.green() as f64,
                    accent_gdk_rgba.blue() as f64,
                    accent_gdk_rgba.alpha() as f64,
                );
                cr.new_sub_path();
                cr.arc(center_x, center_y, radius, start_angle, end_angle);
                let _ = cr.stroke();
            }

            // 4. DRAW PERCENTAGE TEXT IN THE CENTER
            cr.select_font_face(
                "Sans",
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );

            let font_size = size * 0.35;
            cr.set_font_size(font_size);

            let percentage_text = format!("{:.0}%", fraction * 100.0);

            if let Ok(extents) = cr.text_extents(&percentage_text) {
                let text_x = center_x - (extents.width() / 2.0) - extents.x_bearing();
                let text_y = center_y - (extents.height() / 2.0) - extents.y_bearing();

                cr.move_to(text_x, text_y);

                // Use widget's own (non-deprecated) foreground color method
                let text_color = widget.color();
                cr.set_source_rgba(
                    text_color.red() as f64,
                    text_color.green() as f64,
                    text_color.blue() as f64,
                    text_color.alpha() as f64,
                );

                let _ = cr.show_text(&percentage_text);
            }
        });

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CircularProgressInput::SetFraction(f) => {
                if f.is_nan() || f.is_infinite() {
                    return;
                }

                if let Ok(mut guard) = self.fraction.try_borrow_mut() {
                    *guard = f.clamp(0.0, 1.0);
                }
                root.queue_draw();
            }
        }
    }
}
