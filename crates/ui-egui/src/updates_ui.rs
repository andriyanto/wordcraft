//! Software updates: a check in the background at startup (and on Help ▸ Check for Updates…),
//! and a prompt that offers the new release for download.
//!
//! **This copy never installs an update itself.** Its builds are not code-signed, and an
//! installer the app fetched and started on its own is what Windows SmartScreen stops, with the
//! app already closed for the update. So the prompt has Download and Later only: Download, and
//! Help ▸ Download Latest…, open the release's page in the browser, where the person
//! downloads the installer and runs it themselves, with the warning the system shows them.
//!
//! The check and the two windows come from the update module, `crates/update` (`check_for_update`, `StartupPrompt`,
//! `CheckResultDialog`); what is left out is its self-update (`updater::Updater`).

use crate::WordApp;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, TryRecvError, channel};
#[cfg(not(target_arch = "wasm32"))]
use wordcraft_update::dialog::{CheckResultDialog, CheckResultResponse};
#[cfg(not(target_arch = "wasm32"))]
use wordcraft_update::updater::Status;
#[cfg(not(target_arch = "wasm32"))]
use wordcraft_update::{Release, StartupPrompt, StartupPromptResponse};

#[cfg(not(target_arch = "wasm32"))]
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a check came to: a newer release, none, or why it could not be asked.
#[cfg(not(target_arch = "wasm32"))]
type Checked = Result<Option<Release>, String>;
/// Asks for the newest release (blocking; it runs on its own thread). Tests stand in for the
/// network with one.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type CheckSource = std::sync::Arc<dyn Fn() -> Checked + Send + Sync>;

pub(crate) struct Runtime {
    /// The desktop app turns the check at start on; tests and the web build leave it off.
    #[cfg(not(target_arch = "wasm32"))]
    startup_check: bool,
    /// The check at start has been started; it runs once.
    #[cfg(not(target_arch = "wasm32"))]
    started: bool,
    /// Help ▸ Check for Updates… was chosen; the next `tick` starts it.
    #[cfg(not(target_arch = "wasm32"))]
    check_requested: bool,
    /// The check running was asked for by the person: its outcome is said whatever it is.
    #[cfg(not(target_arch = "wasm32"))]
    requested: bool,
    #[cfg(not(target_arch = "wasm32"))]
    check: Option<Receiver<Checked>>,
    /// What a requested check came to, for the result window.
    #[cfg(not(target_arch = "wasm32"))]
    status: Status,
    /// A newer release, on offer.
    #[cfg(not(target_arch = "wasm32"))]
    prompt: Option<StartupPrompt>,
    #[cfg(not(target_arch = "wasm32"))]
    result_dialog: CheckResultDialog,
    /// In place of the network (tests).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) source: Option<CheckSource>,
    /// The context of the last frame, to wake the UI when a check is queued from a menu callback.
    #[cfg(not(target_arch = "wasm32"))]
    repaint: Option<egui::Context>,
    pub(crate) manual_window_open: bool,
    #[cfg(target_arch = "wasm32")]
    pub(crate) status_message: Option<String>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            startup_check: false,
            #[cfg(not(target_arch = "wasm32"))]
            started: false,
            #[cfg(not(target_arch = "wasm32"))]
            check_requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            check: None,
            #[cfg(not(target_arch = "wasm32"))]
            status: Status::Idle,
            #[cfg(not(target_arch = "wasm32"))]
            prompt: None,
            #[cfg(not(target_arch = "wasm32"))]
            result_dialog: CheckResultDialog::new(&crate::UPDATE_APP, VERSION).with_translator(translate),
            #[cfg(not(target_arch = "wasm32"))]
            source: None,
            #[cfg(not(target_arch = "wasm32"))]
            repaint: None,
            manual_window_open: false,
            #[cfg(target_arch = "wasm32")]
            status_message: None,
        }
    }
}

/// The update prompt's and the result window's words in the interface language: the update module
/// passes its English text and the values of its `{name}` placeholders each time it draws.
#[cfg(not(target_arch = "wasm32"))]
fn translate(text: &str, args: &[(&str, &str)]) -> String {
    crate::i18n::fmt(crate::i18n::t(text), args)
}

/// The version this build compares releases with: its own, or the one `WORDCRAFT_PRETEND_VERSION`
/// names for this run (to see the prompt without waiting for a release).
#[cfg(not(target_arch = "wasm32"))]
fn running_version() -> String {
    wordcraft_update::self_update::current_version(&crate::UPDATE_APP, VERSION).map_or_else(|| VERSION.to_string(), |v| v.to_string())
}

/// The page Help ▸ Download Latest… opens: this repository's releases, the newest first. (Not
/// `releases/latest`, which is no page while every release is a pre-release.)
pub(crate) fn download_page() -> String {
    format!("https://github.com/{}/releases", crate::UPDATE_REPO)
}

/// The page Download opens for `release`: its own page when the link is a plain web link, else
/// this repository's releases. (A `Release` can come from a cache or a feed, so its link is
/// checked here as the update module checks it.)
#[cfg(not(target_arch = "wasm32"))]
fn page_of(release: &Release) -> String {
    if wordcraft_update::release::safe_web_link(&release.html_url) { release.html_url.clone() } else { crate::UPDATE_APP.releases_url() }
}

impl WordApp {
    /// Whether the app asks for a newer release when it starts (off until the desktop app turns it on).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup_check = on;
    }
}

pub(crate) fn check(app: &mut WordApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        rt.check_requested = true;
        // The result window shows the check from this frame on, also when the command ran while
        // the ribbon drew, after the frame's `tick`.
        rt.status = Status::Checking;
        rt.manual_window_open = true;
        if let Some(ctx) = &rt.repaint {
            ctx.request_repaint();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.updates_rt.manual_window_open = true;
        app.updates_rt.status_message = Some(tl!("Update check is not supported in the web browser.").to_string());
    }
}

/// Help ▸ Download Latest…: open this repository's releases page in the browser.
pub(crate) fn download(app: &mut WordApp) {
    app.canvas.open_url = Some(download_page());
}

#[cfg(not(target_arch = "wasm32"))]
impl Runtime {
    fn begin(&mut self, ctx: &egui::Context, requested: bool) {
        if requested {
            self.requested = true;
            self.status = Status::Checking;
        }
        if self.check.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let (source, version, ctx) = (self.source.clone(), running_version(), ctx.clone());
        let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let ask = || match source {
                Some(ask) => ask(),
                None => wordcraft_update::Http::new(&format!("{}/Update", crate::UPDATE_APP.name))
                    .and_then(|http| wordcraft_update::check_for_update(&crate::UPDATE_APP, &http, &version)),
            };
            let answer =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(ask)).unwrap_or_else(|_| Err("the update check failed unexpectedly".into()));
            let _ = tx.send(answer);
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => self.check = Some(rx),
            Err(e) => self.finish(Err(format!("couldn't start the update check ({e})"))),
        }
    }

    /// Takes in a check's answer. The check at start says nothing unless it found a release.
    fn finish(&mut self, answer: Checked) {
        let requested = std::mem::take(&mut self.requested);
        match answer {
            Ok(Some(release)) => {
                // Download and Later: never "Update and Restart" (see the module docs).
                let mut prompt = StartupPrompt::new(&crate::UPDATE_APP, release, running_version()).with_translator(translate);
                prompt.is_self_updating = false;
                self.prompt = Some(prompt);
                self.status = Status::Idle;
            }
            Ok(None) if requested => self.status = Status::UpToDate,
            Ok(None) => {}
            Err(e) => {
                log::warn!("WordKu update check failed: {e}");
                if requested {
                    self.status = Status::Failed(e);
                }
            }
        }
    }
}

pub(crate) fn tick(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        rt.repaint = Some(ctx.clone());
        if std::mem::take(&mut rt.check_requested) {
            rt.begin(ctx, true);
        } else if rt.startup_check && !rt.started {
            rt.started = true;
            rt.begin(ctx, false);
        }
        let Some(rx) = &rt.check else { return };
        match rx.try_recv() {
            Ok(answer) => {
                rt.check = None;
                rt.finish(answer);
            }
            // The answer is on its way: look again soon, also when nothing else repaints.
            Err(TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
            Err(TryRecvError::Disconnected) => {
                rt.check = None;
                rt.finish(Err("the update check stopped unexpectedly".into()));
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        if let Some(prompt) = &mut rt.prompt {
            match prompt.show(ctx) {
                StartupPromptResponse::Pending => {}
                // The prompt has no "Update and Restart" here; were it ever answered, it downloads.
                StartupPromptResponse::Download | StartupPromptResponse::UpdateAndRestart => {
                    ctx.open_url(egui::OpenUrl::new_tab(page_of(&prompt.release)));
                    rt.prompt = None;
                }
                StartupPromptResponse::Later => rt.prompt = None,
            }
        }
        // The result window answers Help ▸ Check for Updates; a found update leaves the status
        // idle and the prompt above takes over.
        rt.manual_window_open = !matches!(rt.status, Status::Idle) && rt.prompt.is_none();
        if rt.manual_window_open {
            match rt.result_dialog.show(ctx, &rt.status) {
                CheckResultResponse::None => {}
                CheckResultResponse::Close | CheckResultResponse::RestartToUpdate => {
                    rt.manual_window_open = false;
                    rt.status = Status::Idle;
                }
                CheckResultResponse::CheckAgain => rt.check_requested = true,
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    if app.updates_rt.manual_window_open {
        let mut open = true;
        egui::Window::new(tl!("Check for Updates"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                crate::rtl::root(ui, |ui| {
                    if let Some(msg) = &app.updates_rt.status_message {
                        ui.label(msg);
                    }
                })
            });
        if !open {
            app.updates_rt.manual_window_open = false;
            app.updates_rt.status_message = None;
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Updates come from this project's own releases, never from the Craft App Suite's
    /// repository, and are WordKu's own (its own asset names).
    #[test]
    fn updates_come_from_this_projects_own_repository() {
        let (app, suite) = (crate::UPDATE_APP, wordcraft_update::suite::WORDCRAFT);
        assert_eq!(app.repo, "andriyanto/wordku");
        assert_eq!(app.releases_url(), "https://github.com/andriyanto/wordku/releases");
        assert_ne!(app.repo, suite.repo);
        assert_ne!(app.developer, suite.developer);
        // WordKu is its own app: its own asset names, not WordCraft's.
        assert_eq!((app.id, app.name), ("wordku", "WordKu"));
        assert_ne!(app.id, suite.id);
        assert_eq!(download_page(), "https://github.com/andriyanto/wordku/releases");
    }

    fn release(page: &str) -> Release {
        Release { tag: "v99.0.0".into(), html_url: page.into(), ..Release::default() }
    }

    /// An app whose check answers `answer`.
    fn app_with(answer: Checked) -> WordApp {
        let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        app.updates_rt.source = Some(Arc::new(move || answer.clone()));
        app
    }

    /// Frames until the check running has answered.
    fn settle(app: &mut WordApp, ctx: &egui::Context) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            tick(app, ctx);
            if app.updates_rt.check.is_none() && !app.updates_rt.check_requested {
                return;
            }
            assert!(std::time::Instant::now() < deadline, "the check never ended: {:?}", app.updates_rt.status);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    /// One frame of the update windows: every text it painted with where it is, and the link the
    /// frame asked the system to open.
    fn frame(app: &mut WordApp, ctx: &egui::Context, events: Vec<egui::Event>) -> (Vec<(String, egui::Pos2)>, Option<String>) {
        fn collect(shape: &egui::Shape, out: &mut Vec<(String, egui::Pos2)>) {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
                egui::Shape::Text(t) => out.push((t.galley.text().to_owned(), t.pos + t.galley.rect.center().to_vec2())),
                _ => {}
            }
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
        let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
            tick(app, ui.ctx());
            show(app, ui.ctx());
        });
        output.textures_delta.clear(); // Headless: no renderer takes the font atlas.
        let mut texts = Vec::new();
        output.shapes.iter().for_each(|c| collect(&c.shape, &mut texts));
        let opened = output.platform_output.commands.iter().find_map(|c| match c {
            egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
            _ => None,
        });
        (texts, opened)
    }

    /// A newer release is offered for download: the prompt can't install it.
    #[test]
    fn a_newer_release_is_offered_for_download_and_never_installed() {
        let page = "https://github.com/andriyanto/wordku/releases/tag/v99.0.0";
        let mut app = app_with(Ok(Some(release(page))));
        app.set_startup_update_check(true);
        let ctx = egui::Context::default();
        settle(&mut app, &ctx);
        let prompt = app.updates_rt.prompt.as_ref().expect("the release is on offer");
        assert!(!prompt.is_self_updating && !prompt.is_ready && !prompt.is_installing, "Download and Later only");
        assert_eq!(page_of(&prompt.release), page);
        assert_eq!(app.updates_rt.status, Status::Idle, "a check at start opens no result window");
        // The check at start runs once.
        tick(&mut app, &ctx);
        assert!(app.updates_rt.check.is_none());
    }

    /// What the person sees: Download and Later, no button that installs; Download opens the
    /// release's page in the browser and closes the prompt.
    #[test]
    fn the_prompt_has_download_and_later_and_download_opens_the_browser() {
        let page = "https://github.com/andriyanto/wordku/releases/tag/v99.0.0";
        let mut app = app_with(Ok(Some(release(page))));
        app.set_startup_update_check(true);
        let ctx = egui::Context::default();
        settle(&mut app, &ctx);
        frame(&mut app, &ctx, Vec::new());
        let (texts, opened) = frame(&mut app, &ctx, Vec::new());
        let has = |label: &str| texts.iter().any(|(t, _)| t.eq_ignore_ascii_case(label));
        assert!(has("Download") && has("Later"), "{texts:?}");
        assert!(!has("Update and Restart") && !has("Restart to Update"), "{texts:?}");
        assert_eq!(opened, None, "nothing opens until Download is pressed");
        let at = texts.iter().find(|(t, _)| t.eq_ignore_ascii_case("Download")).map(|(_, at)| *at).unwrap();
        let button =
            |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at)]);
        frame(&mut app, &ctx, vec![button(true)]);
        let (_, opened) = frame(&mut app, &ctx, vec![button(false)]);
        assert_eq!(opened.as_deref(), Some(page));
        assert!(app.updates_rt.prompt.is_none(), "the prompt closes once the page is open");
    }

    #[test]
    fn a_link_that_is_no_plain_web_link_opens_the_releases_page_instead() {
        for odd in ["file:///C:/Windows/System32/calc.exe", "javascript:alert(1)", "", "http://example.com/x"] {
            let page = page_of(&release(odd));
            assert!(page == odd || page == "https://github.com/andriyanto/wordku/releases", "{odd} → {page}");
            assert!(wordcraft_update::release::safe_web_link(&page), "{odd} → {page}");
        }
    }

    /// Help ▸ Check for Updates runs while the ribbon draws, after the frame's `tick`: the result
    /// window shows the check from that frame on, and then what it came to.
    #[test]
    fn a_check_asked_for_while_drawing_opens_the_result_window() {
        let mut app = app_with(Err("offline".into()));
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            tick(&mut app, ui.ctx());
            check(&mut app);
            show(&mut app, ui.ctx());
        });
        out.textures_delta.clear();
        assert_eq!(app.updates_rt.status, Status::Checking);
        assert!(app.updates_rt.manual_window_open, "the result window shows the check");
        settle(&mut app, &ctx);
        frame(&mut app, &ctx, Vec::new());
        assert_eq!(app.updates_rt.status, Status::Failed("offline".into()));
        assert!(app.updates_rt.manual_window_open, "the result window says why");
        assert!(app.updates_rt.prompt.is_none());
    }

    /// A requested check that finds nothing newer says so; the same check at start says nothing,
    /// and neither does one that fails at start. Esc closes the result window.
    #[test]
    fn a_requested_check_reports_its_outcome_and_a_check_at_start_only_a_release() {
        let ctx = egui::Context::default();
        let mut app = app_with(Ok(None));
        check(&mut app);
        settle(&mut app, &ctx);
        frame(&mut app, &ctx, Vec::new());
        assert_eq!(app.updates_rt.status, Status::UpToDate);
        assert!(app.updates_rt.manual_window_open);
        let escape = egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        frame(&mut app, &ctx, vec![escape]);
        assert!(!app.updates_rt.manual_window_open, "Esc closes the window");
        assert_eq!(app.updates_rt.status, Status::Idle, "closing clears the status");

        for quiet in [Ok(None), Err("offline".to_string())] {
            let mut app = app_with(quiet);
            app.set_startup_update_check(true);
            settle(&mut app, &ctx);
            frame(&mut app, &ctx, Vec::new());
            assert_eq!(app.updates_rt.status, Status::Idle);
            assert!(!app.updates_rt.manual_window_open && app.updates_rt.prompt.is_none());
        }
    }

    /// Help ▸ Download Latest… opens this repository's releases in the browser.
    #[test]
    fn the_download_command_opens_the_releases_page() {
        let mut app = app_with(Ok(None));
        app.ui_command("ui.downloadUpdate", &serde_json::json!({})).expect("a UI command").expect("it runs");
        assert_eq!(app.canvas.open_url.as_deref(), Some("https://github.com/andriyanto/wordku/releases"));
    }
}
