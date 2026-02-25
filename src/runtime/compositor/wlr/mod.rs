mod action;
mod dispatch;
mod matching;
mod state;

pub fn activate(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    action::activate(app_id, title, duplicate_rank)
}

pub fn close(app_id: &str, title: &str, duplicate_rank: usize) -> Result<(), String> {
    action::close(app_id, title, duplicate_rank)
}

pub fn set_minimized(
    app_id: &str,
    title: &str,
    duplicate_rank: usize,
    minimized: bool,
) -> Result<(), String> {
    action::set_minimized(app_id, title, duplicate_rank, minimized)
}
