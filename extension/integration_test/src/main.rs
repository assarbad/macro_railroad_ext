const DIAGRAM_CONTAINER: &str =
    "#main-content > div.docblock.item-decl > div > div > div.railroad_container, #main > div.docblock.type-decl > div > div > div.railroad_container";
const OPTIONS: &str =
    "#main-content > div.docblock.item-decl > div > div > div.railroad_container > div > div > img, #main > div.docblock.type-decl > div > div > div.railroad_container";
const OPT_FULLSCREEN: &str =
    "#main-content > div.docblock.item-decl > div > div > div.railroad_container > div > img, #main > div.docblock.type-decl > div > div > div.railroad_container";
const URL_NAMED: &str = "https://docs.rs/nom/4.2.2/nom/macro.named_attr.html";
const URL_PANIC: &str = "https://doc.rust-lang.org/std/macro.panic.html";
const URL_INFO: &str = "https://docs.rs/tracing/0.1.36/tracing/macro.info.html";

use failure::Fallible;
use std::{env, fs, io, ops, sync::Arc, thread, time};

struct Browser {
    _ext: tempdir::TempDir,
    browser: headless_chrome::browser::Browser,
}

impl Browser {
    fn extract_extension() -> Fallible<tempdir::TempDir> {
        log::debug!("Extracting extension...");
        let packed_path =
            env::var_os("MACRO_RAILROAD_PACKED").expect("Archive path not given by env");
        let packed_f = fs::File::open(packed_path)?;
        let extract_path = tempdir::TempDir::new("mrtest")?;
        let mut zip_archive = zip::ZipArchive::new(packed_f)?;
        for i in 0..zip_archive.len() {
            let mut f = zip_archive.by_index(i)?;
            let fname = extract_path
                .path()
                .to_path_buf()
                .join(f.enclosed_name().unwrap());
            fs::create_dir_all(&fname.parent().unwrap())?;
            let mut e = std::fs::File::create(fname)?;
            io::copy(&mut f, &mut e)?;
        }
        Ok(extract_path)
    }

    fn new() -> Fallible<Self> {
        let ext = Self::extract_extension()?;
        log::debug!("Starting browser...");
        let browser = headless_chrome::Browser::new(
            headless_chrome::LaunchOptionsBuilder::default()
                .extensions(vec![ext.path().as_ref()])
                .window_size(Some((1600, 1400)))
                .path(Some(
                    headless_chrome::browser::default_executable().unwrap(),
                ))
                .headless(false)
                .build()
                .unwrap(),
        )?;
        log::info!("Browser version {:?}", browser.get_version()?);
        Ok(Self { _ext: ext, browser })
    }

    fn navigate_to_macro_page(
        &self,
        url: &str,
    ) -> Fallible<Arc<headless_chrome::browser::tab::Tab>> {
        let tab = self.wait_for_initial_tab()?;
        log::debug!("Navigating to `{}`", &url);
        tab.navigate_to(url)?;
        log::debug!("Waiting for decl-element");
        // Ignore if the selector is not there, might be uncollapsed already...
        if let Ok(elem) =
            tab.wait_for_element("#main > div.toggle-wrapper.collapsed > a > span.toggle-label")
        {
            elem.click()?;
        }
        log::debug!("Waiting for diagram");
        tab.wait_for_element(DIAGRAM_CONTAINER)?;
        log::debug!("Successfully navigated");
        Ok(tab)
    }

    #[cfg(test)]
    fn testable_tab(&self) -> Fallible<Arc<headless_chrome::browser::tab::Tab>> {
        self.navigate_to_macro_page(URL_PANIC)
    }
}

impl ops::Deref for Browser {
    type Target = headless_chrome::browser::Browser;

    fn deref(&self) -> &Self::Target {
        &self.browser
    }
}

fn main() -> Fallible<()> {
    env_logger::init();

    match env::args().last().as_deref().expect("no arguments") {
        "screenshots" => screenshots(),
        "playground" => playground(),
        p => panic!("unknown argument `{}`", p),
    }
}

fn playground() -> Fallible<()> {
    let browser = Browser::new()?;
    browser.navigate_to_macro_page(URL_PANIC)?;

    loop {
        log::info!("Waiting...");
        thread::sleep(time::Duration::from_secs(5));
    }
}

fn screenshots() -> Fallible<()> {
    let browser = Browser::new()?;

    let screenshot = |tab: Arc<headless_chrome::Tab>, fname: &str| -> Fallible<()> {
        let png_data = tab.capture_screenshot(
            headless_chrome::protocol::page::ScreenshotFormat::PNG,
            None,
            true,
        )?;
        fs::write(fname, &png_data)?;
        log::info!("Successfully screenshotted `{}`", &fname);
        Ok(())
    };

    let screenshot_fs = |url: &str, fname: &str| -> Fallible<()> {
        let tab = browser.navigate_to_macro_page(url)?;
        log::debug!("Waiting for Options...");
        tab.find_element(OPTIONS)?.click()?;
        log::debug!("Waiting for Fullscreen...");
        tab.wait_for_element(OPT_FULLSCREEN)?.click()?;
        thread::sleep(time::Duration::from_secs(2)); // Wait for the gfx
        screenshot(tab, fname)
    };

    let screenshot_opt = |url: &str, fname: &str| -> Fallible<()> {
        let tab = browser.navigate_to_macro_page(url)?;
        tab.find_element(OPTIONS)?.click()?;
        screenshot(tab, fname)
    };

    screenshot(browser.navigate_to_macro_page(URL_PANIC)?, "std_panic.png")?;

    screenshot(
        browser.navigate_to_macro_page(URL_NAMED)?,
        "nom_named_attr.png",
    )?;

    screenshot_fs(URL_PANIC, "std_panic_fs.png")?;
    screenshot_fs(URL_NAMED, "nom_named_attr_fs.png")?;

    screenshot_opt(URL_PANIC, "std_panic_options.png")?;
    screenshot_opt(URL_INFO, "std_info_options.png")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const LEGEND: &str =
        "#main-content > div.docblock.item-decl > div > div > div.railroad_container > svg > g > g.legend, #main > div.docblock.type-decl > div > div > div.railroad_container > svg > g > g.legend";
    const MAIN: &str = "#main-content, #main";
    const MODAL_CONTAINER: &str = "#main-content > div.docblock.item-decl > div > div > div.railroad_modal, #main > div.docblock.type-decl > div > div > div.railroad_modal";
    const OPT_LEGEND: &str = "#main-content > div.docblock.item-decl > div > div > div.railroad_container > div.railroad_dropdown_content.railroad_dropdown_show > ul > li:nth-child(4) > label, #main > div.docblock.type-decl > div > div > div.railroad_container > div.railroad_dropdown_content.railroad_dropdown_show > ul > li:nth-child(4) > label";
    const MACRO_BLOCK: &str = "#main-content > div.docblock.item-decl > div > div > pre, #main > div.docblock.type-decl > div > div > pre";
    const RAILROAD_CONTAINER: &str = "#main-content > div.docblock.item-decl > div > div > div.railroad_container, #main > div.docblock.type-decl > div > div > div.railroad_container";
    const DROPDOWN_CONTAINER: &str = "#main-content > div.docblock.item-decl > div > div > div.railroad_container > div.railroad_dropdown_content.railroad_dropdown_show, #main > div.docblock.type-decl > div > div > div.railroad_container > div.railroad_dropdown_content.railroad_dropdown_show";
    const URL_BITFLAGS: &str = "https://docs.rs/bitflags/1.1.0/bitflags/macro.bitflags.html";

    fn init_log() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn loads() -> Fallible<()> {
        init_log();
        let browser = Browser::new()?;
        let tab = browser.wait_for_initial_tab()?;
        tab.navigate_to("https://doc.rust-lang.org")?;
        // TODO assert the extension loaded
        // TODO assert that the stylesheet get loaded, via document.syleSheets
        tab.navigate_to("https://docs.rs")?;
        // TODO assert the extension loaded
        Ok(())
    }

    #[test]
    fn executes() -> Fallible<()> {
        init_log();
        let browser = Browser::new()?;
        let tab = browser.testable_tab()?;
        tab.find_element(MODAL_CONTAINER).map(|_| ())
    }

    fn test_placement(browser: &Browser, url: &str) -> Fallible<Arc<headless_chrome::Tab>> {
        let tab = browser.navigate_to_macro_page(url)?;
        log::debug!("Looking for main-box");
        let main_box = tab.find_element(MAIN)?.get_box_model()?;
        log::debug!("Looking for macro-box");
        let macro_block_box = tab.find_element(MACRO_BLOCK)?.get_box_model()?;
        assert!(macro_block_box.content.within_bounds_of(&main_box.margin));

        log::debug!("Looking for diagram-box");
        let inline_dia_box = tab.find_element(DIAGRAM_CONTAINER)?.get_box_model()?;
        assert!(inline_dia_box.content.within_bounds_of(&main_box.margin));
        assert!(inline_dia_box.content.above(&macro_block_box.margin));
        assert!(inline_dia_box
            .content
            .within_horizontal_bounds_of(&macro_block_box.margin));
        Ok(tab)
    }

    #[test]
    fn placement() -> Fallible<()> {
        init_log();
        let browser = Browser::new()?;
        test_placement(&browser, URL_PANIC)?;
        test_placement(&browser, URL_BITFLAGS)?;
        test_placement(&browser, URL_NAMED)?;
        Ok(())
    }

    #[test]
    fn issue13() -> Fallible<()> {
        init_log();
        let browser = Browser::new()?;

        let test = |browser: &Browser, url: &str| -> Fallible<()> {
            let tab = test_placement(&browser, url)?;
            tab.find_element(OPTIONS)?.click()?;
            log::debug!("Looking for dropdown-box");
            let railroad_box = tab.find_element(RAILROAD_CONTAINER)?.get_box_model()?;
            log::debug!("Looking for dropdown-box");
            let dropdown_box = tab.find_element(DROPDOWN_CONTAINER)?.get_box_model()?;

            assert_eq!(
                railroad_box.content.most_right(),
                dropdown_box.margin.most_right()
            );
            let most_bottom = railroad_box
                .margin
                .top_right
                .y
                .max(railroad_box.margin.top_left.y)
                .max(railroad_box.margin.bottom_right.y)
                .max(railroad_box.margin.bottom_left.y);
            assert_eq!(most_bottom, dropdown_box.margin.most_top());
            Ok(())
        };

        test(&browser, URL_PANIC)?;
        test(&browser, URL_INFO)?;

        Ok(())
    }

    #[test]
    fn set_options() -> Fallible<()> {
        init_log();
        let browser = Browser::new()?;
        let tab = browser.testable_tab()?;
        log::debug!("Looking for legend");
        assert!(tab.find_element(LEGEND).is_ok()); // Legend is there?
        log::debug!("Opening options...");
        tab.find_element(OPTIONS)?.click()?; // Open the options
        log::debug!("Disabling legend...");
        tab.wait_for_element(OPT_LEGEND)?.click()?; // Disable legend
        log::debug!("Waiting for legend to disappear...");
        assert!(headless_chrome::util::Wait::default()
            .until(|| tab.find_element(LEGEND).err())
            .is_ok());
        Ok(())
    }
}
