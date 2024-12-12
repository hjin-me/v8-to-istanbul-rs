use std::time;
use tracing::info;

pub struct Timer {
    start: time::Instant,
    name: String,
}
impl Timer {
    pub fn new(name: &str) -> Self {
        Timer {
            start: time::Instant::now(),
            name: name.to_string(),
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let now = time::Instant::now();
        info!(
            "{}耗时: {:.3}s",
            self.name,
            (now - self.start).as_secs_f32()
        );
    }
}
