use std::time::{Duration, Instant};

use flexaudio_os_windows::list_output_devices;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

pub(super) struct DefaultEndpointFollower {
    last: Option<String>,
    next_poll: Instant,
}

impl DefaultEndpointFollower {
    pub(super) fn new() -> Self {
        Self {
            last: default_output_id(),
            next_poll: Instant::now() + POLL_INTERVAL,
        }
    }

    pub(super) fn changed(&mut self) -> Option<String> {
        let now = Instant::now();
        if now < self.next_poll {
            return None;
        }
        self.next_poll = now + POLL_INTERVAL;

        let current = default_output_id()?;
        if let Some(last) = &self.last {
            return (last != &current).then_some(current);
        }
        self.last = Some(current);
        None
    }

    pub(super) fn accept(&mut self) {
        if let Some(id) = default_output_id() {
            self.last = Some(id);
        }
    }
}

fn default_output_id() -> Option<String> {
    list_output_devices()
        .ok()?
        .into_iter()
        .find(|device| device.is_default)
        .map(|device| device.id)
}
