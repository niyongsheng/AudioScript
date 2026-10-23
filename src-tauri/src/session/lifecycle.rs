#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Status {
    #[default]
    Ready,
    Capturing,
    Error,
}

pub(super) enum Failure<T> {
    Stale,
    Accepted(Option<T>),
}

pub(super) struct Lifecycle<T> {
    next_epoch: u64,
    active_epoch: Option<u64>,
    initializing_epoch: Option<u64>,
    running: Option<T>,
    status: Status,
    error: Option<String>,
}

impl<T> Default for Lifecycle<T> {
    fn default() -> Self {
        Self {
            next_epoch: 0,
            active_epoch: None,
            initializing_epoch: None,
            running: None,
            status: Status::Ready,
            error: None,
        }
    }
}

impl<T> Lifecycle<T> {
    pub(super) fn begin(&mut self) -> Result<u64, &'static str> {
        if self.active_epoch.is_some() || self.initializing_epoch.is_some() {
            return Err("会话正在启动或采集中");
        }
        self.next_epoch += 1;
        self.active_epoch = Some(self.next_epoch);
        self.initializing_epoch = self.active_epoch;
        self.status = Status::Ready;
        self.error = None;
        Ok(self.next_epoch)
    }

    pub(super) fn publish(&mut self, epoch: u64, running: T) -> Result<(), T> {
        if self.active_epoch != Some(epoch) {
            return Err(running);
        }
        self.running = Some(running);
        self.initializing_epoch = None;
        self.status = Status::Capturing;
        Ok(())
    }

    pub(super) fn fail(&mut self, epoch: u64, error: String) -> Failure<T> {
        if self.active_epoch != Some(epoch) {
            return Failure::Stale;
        }
        self.active_epoch = None;
        self.status = Status::Error;
        self.error = Some(error);
        Failure::Accepted(self.running.take())
    }

    pub(super) fn abort_start(&mut self, epoch: u64) {
        if self.initializing_epoch == Some(epoch) {
            self.initializing_epoch = None;
        }
        if self.active_epoch == Some(epoch) && self.running.is_none() {
            self.active_epoch = None;
        }
    }

    pub(super) fn stop(&mut self) -> Option<T> {
        self.active_epoch = None;
        self.status = Status::Ready;
        self.error = None;
        self.running.take()
    }

    pub(super) fn status(&self) -> Status {
        self.status
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(super) fn running(&self) -> Option<&T> {
        self.running.as_ref()
    }

    pub(super) fn ensure_clearable(&self, teardown_active: bool) -> Result<(), &'static str> {
        if self.initializing_epoch.is_some() {
            return Err("会话仍在启动，请等待启动结束后再清空");
        }
        if self.active_epoch.is_some() {
            return Err("请先停止转写，再清空记录");
        }
        if teardown_active {
            return Err("会话仍在收尾，请稍后再清空");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_start_can_be_retried() {
        let mut state = Lifecycle::<&str>::default();
        let failed = state.begin().unwrap();
        state.abort_start(failed);
        let retry = state.begin().unwrap();
        state.publish(retry, "重试资源").unwrap();
        assert_eq!(state.running(), Some(&"重试资源"));
        assert_eq!(state.status(), Status::Capturing);
    }

    #[test]
    fn runtime_failure_returns_resources_once_and_allows_retry() {
        let mut state = Lifecycle::default();
        let epoch = state.begin().unwrap();
        state.publish(epoch, "音频与推理资源").unwrap();
        assert!(matches!(
            state.fail(epoch, "设备已断开".into()),
            Failure::Accepted(Some("音频与推理资源"))
        ));
        assert_eq!(state.status(), Status::Error);
        assert_eq!(state.error(), Some("设备已断开"));
        assert!(state.running().is_none());
        assert!(matches!(
            state.fail(epoch, "重复故障".into()),
            Failure::Stale
        ));

        let retry = state.begin().unwrap();
        state.publish(retry, "新设备").unwrap();
        assert_eq!(state.error(), None);
        assert_eq!(state.status(), Status::Capturing);
    }

    #[test]
    fn early_failure_cannot_be_overwritten_by_capture_publication() {
        let mut state = Lifecycle::default();
        let epoch = state.begin().unwrap();
        assert!(matches!(
            state.fail(epoch, "启动后立即断开".into()),
            Failure::Accepted(None)
        ));
        assert_eq!(
            state.publish(epoch, "必须释放的资源"),
            Err("必须释放的资源")
        );
        assert_eq!(state.status(), Status::Error);
    }

    #[test]
    fn old_failure_cannot_stop_a_new_session() {
        let mut state = Lifecycle::default();
        let old = state.begin().unwrap();
        state.publish(old, "旧设备").unwrap();
        assert_eq!(state.stop(), Some("旧设备"));
        let new = state.begin().unwrap();
        state.publish(new, "新设备").unwrap();
        assert!(matches!(state.fail(old, "迟到故障".into()), Failure::Stale));
        assert_eq!(state.running(), Some(&"新设备"));
        assert_eq!(state.status(), Status::Capturing);
    }

    #[test]
    fn stop_during_start_rejects_late_resources() {
        let mut state = Lifecycle::default();
        let epoch = state.begin().unwrap();
        assert!(state.stop().is_none());
        assert_eq!(state.publish(epoch, "已取消的资源"), Err("已取消的资源"));
        assert_eq!(state.status(), Status::Ready);
        assert!(state.begin().is_err());
        assert!(state.ensure_clearable(false).is_err());
        state.abort_start(epoch);
        assert!(state.ensure_clearable(false).is_ok());
        assert!(state.begin().is_ok());
    }

    #[test]
    fn clear_guard_rejects_capturing_and_stopping_sessions() {
        let mut state = Lifecycle::default();
        let epoch = state.begin().unwrap();
        state.publish(epoch, "采集资源").unwrap();
        assert!(state.ensure_clearable(false).is_err());
        state.stop();
        assert!(state.ensure_clearable(true).is_err());
        assert!(state.ensure_clearable(false).is_ok());
    }
}
