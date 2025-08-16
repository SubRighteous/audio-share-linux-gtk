use get_if_addrs::get_if_addrs;

use std::io::{BufRead, BufReader};

use std::process::{Child, Command, Stdio};

use std::sync::{Arc, Mutex};

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
                println!("Skipping line: {:?}", line); // Debug
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

fn get_firewalld_rule_exists(address: &str, port: &u16) -> bool {
    // Try running firewall-cmd, exit early if it's not installed
    let output = Command::new("firewall-cmd")
        .arg("--list-rich-rules")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return true, // If firewall-cmd doesn't exist, assume OK
    };

    let rules = String::from_utf8_lossy(&output.stdout);

    let tcp_rule = format!("destination address=\"{}\" port port=\"{}\" protocol=\"tcp\"", address, port);
    let udp_rule = format!("destination address=\"{}\" port port=\"{}\" protocol=\"udp\"", address, port);

    rules.contains(&tcp_rule) && rules.contains(&udp_rule)
}

fn get_ufw_rule_exists(address: &str, port: &u16) -> bool {
    let output = std::process::Command::new("ufw")
        .arg("status")
        .arg("numbered")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return true, // Assume OK if ufw is not installed
    };

    let rules = String::from_utf8_lossy(&output.stdout);
    let tcp_rule = format!("{}:{} ALLOW", address, port);
    let udp_rule = format!("{}:{} ALLOW", address, port);

    rules.contains(&tcp_rule) && rules.contains(&udp_rule)
}

fn firewall_allows(address: &str, port: &u16) -> bool {
    get_firewalld_rule_exists(address, port) && get_ufw_rule_exists(address, port)
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

         if firewall_allows(&server_ip, &server_port) == false {
            let stop_notifier = self.process_stop_notifier.clone();

            let reason = ProcessStopReason::FirewallBlocked;
            let _ = stop_notifier.send(Some(reason));

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
                                    //println!("IP detected: {}", ip);
                                    let _ = device_connected_notifier.send((ip.to_string(), true));
                                }
                            }

                        }

                        if line.contains("[info] close"){
                            // Split by spaces and take the last part
                            if let Some(last) = line.split_whitespace().last() {
                                // Split by ':' to separate IP and port
                                if let Some((ip, _port)) = last.split_once(':') {
                                    //println!("Close IP detected: {}", ip);
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
