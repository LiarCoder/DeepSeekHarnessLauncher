use windows::core::w;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject,
};

const MUTEX_NAME: windows::core::PCWSTR = w!("Local\\deepseek-harness-launcher.SingleInstance");
const SIGNAL_NAME: windows::core::PCWSTR = w!("Local\\deepseek-harness-launcher.OpenWebUi");

pub struct SingleInstance {
    mutex: HANDLE,
    open_web_ui_signal: HANDLE,
}

impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>, String> {
        unsafe {
            let mutex = CreateMutexW(None, true, MUTEX_NAME)
                .map_err(|error| format!("创建单实例互斥体失败：{error}"))?;
            let already_exists = GetLastError() == ERROR_ALREADY_EXISTS;
            let signal = match CreateEventW(None, false, false, SIGNAL_NAME) {
                Ok(signal) => signal,
                Err(error) => {
                    let _ = CloseHandle(mutex);
                    return Err(format!("创建打开 Web UI 信号失败：{error}"));
                }
            };

            if already_exists {
                let _ = SetEvent(signal);
                let _ = CloseHandle(signal);
                let _ = CloseHandle(mutex);
                return Ok(None);
            }

            Ok(Some(Self {
                mutex,
                open_web_ui_signal: signal,
            }))
        }
    }

    pub fn is_open_web_ui_requested(&self) -> bool {
        unsafe { WaitForSingleObject(self.open_web_ui_signal, 0) == WAIT_OBJECT_0 }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.open_web_ui_signal);
            let _ = CloseHandle(self.mutex);
        }
    }
}
