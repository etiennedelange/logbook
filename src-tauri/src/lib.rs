mod commands;
mod update;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use logbook_core::service::SessionService;
use tauri::{AppHandle, Emitter, Manager};

/// Shared by the startup indexer and every UI command.
pub(crate) struct AppState {
    pub service: Arc<SessionService>,
    /// True while an indexer run is in progress, so a second one isn't started on top.
    pub indexing: Arc<AtomicBool>,
}

/// Re-scans changed sessions on a blocking thread, then tells the UI to refetch.
/// A no-op if a scan is already running.
pub(crate) fn reindex(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.indexing.swap(true, Ordering::SeqCst) {
        return;
    }
    let service = state.service.clone();
    let indexing = state.indexing.clone();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Clears the flag even if the scan panics, so a later reindex can still run.
        let _done = ClearOnDrop(indexing);
        if let Err(e) = service.rebuild() {
            eprintln!("indexing failed: {e:#}");
        }
        let _ = app.emit("index-updated", ());
    });
}

struct ClearOnDrop(Arc<AtomicBool>);

impl Drop for ClearOnDrop {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Turns off browser chrome in the WebView2 (context menu, Ctrl+P/F/R/S, F5/F12, status bar, ...)
/// so the window behaves like a native app. Copy/paste inside inputs still works.
#[cfg(windows)]
fn lock_down_webview(app: &AppHandle) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows_core::Interface;

    let Some(w) = app.get_webview_window("main") else { return };
    let _ = w.with_webview(|wv| unsafe {
        let Ok(core) = wv.controller().CoreWebView2() else { return };
        let Ok(settings) = core.Settings() else { return };
        let _ = settings.SetAreDefaultContextMenusEnabled(false);
        let _ = settings.SetAreDevToolsEnabled(cfg!(debug_assertions));
        let _ = settings.SetIsStatusBarEnabled(false);
        let _ = settings.SetIsZoomControlEnabled(false);
        let _ = settings.SetIsBuiltInErrorPageEnabled(false);
        if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
            let _ = s3.SetAreBrowserAcceleratorKeysEnabled(false);
        }
    });
}

fn show_main<R: tauri::Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The Tauri context. Debug builds only: `LOGBOOK_E2E_DEVTOOLS_PORT` opens the WebView2 DevTools
/// protocol on that port, so `tools/e2e` can drive the real app. (WebView2 ignores
/// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, which msedgedriver relies on, because wry always passes
/// its own arguments.)
fn context() -> tauri::Context<tauri::Wry> {
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(debug_assertions)]
    if let Some(port) = std::env::var("LOGBOOK_E2E_DEVTOOLS_PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
        for window in &mut context.config_mut().app.windows {
            // wry's defaults, plus the debugging port.
            window.additional_browser_args = Some(format!(
                "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-port={port}"
            ));
        }
    }
    context
}

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just brings the running window forward and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::index_status,
            commands::reindex_now,
            commands::recent_sessions,
            commands::search_sessions,
            commands::get_session,
            commands::get_transcript,
            commands::get_session_files,
            commands::get_file_edits,
            commands::resume_command,
            commands::resume_session,
            commands::open_folder,
            commands::open_link,
            update::check_update,
            update::install_update,
            update::open_release_page,
        ])
        .setup(|app| {
            #[cfg(windows)]
            lock_down_webview(app.handle());

            // Open the cache without scanning so the window shows cached sessions straight away.
            app.manage(AppState {
                service: Arc::new(SessionService::open()?),
                indexing: Arc::new(AtomicBool::new(false)),
            });
            reindex(app.handle());
            Ok(())
        })
        .run(context())
        .expect("error while running Logbook");
}
