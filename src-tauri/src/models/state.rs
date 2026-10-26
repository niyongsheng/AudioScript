use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::status::ModelJobStatus;

pub struct ActiveJob {
    pub job_id: String,
    pub cancel: Arc<AtomicBool>,
    pub status: Arc<Mutex<ModelJobStatus>>,
}

#[derive(Default)]
pub struct ModelState {
    job: Mutex<Option<ActiveJob>>,
}

impl ModelState {
    pub fn snapshot(&self) -> Option<ModelJobStatus> {
        let guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .as_ref()
            .map(|job| job.status.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    pub fn is_busy(&self) -> bool {
        self.job.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    pub fn start(
        &self,
        model_id: &str,
    ) -> Option<(String, Arc<AtomicBool>, Arc<Mutex<ModelJobStatus>>)> {
        let mut guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_some() {
            return None;
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(ModelJobStatus::new(
            job_id.clone(),
            model_id.to_owned(),
        )));

        *guard = Some(ActiveJob {
            job_id: job_id.clone(),
            cancel: Arc::clone(&cancel),
            status: Arc::clone(&status),
        });

        Some((job_id, cancel, status))
    }

    pub fn cancel(&self, job_id: &str) -> bool {
        let guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(job) if job.job_id == job_id => {
                job.cancel.store(true, Ordering::SeqCst);
                true
            }
            _ => false,
        }
    }

    pub fn clear(&self, job_id: &str) {
        let mut guard = self.job.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref().is_some_and(|job| job.job_id == job_id) {
            *guard = None;
        }
    }
}
