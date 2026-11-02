//! Tells the open content tab when files appear in or leave its folder, so a mod copied in
//! through the file manager shows up without reopening the tab.

use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tauri::{AppHandle, Emitter};

const QUIET: Duration = Duration::from_millis(400);

/// Only one tab is open at a time, so one watcher is enough.
#[derive(Default)]
pub struct FolderWatch(Mutex<Option<Debouncer<RecommendedWatcher>>>);

impl FolderWatch {
    pub fn start(&self, app: &AppHandle, dir: &Path) -> notify_debouncer_mini::notify::Result<()> {
        let app = app.clone();
        let mut debouncer = new_debouncer(QUIET, move |result: DebounceEventResult| {
            if result.is_ok_and(|events| !events.is_empty()) {
                let _ = app.emit("content-changed", ());
            }
        })?;
        debouncer
            .watcher()
            .watch(dir, RecursiveMode::NonRecursive)?;
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(debouncer);
        Ok(())
    }

    pub fn stop(&self) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}
