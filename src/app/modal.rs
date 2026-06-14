use crate::event::ConfirmAction;

#[derive(Debug, Clone)]
pub enum Modal {
    Help,
    Filter,
    Command,
    CommandError(String),
    Confirm(ConfirmAction),
    Details,
}
