use std::time::Duration;

#[derive(Debug, Default)]
pub struct Statistic {
    pub image_time: Duration,
    pub propagate_time: Duration,
    pub propagate_time_a: Duration,
    pub propagate_time_b: Duration,
    pub post_reachable_time: Duration,
    pub fair_cycle_time: Duration,
}
