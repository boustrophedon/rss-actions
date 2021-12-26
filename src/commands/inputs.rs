use crate::{Feed, Filter};

pub struct ListFeedsCmd;
pub struct ListFiltersCmd;
pub struct AddFeedCmd(pub Feed);
pub struct AddFilterCmd(pub Filter);
pub struct UpdateCmd;
/// Feed alias, filter keywords to match on
pub struct DeleteFilterCmd(pub String, pub Vec<String>);
impl DeleteFilterCmd {
    pub fn new(alias: &str, filters: &[&str]) -> DeleteFilterCmd {
        DeleteFilterCmd(alias.into(), filters.iter().map(|&s| s.into()).collect())
    }
}
/// Feed alias
pub struct DeleteFeedCmd(pub String);
