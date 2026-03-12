use crate::init::START_TIME;

pub fn process_uptime() -> Dhms {
    let time = START_TIME.get().unwrap();
    Dhms(time.elapsed())
}

pub struct Dhms(pub std::time::Duration);

impl std::fmt::Display for Dhms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let secs = self.0.as_secs();

        let d = secs / 86_400;
        let h = (secs % 86_400) / 3_600;
        let m = (secs % 3_600) / 60;
        let s = secs % 60;

        write!(f, "{}:{:02}:{:02}:{:02}", d, h, m, s)
    }
}
