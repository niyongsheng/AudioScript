use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::{class, msg_send};
use objc2_foundation::{NSBundle, NSString};

use super::{MicPermission, MicRequestResult};

const PROMPT_TIMEOUT: Duration = Duration::from_secs(30);

const STATUS_NOT_DETERMINED: isize = 0;
const STATUS_RESTRICTED: isize = 1;
const STATUS_DENIED: isize = 2;
const STATUS_AUTHORIZED: isize = 3;

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: &'static NSString;
}

static REQUEST: Mutex<()> = Mutex::new(());

pub fn status() -> MicPermission {
    // SAFETY: AVMediaTypeAudio 是 AVFoundation 的不可变常量；authorizationStatusForMediaType: 是文档化的线程安全只读查询，不弹任何 UI。
    let raw: isize = unsafe {
        msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeAudio]
    };

    match raw {
        STATUS_AUTHORIZED => MicPermission::Granted,
        STATUS_DENIED => MicPermission::Denied,
        STATUS_RESTRICTED => MicPermission::Restricted,
        STATUS_NOT_DETERMINED => MicPermission::NotDetermined,
        _ => MicPermission::Unknown,
    }
}

pub fn has_usage_description(key: &'static str) -> bool {
    let key = NSString::from_str(key);
    let value = NSBundle::mainBundle().objectForInfoDictionaryKey(&key);
    value
        .and_then(|v| v.downcast::<NSString>().ok())
        .is_some_and(|v| !v.to_string().trim().is_empty())
}

pub fn request() -> Result<MicRequestResult, String> {
    let current = status();
    if current != MicPermission::NotDetermined {
        return Ok(MicRequestResult {
            state: current,
            timed_out: false,
        });
    }

    if !has_usage_description("NSMicrophoneUsageDescription") {
        return Err(crate::i18n::tr(crate::i18n::Key::PermissionMicUnbundled).into());
    }

    let _guard = REQUEST
        .try_lock()
        .map_err(|_| crate::i18n::tr(crate::i18n::Key::PermissionRequestInFlight).to_string())?;

    let (tx, rx) = mpsc::channel::<bool>();
    let block = RcBlock::new(move |granted: Bool| {
        let _ = tx.send(granted.as_bool());
    });

    // SAFETY: AVFoundation 会复制这个 block 并在自己的队列上回调；block 只捕获了一个 Send 的 mpsc::Sender，本线程此后不再触碰它。
    unsafe {
        let _: () = msg_send![
            class!(AVCaptureDevice),
            requestAccessForMediaType: AVMediaTypeAudio,
            completionHandler: &*block
        ];
    }

    let answered = rx.recv_timeout(PROMPT_TIMEOUT);
    drop(_guard);

    match answered {
        Ok(true) => Ok(MicRequestResult {
            state: MicPermission::Granted,
            timed_out: false,
        }),
        Ok(false) => Ok(MicRequestResult {
            state: MicPermission::Denied,
            timed_out: false,
        }),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(MicRequestResult {
            state: status(),
            timed_out: true,
        }),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(crate::i18n::tr(crate::i18n::Key::PermissionCallbackDisconnected).into())
        }
    }
}
