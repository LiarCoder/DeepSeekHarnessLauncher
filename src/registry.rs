use serde::Deserialize;
use std::ptr::null_mut;
use windows::core::PCWSTR;
use windows::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

const REGISTRY_HOST: &str = "registry.npmjs.org";
const REGISTRY_PATH: &str = "/@deepseek-ai%2Fdsh/latest";
const GITHUB_API_HOST: &str = "api.github.com";
const GITHUB_LATEST_RELEASE_PATH: &str = "/repos/LiarCoder/DeepSeekHarnessLauncher/releases/latest";

#[derive(Deserialize)]
struct LatestPackage {
    version: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LauncherRelease {
    pub version: String,
    pub page_url: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
}

pub fn latest_version() -> Result<String, String> {
    let body = get(REGISTRY_HOST, REGISTRY_PATH)?;
    let package: LatestPackage = serde_json::from_slice(&body)
        .map_err(|error| format!("解析 npm registry 响应失败：{error}"))?;
    let version = package.version.trim();
    if version.is_empty() {
        return Err("npm registry 未返回 dsh 版本号".to_owned());
    }
    Ok(version.to_owned())
}

pub fn latest_launcher_release() -> Result<LauncherRelease, String> {
    let body = get(GITHUB_API_HOST, GITHUB_LATEST_RELEASE_PATH)?;
    parse_latest_launcher_release(&body)
}

fn parse_latest_launcher_release(body: &[u8]) -> Result<LauncherRelease, String> {
    let release: LatestRelease = serde_json::from_slice(body)
        .map_err(|error| format!("解析 GitHub Release 响应失败：{error}"))?;
    let version = release.tag_name.trim();
    if version.is_empty() {
        return Err("GitHub Release 未返回 Launcher 版本号".to_owned());
    }
    let page_url = release.html_url.trim();
    if page_url.is_empty() {
        return Err("GitHub Release 未返回发布页地址".to_owned());
    }
    Ok(LauncherRelease {
        version: version.to_owned(),
        page_url: page_url.to_owned(),
    })
}

fn get(host: &str, path: &str) -> Result<Vec<u8>, String> {
    let agent = wide(&format!(
        "DeepSeekHarnessLauncher/{}",
        env!("CARGO_PKG_VERSION")
    ));
    let host = wide(host);
    let path = wide(path);

    unsafe {
        let session = WinHttpOpen(
            PCWSTR(agent.as_ptr()),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            return Err("无法初始化 Windows HTTP 会话".to_owned());
        }

        let result = get_with_session(session, &host, &path);
        let _ = WinHttpCloseHandle(session);
        result
    }
}

unsafe fn get_with_session(
    session: *mut core::ffi::c_void,
    host: &[u16],
    path: &[u16],
) -> Result<Vec<u8>, String> {
    WinHttpSetTimeouts(session, 5_000, 5_000, 10_000, 15_000)
        .map_err(|error| format!("设置更新请求超时失败：{error}"))?;

    let connection = WinHttpConnect(session, PCWSTR(host.as_ptr()), 443, 0);
    if connection.is_null() {
        return Err("无法连接更新服务".to_owned());
    }

    let request = WinHttpOpenRequest(
        connection,
        PCWSTR::from_raw(wide("GET").as_ptr()),
        PCWSTR(path.as_ptr()),
        PCWSTR::null(),
        PCWSTR::null(),
        null_mut(),
        WINHTTP_FLAG_SECURE,
    );
    if request.is_null() {
        let _ = WinHttpCloseHandle(connection);
        return Err("无法创建更新请求".to_owned());
    }

    let result = receive_response(request);
    let _ = WinHttpCloseHandle(request);
    let _ = WinHttpCloseHandle(connection);
    result
}

unsafe fn receive_response(request: *mut core::ffi::c_void) -> Result<Vec<u8>, String> {
    WinHttpSendRequest(request, None, None, 0, 0, 0)
        .map_err(|error| format!("发送更新请求失败：{error}"))?;
    WinHttpReceiveResponse(request, null_mut())
        .map_err(|error| format!("接收更新响应失败：{error}"))?;

    let mut status_code = 0u32;
    let mut status_size = core::mem::size_of::<u32>() as u32;
    let mut header_index = 0u32;
    WinHttpQueryHeaders(
        request,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        PCWSTR::null(),
        Some((&mut status_code as *mut u32).cast()),
        &mut status_size,
        &mut header_index,
    )
    .map_err(|error| format!("读取更新服务状态失败：{error}"))?;
    if !(200..300).contains(&status_code) {
        return Err(format!("更新服务返回 HTTP {status_code}"));
    }

    let mut body = Vec::new();
    loop {
        let mut available = 0u32;
        WinHttpQueryDataAvailable(request, &mut available)
            .map_err(|error| format!("读取更新响应长度失败：{error}"))?;
        if available == 0 {
            break;
        }

        let previous_len = body.len();
        body.resize(previous_len + available as usize, 0);
        let mut read = 0u32;
        WinHttpReadData(
            request,
            body[previous_len..].as_mut_ptr().cast(),
            available,
            &mut read,
        )
        .map_err(|error| format!("读取更新响应失败：{error}"))?;
        body.truncate(previous_len + read as usize);
        if read == 0 {
            break;
        }
    }

    Ok(body)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::parse_latest_launcher_release;

    #[test]
    fn parses_github_latest_launcher_release() {
        let release = parse_latest_launcher_release(
            br#"{"tag_name":"v0.3.0","html_url":"https://github.com/LiarCoder/DeepSeekHarnessLauncher/releases/tag/v0.3.0"}"#,
        )
        .expect("parse release");

        assert_eq!(release.version, "v0.3.0");
        assert_eq!(
            release.page_url,
            "https://github.com/LiarCoder/DeepSeekHarnessLauncher/releases/tag/v0.3.0"
        );
    }

    #[test]
    fn rejects_release_without_version() {
        let result = parse_latest_launcher_release(
            br#"{"tag_name":"  ","html_url":"https://github.com/LiarCoder/DeepSeekHarnessLauncher/releases/latest"}"#,
        );

        assert_eq!(
            result.expect_err("missing version should fail"),
            "GitHub Release 未返回 Launcher 版本号"
        );
    }
}
