use crate::{Feed, Filter};

pub struct ListFeedsCmd;
pub struct ListFiltersCmd;
pub struct AddFeedCmd(pub Feed);
pub struct AddFilterCmd(pub Filter);
pub struct UpdateCmd;
