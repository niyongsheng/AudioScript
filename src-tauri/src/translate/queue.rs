use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub segment_id: String,
    pub text: String,
}

struct Inner {
    items: VecDeque<Job>,
    closed: bool,
}

pub struct JobQueue {
    inner: Mutex<Inner>,
    ready: Condvar,
    capacity: usize,
}

impl JobQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                items: VecDeque::new(),
                closed: false,
            }),
            ready: Condvar::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&self, job: Job) -> Option<Job> {
        let mut g = self.inner.lock().expect("job queue mutex poisoned");
        if g.closed {
            return Some(job);
        }

        let dropped = if g.items.len() >= self.capacity {
            g.items.pop_front()
        } else {
            None
        };

        g.items.push_back(job);
        drop(g);
        self.ready.notify_one();
        dropped
    }

    pub fn pop_blocking(&self) -> Option<Job> {
        let mut g = self.inner.lock().expect("job queue mutex poisoned");
        loop {
            if let Some(job) = g.items.pop_front() {
                return Some(job);
            }
            if g.closed {
                return None;
            }
            g = self.ready.wait(g).expect("job queue mutex poisoned");
        }
    }

    pub fn close(&self) {
        let mut g = self.inner.lock().expect("job queue mutex poisoned");
        g.closed = true;
        drop(g);
        self.ready.notify_all();
    }

    pub fn close_and_drain(&self) -> Vec<Job> {
        let mut g = self.inner.lock().expect("job queue mutex poisoned");
        g.closed = true;
        let jobs = g.items.drain(..).collect();
        drop(g);
        self.ready.notify_all();
        jobs
    }
}

impl Drop for JobQueue {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn job(id: &str) -> Job {
        Job {
            segment_id: id.into(),
            text: format!("text-{id}"),
        }
    }

    #[test]
    fn overflow_drops_the_oldest_not_the_newest() {
        let q = JobQueue::new(3);
        assert!(q.push(job("1")).is_none());
        assert!(q.push(job("2")).is_none());
        assert!(q.push(job("3")).is_none());

        let dropped = q.push(job("4")).expect("第 4 条应挤掉最旧的");
        assert_eq!(dropped.segment_id, "1", "丢的必须是最旧的");

        assert_eq!(q.pop_blocking().unwrap().segment_id, "2");
        assert_eq!(q.pop_blocking().unwrap().segment_id, "3");
        assert_eq!(q.pop_blocking().unwrap().segment_id, "4");
    }

    #[test]
    fn closed_queue_returns_none_so_consumer_can_exit() {
        let q = JobQueue::new(4);
        q.push(job("1"));
        q.close();

        assert_eq!(q.pop_blocking().unwrap().segment_id, "1");
        assert!(
            q.pop_blocking().is_none(),
            "关闭且空后应返回 None 让线程退出"
        );
    }

    #[test]
    fn push_after_close_is_rejected() {
        let q = JobQueue::new(4);
        q.close();
        assert!(q.push(job("1")).is_some(), "关闭后入队应被拒并归还任务");
    }

    #[test]
    fn consumer_blocks_then_wakes_on_push() {
        let q = Arc::new(JobQueue::new(4));
        let q2 = Arc::clone(&q);

        let h = std::thread::spawn(move || q2.pop_blocking().map(|j| j.segment_id));

        std::thread::sleep(Duration::from_millis(50));
        q.push(job("late"));

        assert_eq!(h.join().unwrap().as_deref(), Some("late"));
    }

    #[test]
    fn close_wakes_a_blocked_consumer() {
        let q = Arc::new(JobQueue::new(4));
        let q2 = Arc::clone(&q);

        let h = std::thread::spawn(move || q2.pop_blocking());

        std::thread::sleep(Duration::from_millis(50));
        q.close();

        assert!(h.join().unwrap().is_none());
    }

    #[test]
    fn cancellation_returns_jobs_and_prevents_further_consumption() {
        let queue = JobQueue::new(4);
        queue.push(job("first"));
        queue.push(job("second"));
        let cancelled = queue.close_and_drain();
        assert_eq!(cancelled, vec![job("first"), job("second")]);
        assert!(queue.pop_blocking().is_none());
        assert_eq!(queue.push(job("late")), Some(job("late")));
    }
}
