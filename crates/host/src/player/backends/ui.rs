use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ruffle_core::backend::ui::{
    DialogResultFuture, FileFilter, FontDefinition, FullscreenError, LanguageIdentifier,
    MouseCursor, MultiDialogResultFuture, NullUiBackend, UiBackend,
};
use ruffle_core::font::FontQuery;
use url::Url;

const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub(crate) struct Clipboard(mpsc::Sender<Request>);

enum Request {
    Read(mpsc::SyncSender<String>),
    Write(String),
}

trait Store {
    fn read(&mut self) -> String;
    fn write(&mut self, text: String);
}

impl Store for arboard::Clipboard {
    fn read(&mut self) -> String {
        self.get_text().unwrap_or_default()
    }

    fn write(&mut self, text: String) {
        if let Err(error) = self.set_text(text) {
            tracing::warn!(%error, "couldn't write the clipboard");
        }
    }
}

impl Clipboard {
    pub(crate) fn system() -> std::io::Result<Clipboard> {
        Clipboard::spawn(arboard::Clipboard::new)
    }

    fn spawn<S: Store, E: std::fmt::Display>(
        open: impl FnOnce() -> Result<S, E> + Send + 'static,
    ) -> std::io::Result<Clipboard> {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("lucent-clipboard".into())
            .spawn(move || {
                let mut store = match open() {
                    Ok(store) => store,
                    Err(error) => {
                        tracing::warn!(%error, "no system clipboard");
                        return;
                    }
                };
                for request in rx {
                    match request {
                        Request::Read(reply) => {
                            let _ = reply.send(store.read());
                        }
                        Request::Write(text) => store.write(text),
                    }
                }
            })?;
        Ok(Clipboard(tx))
    }

    fn read(&self) -> String {
        let (tx, rx) = mpsc::sync_channel(1);
        if self.0.send(Request::Read(tx)).is_err() {
            return String::new();
        }
        rx.recv_timeout(CLIPBOARD_READ_TIMEOUT).unwrap_or_default()
    }

    fn write(&self, text: String) {
        let _ = self.0.send(Request::Write(text));
    }
}

pub(crate) struct Ui {
    clipboard: Clipboard,
    rest: NullUiBackend,
}

impl Ui {
    pub(crate) fn new(clipboard: Clipboard) -> Ui {
        Ui {
            clipboard,
            rest: NullUiBackend::new(),
        }
    }
}

impl UiBackend for Ui {
    fn clipboard_content(&mut self) -> String {
        self.clipboard.read()
    }

    fn set_clipboard_content(&mut self, content: String) {
        self.clipboard.write(content);
    }

    fn mouse_visible(&self) -> bool {
        self.rest.mouse_visible()
    }

    fn set_mouse_visible(&mut self, visible: bool) {
        self.rest.set_mouse_visible(visible);
    }

    fn set_mouse_cursor(&mut self, cursor: MouseCursor) {
        self.rest.set_mouse_cursor(cursor);
    }

    fn set_fullscreen(&mut self, is_full: bool) -> Result<(), FullscreenError> {
        self.rest.set_fullscreen(is_full)
    }

    fn display_root_movie_download_failed_message(&self, invalid_swf: bool, fetch_error: String) {
        self.rest
            .display_root_movie_download_failed_message(invalid_swf, fetch_error);
    }

    fn message(&self, message: &str) {
        self.rest.message(message);
    }

    fn open_virtual_keyboard(&self) {
        self.rest.open_virtual_keyboard();
    }

    fn close_virtual_keyboard(&self) {
        self.rest.close_virtual_keyboard();
    }

    fn language(&self) -> LanguageIdentifier {
        self.rest.language()
    }

    fn display_unsupported_video(&self, url: Url) {
        self.rest.display_unsupported_video(url);
    }

    fn load_device_font(&self, query: &FontQuery, register: &mut dyn FnMut(FontDefinition)) {
        self.rest.load_device_font(query, register);
    }

    fn sort_device_fonts(
        &self,
        query: &FontQuery,
        register: &mut dyn FnMut(FontDefinition),
    ) -> Vec<FontQuery> {
        self.rest.sort_device_fonts(query, register)
    }

    fn display_file_open_dialog(&mut self, filters: Vec<FileFilter>) -> Option<DialogResultFuture> {
        self.rest.display_file_open_dialog(filters)
    }

    fn display_file_open_dialog_multiple(
        &mut self,
        filters: Vec<FileFilter>,
    ) -> Option<MultiDialogResultFuture> {
        self.rest.display_file_open_dialog_multiple(filters)
    }

    fn display_file_save_dialog(
        &mut self,
        file_name: String,
        title: String,
    ) -> Option<DialogResultFuture> {
        self.rest.display_file_save_dialog(file_name, title)
    }

    fn close_file_dialog(&mut self) {
        self.rest.close_file_dialog();
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;

    struct System {
        text: String,
        writes: mpsc::Sender<String>,
    }

    impl Store for System {
        fn read(&mut self) -> String {
            self.text.clone()
        }

        fn write(&mut self, text: String) {
            self.writes.send(text).unwrap();
        }
    }

    struct Stuck(mpsc::Receiver<()>);

    impl Store for Stuck {
        fn read(&mut self) -> String {
            let _ = self.0.recv();
            "too late".into()
        }

        fn write(&mut self, _text: String) {}
    }

    #[test]
    fn paste_reads_the_system_clipboard() {
        let (writes, _) = mpsc::channel();
        let system = System {
            text: "hunter2".into(),
            writes,
        };
        let mut ui = Ui::new(Clipboard::spawn(move || Ok::<_, Infallible>(system)).unwrap());
        assert!(ui.clipboard_available());
        assert_eq!(ui.clipboard_content(), "hunter2");
    }

    #[test]
    fn copy_writes_the_system_clipboard() {
        let (writes, written) = mpsc::channel();
        let system = System {
            text: String::new(),
            writes,
        };
        let mut ui = Ui::new(Clipboard::spawn(move || Ok::<_, Infallible>(system)).unwrap());
        ui.set_clipboard_content("copied".into());
        assert_eq!(
            written.recv_timeout(Duration::from_secs(1)),
            Ok("copied".into())
        );
    }

    #[test]
    fn a_stuck_clipboard_gives_up_instead_of_holding_the_player() {
        let (_release, stuck) = mpsc::channel();
        let clipboard = Clipboard::spawn(move || Ok::<_, Infallible>(Stuck(stuck))).unwrap();
        let (done, pasted) = mpsc::channel();
        thread::spawn(move || {
            let mut ui = Ui::new(clipboard);
            let text = ui.clipboard_content();
            ui.set_clipboard_content("copied".into());
            done.send(text).unwrap();
        });
        assert_eq!(
            pasted.recv_timeout(Duration::from_secs(2)),
            Ok(String::new())
        );
    }
}
