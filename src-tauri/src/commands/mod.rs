pub mod app;
pub mod attachments;
pub mod conversations;
pub mod projects;
pub mod providers;
pub mod settings;
pub mod tasks;

use std::sync::Arc;

use tauri::State;

use crate::state::AppContext;

pub(crate) fn ctx(state: &State<'_, Arc<AppContext>>) -> Arc<AppContext> {
    state.inner().clone()
}
