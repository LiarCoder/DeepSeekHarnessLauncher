use crate::command::{build_command, taskkill};
use crate::dsh;
use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum ProcessEvent {
    Output(String),
    UnexpectedExit,
}

struct ProcessState {
    process_id: Option<u32>,
    web_ui_url: Option<String>,
    running: bool,
    ready: bool,
    stopping: bool,
}

pub struct HarnessProcessManager {
    state: Arc<Mutex<ProcessState>>,
    running: Arc<AtomicBool>,
    events: Sender<ProcessEvent>,
}

impl HarnessProcessManager {
    pub fn new(events: Sender<ProcessEvent>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ProcessState {
                process_id: None,
                web_ui_url: None,
                running: false,
                ready: false,
                stopping: false,
            })),
            running: Arc::new(AtomicBool::new(false)),
            events,
        }
    }

    pub fn start_and_wait(&self, timeout: Duration) -> Result<(u32, String), String> {
        if let Some(url) = self.web_ui_url() {
            return self
                .process_id()
                .map(|process_id| (process_id, url))
                .ok_or_else(|| "Harness 进程状态不完整".to_owned());
        }

        let port = find_available_port()?;
        let dsh_command = dsh::locate()?;
        let args = vec!["web".to_owned(), "--port".to_owned(), port.to_string()];
        let mut child = build_command(&dsh_command, &args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("启动 dsh 失败：{error}"))?;

        let process_id = child.id();
        let web_ui_url = format!("http://127.0.0.1:{port}/");
        self.running.store(true, Ordering::SeqCst);
        if let Ok(mut state) = self.state.lock() {
            state.process_id = Some(process_id);
            state.web_ui_url = Some(web_ui_url.clone());
            state.running = true;
            state.ready = false;
            state.stopping = false;
        }

        spawn_output_reader(child.stdout.take(), self.events.clone());
        spawn_output_reader(child.stderr.take(), self.events.clone());
        self.spawn_exit_monitor(child);

        let started_at = Instant::now();
        while started_at.elapsed() < timeout {
            if !self.running.load(Ordering::SeqCst) {
                return Err("dsh 在服务就绪前退出".to_owned());
            }
            if TcpStream::connect(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).is_ok() {
                if let Ok(mut state) = self.state.lock() {
                    state.ready = true;
                }
                return Ok((process_id, web_ui_url));
            }
            thread::sleep(Duration::from_millis(150));
        }

        self.stop();
        Err("等待 dsh 启动超时".to_owned())
    }

    pub fn stop(&self) {
        let process_id = if let Ok(mut state) = self.state.lock() {
            state.stopping = true;
            state.ready = false;
            state.process_id
        } else {
            None
        };

        if let Some(process_id) = process_id {
            taskkill(process_id);
        }

        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut state) = self.state.lock() {
            state.running = false;
            state.ready = false;
            state.process_id = None;
            state.web_ui_url = None;
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn process_id(&self) -> Option<u32> {
        self.state.lock().ok()?.process_id
    }

    pub fn web_ui_url(&self) -> Option<String> {
        self.state.lock().ok()?.web_ui_url.clone()
    }

    fn spawn_exit_monitor(&self, mut child: std::process::Child) {
        let state = Arc::clone(&self.state);
        let running = Arc::clone(&self.running);
        let events = self.events.clone();
        thread::spawn(move || {
            let _ = child.wait();
            running.store(false, Ordering::SeqCst);
            let unexpectedly_exited = if let Ok(mut state) = state.lock() {
                let unexpectedly_exited = !state.stopping && state.ready;
                state.running = false;
                state.ready = false;
                state.process_id = None;
                state.web_ui_url = None;
                unexpectedly_exited
            } else {
                false
            };
            if unexpectedly_exited {
                let _ = events.send(ProcessEvent::UnexpectedExit);
            }
        });
    }
}

impl Drop for HarnessProcessManager {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_output_reader<R>(stream: Option<R>, events: Sender<ProcessEvent>)
where
    R: std::io::Read + Send + 'static,
{
    let Some(stream) = stream else {
        return;
    };
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if !line.is_empty() {
                let _ = events.send(ProcessEvent::Output(line));
            }
        }
    });
}

fn find_available_port() -> Result<u16, String> {
    if can_bind(3080) {
        return Ok(3080);
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("寻找可用端口失败：{error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("读取可用端口失败：{error}"))
}

fn can_bind(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
}
