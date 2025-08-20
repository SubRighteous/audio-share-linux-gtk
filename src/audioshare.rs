use get_if_addrs::get_if_addrs;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tokio::sync::broadcast;

pub fn get_local_ipv4() -> String {
    if let Ok(interfaces) = get_if_addrs() {
        for iface in interfaces {
            // skip loopback and non-IPv4 addresses
            if !iface.is_loopback() {
                if let std::net::IpAddr::V4(ipv4) = iface.ip() {
                    return ipv4.to_string();
                }
            }
        }
    }
    "8.8.8.8".to_string()
}

pub fn get_audio_endpoints() -> Vec<(bool, u16, String)> {
    let output = Command::new("/app/bin/as-cmd")
        .arg("--list-endpoint")
        .output()
        .expect("Failed to launch as-cmd");

    let command_output = String::from_utf8_lossy(&output.stdout);

    command_output
        .lines()
        .filter(|line| !line.is_empty() && *line != "endpoint list:")
        .filter_map(|line| {
            // Check if the line starts with '*' (after trimming leading whitespace)
            let trimmed = line.trim_start();
            let is_default = trimmed.starts_with('*');

            // Remove the '*' so we can parse the rest cleanly
            let clean_line = if is_default {
                trimmed.trim_start_matches("*").trim_start()
            } else {
                trimmed
            };

            // Use regex-free string splitting
            let id_part = clean_line.split("id:").nth(1)?;
            let name_part = id_part.split("name:").collect::<Vec<&str>>();

            if name_part.len() != 2 {
                return None;
            }

            let id_str = name_part[0].trim();
            let name_str = name_part[1].trim();

            let id: u16 = id_str.parse().ok()?;
            Some((is_default, id, name_str.to_string()))
        })
        .collect()
}

pub fn get_default_endpoint() -> Option<(bool, u16, String)> {
    get_audio_endpoints()
        .into_iter()
        .find(|(is_default, _, _)| *is_default)

    // Example of usage
    // if let Some((_, id, name)) = get_default_endpoint(input) {
    //     println!("Default endpoint -> id: {}, name: {}", id, name);
    // } else {
    //     println!("No default endpoint found");
    // }
}

pub fn get_endpoint_id(_name: &String) -> Option<u32> {
    get_audio_endpoints()
        .into_iter()
        .find(|(_, _, name)| name == _name)
        .map(|(_, id, _)| id as u32)
}

pub fn get_encoding_key(_name: &String) -> Option<String> {
    get_audio_encoding()
        .into_iter()
        .find(|(_, desc)| desc == _name)
        .map(|(key, _)| key as String)
}

pub fn get_default_encoding() -> Option<(String, String)> {
    get_audio_encoding()
        .into_iter()
        .find(|(name, _)| name == "default")
}

pub fn get_endpoint_position_in_dropdown(_name: &String) -> u32 {
    get_audio_endpoints()
        .iter()
        .position(|&(_flag, _id, ref name)| name == _name)
        .map(|idx| idx as u32)
        .expect("Couldn't find endpoint in Vec")
}

pub fn get_encoding_position_in_dropdown(_name: &String) -> u32 {
    get_audio_encoding()
        .iter()
        .position(|&(_, ref name)| name == _name)
        .map(|idx| idx as u32)
        .expect("Couldn't find encoding in Vec")
}

pub fn get_audio_encoding() -> Vec<(String, String)> {
    let output = Command::new("/app/bin/as-cmd")
        .arg("--list-encoding")
        .output()
        .expect("Failed to launch as-cmd");

    let command_output = String::from_utf8_lossy(&output.stdout);

    command_output
        .lines()
        .map(str::trim) // remove leading/trailing whitespace first
        .filter(|line| !line.is_empty() && *line != "encoding list:")
        .filter_map(|line| {
            let mut parts = line.splitn(2, char::is_whitespace);
            let key = parts.next()?.trim();
            let value = parts.next()?.trim();

            if key.is_empty() || value.is_empty() {
                println!("Skipping line: {:?}", line);
                None
            } else {
                Some((key.to_string(), value.to_string()))
            }
        })
        .collect()
}

pub fn get_version() {
    let output = Command::new("/app/bin/as-cmd")
        .arg("--version")
        .output()
        .expect("Failed to launch as-cmd");

    println!("\nTesting as-cmd\n{}", "----------");
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("{}", "----------");
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    println!("\n{}\n", "----------");
}

#[derive(Debug)]
pub struct FirewallTestThread {
    pub server_child: Arc<Mutex<Option<TcpListener>>>,
    pub running: Arc<Mutex<bool>>,
    pub result_notifier: broadcast::Sender<bool>,
}

impl FirewallTestThread{
    pub fn new() -> Self {
        let (device_tx, _rx) = broadcast::channel::<bool>(16);
        Self {
            server_child: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(false)),
            result_notifier: device_tx,
        }
    }

    pub fn subscribe_result_event(&self) -> broadcast::Receiver<bool>{
        self.result_notifier.subscribe()
    }

    pub fn start(&self,server_ip: String, server_port: u16,){
        let server_child = self.server_child.clone();
        {
            let guard = server_child.lock().unwrap();
            if guard.is_some() {
                eprintln!("Test already running");
                return;
            }
        }
        let running_guard = self.running.clone();

        let result_notifier = self.result_notifier.clone();

        {
            // check if already running
            let guard = server_child.lock().unwrap();
            if guard.is_some() || *running_guard.lock().unwrap() {
                eprint!("Test already running");
                return;
            }
        }

        *running_guard.lock().unwrap() = true;

        std::thread::spawn(move || {
            let addr = format!("{}:{}", server_ip, server_port);

            let _result = match TcpListener::bind(&addr) {
                Ok(listener) => {
                    listener.set_nonblocking(true).unwrap();
                    let start = Instant::now();
                    let timeout = Duration::from_secs(9);

                    let _guard = server_child.lock().unwrap();
                    //*guard = Some(listener);

                    let mut success = false;
                    while start.elapsed() < timeout && *running_guard.lock().unwrap(){
                        if let Ok((_socket, _addr)) = listener.accept() {
                            success = true;
                            break;
                        }
                        thread::sleep(Duration::from_millis(50)); // avoid busy loop
                    }

                    // Only notify if the system timer went out
                    if *running_guard.lock().unwrap(){
                        let _ = result_notifier.send(success);
                    }

                    success
                }
                Err(_) => {
                    let _ = result_notifier.send(false);
                    false // failed to bind
                }
            };



        });
    }

    pub fn stop(&self) {
       // Set running to false first so the loop sees it
        *self.running.lock().unwrap() = false;

        // Take the listener out of the Arc<Mutex<>> so the loop can’t access it anymore
        self.server_child.lock().unwrap().take();

        println!("Firewall test stopped");

        //*guard = None;
        //*running_guard = false;
        //println!("Testing is stopped");
    }

    pub fn is_running(&self) -> bool {
       *self.running.lock().unwrap()
    }
}

// A message to send when the process stops
#[derive(Debug , Clone, PartialEq, Eq)]
pub enum ProcessStopReason {
    InvalidBinding,
    InvalidArgument,
    FirewallBlocked,
    ExitedSuccessfully,
    Resetting,
    ExitedWithError(Option<i32>),
    FailedToKill,
}


// AudioShare Thread
#[derive(Debug)]
pub struct AudioShareServerThread {
    pub server_child: Arc<Mutex<Option<Child>>>,
    pub running: Arc<Mutex<bool>>,
    pub process_stop_notifier: watch::Sender<Option<ProcessStopReason>>,
    pub device_connected_notifier: broadcast::Sender<(String, bool)>,
}

impl AudioShareServerThread {
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(None);
        let (device_tx, _rx) = broadcast::channel::<(String, bool)>(16);
        Self {
            server_child: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(false)),
            process_stop_notifier: tx,
            device_connected_notifier: device_tx,
        }
    }

    pub fn subscribe_stop_event(&self) -> watch::Receiver<Option<ProcessStopReason>> {
        self.process_stop_notifier.subscribe()
    }

    pub fn subscribe_device_event(&self) -> broadcast::Receiver<(String, bool)>{
        self.device_connected_notifier.subscribe()
    }

    pub fn start(
        &self,
        server_ip: String,
        server_port: u16,
        endpoint_id: u32,
        encoding_key: String,
    ) {
        let mut guard = self.server_child.lock().unwrap();
        let mut running_guard = self.running.lock().unwrap();

        if *running_guard {
            eprint!("Command already running");
            return;
        }

        if guard.is_some() {
            eprintln!("Command already running");
            return;
        }

        println!("Starting server thread with server ip : {server_ip} server port : {server_port} endpoint ID: {endpoint_id}, encoding key: {encoding_key}");

        let binding_arg: String = format!("--bind={}:{}", &server_ip, &server_port.to_string());
        println!("{}", &binding_arg.to_string());

        // Build the command using passed-in variables
        let cmd = Command::new("/app/bin/as-cmd")
            .arg(binding_arg)
            .arg("-e")
            .arg(&endpoint_id.to_string())
            .arg("--encoding")
            .arg(&encoding_key)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match cmd {
            Ok(mut child) => {
                // Spawn a new thread to read the child process's stdout
                let child_stdout = child.stderr.take().unwrap();
                let child_stdinfo = child.stdout.take().unwrap();
                let child_id = self.server_child.clone();
                let running_flag = self.running.clone();
                let running_flag_stdout = self.running.clone();

                let stop_notifier = self.process_stop_notifier.clone();
                let device_connected_notifier = self.device_connected_notifier.clone();

                *guard = Some(child);

                // Thread for stdout
                std::thread::spawn(move || {
                    let reader = BufReader::new(child_stdinfo);
                    for line in reader.lines().flatten() {
                        println!("[AS-CMD Out]: {}", line);
                        if line.contains("[info] accept"){
                            // Split by spaces and take the last part
                            if let Some(last) = line.split_whitespace().last() {
                                // Split by ':' to separate IP and port
                                if let Some((ip, _port)) = last.split_once(':') {
                                    let _ = device_connected_notifier.send((ip.to_string(), true));
                                }
                            }

                        }

                        if line.contains("[info] close"){
                            // Split by spaces and take the last part
                            if let Some(last) = line.split_whitespace().last() {
                                // Split by ':' to separate IP and port
                                if let Some((ip, _port)) = last.split_once(':') {
                                    let _ = device_connected_notifier.send((ip.to_string(), false));
                                }
                            }

                        }
                    }
                    *running_flag_stdout.lock().unwrap() = false;
                });

                // Thread of stderror
                std::thread::spawn(move || {
                    let reader = BufReader::new(child_stdout);

                    let mut reason = ProcessStopReason::ExitedSuccessfully;

                    for line in reader.lines().flatten() {
                        println!("[AS-CMD Error]: {}", line);
                        // Check for specific logs to stop the process
                        if line.contains("bind: Cannot assign requested address") {
                            println!("Detected 'Cannot assign requested address' log. Stopping child process...");

                            reason = ProcessStopReason::InvalidBinding;
                            break;

                        }
                        if line.contains("Invalid argument"){
                            reason = ProcessStopReason::InvalidArgument;
                            break;
                        }
                    }

                    let mut child_guard = child_id.lock().unwrap();
                    if let Some(c) = child_guard.as_mut() {
                        if let Err(e) = c.kill() {
                            eprintln!("Failed to kill child process: {}", e);
                        }
                    }

                    *child_guard = None;
                    *running_flag.lock().unwrap() = false;
                    let _ = stop_notifier.send(Some(reason));
                });

                *running_guard = true;
                println!("Command started");
            }
            Err(e) => {
                eprintln!("Failed to start command: {}", e);
                *running_guard = false;
            }
        }
    }

    pub fn stop(&self) {
        let mut guard = self.server_child.lock().unwrap();
        let mut running_guard = self.running.lock().unwrap();

        if let Some(server_child) = guard.as_mut() {
            match server_child.kill() {
                Ok(_) => println!("Process killed"),
                Err(e) => eprintln!("Failed to kill process: {}", e),
            }
        }

        *guard = None;
        *running_guard = false;
    }

    pub fn reset(&self){
        let mut guard = self.server_child.lock().unwrap();
        let mut running_guard = self.running.lock().unwrap();

        if let Some(server_child) = guard.as_mut() {
            match server_child.kill() {
                Ok(_) => println!("Process killed"),
                Err(e) => eprintln!("Failed to kill process: {}", e),
            }
        }

        let stop_notifier = self.process_stop_notifier.clone();

        let reason = ProcessStopReason::Resetting;

        let _ = stop_notifier.send(Some(reason));

        *guard = None;
        *running_guard = false;
    }

    pub fn is_running(&self) -> bool {

        *self.running.lock().unwrap()
    }
}
