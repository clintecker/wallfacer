//! Remote control via Unix socket
//!
//! Accepts commands over a Unix socket to control the application
//! as if keyboard keys were pressed.

use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const SOCKET_PATH: &str = "/tmp/wallfacer.sock";

/// Commands that can be sent over the socket
#[derive(Debug, Clone)]
pub enum Command {
    Left,
    Right,
    Tab,
    ToggleFps,
    Save,
    Load,
    Quit,
    Effect(usize),
}

/// Controller that listens for commands on a Unix socket
pub struct Controller {
    receiver: Receiver<Command>,
    _listener_thread: thread::JoinHandle<()>,
}

impl Controller {
    /// Create a new controller listening on the Unix socket
    pub fn new() -> Result<Self, String> {
        // Remove existing socket if present
        let _ = std::fs::remove_file(SOCKET_PATH);

        let listener = UnixListener::bind(SOCKET_PATH)
            .map_err(|e| format!("Failed to bind socket: {}", e))?;

        // Set non-blocking so we can check for new connections
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {}", e))?;

        let (sender, receiver) = mpsc::channel();

        let handle = thread::spawn(move || {
            Self::listener_loop(listener, sender);
        });

        Ok(Self {
            receiver,
            _listener_thread: handle,
        })
    }

    fn listener_loop(listener: UnixListener, sender: Sender<Command>) {
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let sender = sender.clone();
                    thread::spawn(move || {
                        Self::handle_client(stream, sender);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection ready, sleep briefly
                    thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => {
                    // Socket closed or error, exit loop
                    break;
                }
            }
        }
    }

    fn handle_client(stream: UnixStream, sender: Sender<Command>) {
        let reader = BufReader::new(stream);
        for line in reader.lines().flatten() {
            if let Some(cmd) = Self::parse_command(&line) {
                if sender.send(cmd).is_err() {
                    break;
                }
            }
        }
    }

    fn parse_command(line: &str) -> Option<Command> {
        let line = line.trim().to_lowercase();
        match line.as_str() {
            "left" | "prev" => Some(Command::Left),
            "right" | "next" => Some(Command::Right),
            "tab" | "calibrate" => Some(Command::Tab),
            "f" | "fps" => Some(Command::ToggleFps),
            "s" | "save" => Some(Command::Save),
            "l" | "load" => Some(Command::Load),
            "q" | "quit" | "exit" => Some(Command::Quit),
            _ => {
                // Try to parse "effect N" or just a number
                if let Some(rest) = line.strip_prefix("effect ") {
                    rest.trim().parse().ok().map(Command::Effect)
                } else {
                    line.parse().ok().map(Command::Effect)
                }
            }
        }
    }

    /// Get any pending commands (non-blocking)
    pub fn poll(&self) -> Vec<Command> {
        let mut commands = Vec::new();
        while let Ok(cmd) = self.receiver.try_recv() {
            commands.push(cmd);
        }
        commands
    }

    /// Get the socket path
    pub fn socket_path() -> &'static str {
        SOCKET_PATH
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // Clean up the socket file
        let _ = std::fs::remove_file(SOCKET_PATH);
    }
}
