mod chapter_parser;
mod i18n;
mod util;

#[test]
fn init_gtk_tests() -> anyhow::Result<()> {
    gst::init()?;
    gtk::init()?;
    adw::init()?;
    Ok(())
}

