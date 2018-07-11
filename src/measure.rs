use time::PreciseTime;

pub struct Measure {
    start_time: PreciseTime,
    intermediate_time: Option<PreciseTime>,
}

impl Default for Measure {
    fn default() -> Self {
        Self {
            start_time: PreciseTime::now(),
            intermediate_time: None,
        }
    }
}

impl Measure {
    pub fn duration(&mut self) -> String {
        let end = PreciseTime::now();
        let duration = {
            if self.intermediate_time.is_none() {
                self.start_time.to(end)
            } else {
                self.intermediate_time.unwrap().to(end)
            }
        };

        self.intermediate_time = Some(PreciseTime::now());

        format!("{}", duration)
    }

    pub fn done(self) -> String {
        let end = PreciseTime::now();
        let duration = self.start_time.to(end);

        format!("{}", duration)
    }
}
