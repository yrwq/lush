use crate::config::DockClickAction;
use crate::runtime::compositor;

pub(super) fn run(
    action: &DockClickAction,
    app_id: &str,
    title: &str,
    duplicate_rank: usize,
) -> Result<(), String> {
    match action {
        DockClickAction::None => Ok(()),
        DockClickAction::Activate => compositor::activate_toplevel(app_id, title, duplicate_rank),
        DockClickAction::Close => compositor::close_toplevel(app_id, title, duplicate_rank),
        DockClickAction::Minimize => {
            compositor::set_toplevel_minimized(app_id, title, duplicate_rank, true)
        }
        DockClickAction::Restore => {
            compositor::set_toplevel_minimized(app_id, title, duplicate_rank, false)
        }
    }
}
