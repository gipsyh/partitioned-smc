use std::time::Duration;

#[derive(Debug, Default)]
pub struct Statistic {
    pub image_time: Duration,
    pub propagate_time: Duration,
    pub post_reachable_time: Duration,
}
