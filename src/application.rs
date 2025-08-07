/* application.rs
 *
 * Copyright 2025 Daniel Rys
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use once_cell::unsync::OnceCell;

use std::cell::{Cell, RefCell};


use crate::audioshare;
use crate::config::VERSION;
use crate::configfile::{get_config_path, load_or_create_config, save_config};
use crate::AudiosharegtkWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AudiosharegtkApplication {
        pub is_server_active: Cell<bool>,
        pub audio_share_server_thread: OnceCell<RefCell<audioshare::AudioShareServerThread>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudiosharegtkApplication {
        const NAME: &'static str = "AudiosharegtkApplication";
        type Type = super::AudiosharegtkApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for AudiosharegtkApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);

            self.audio_share_server_thread
                .set(RefCell::new(audioshare::AudioShareServerThread::new()))
                .expect("audio_share_server_thread already set");
        }
    }

    impl ApplicationImpl for AudiosharegtkApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let application = self.obj();
            // Get the current window or create one if necessary
            let window = application.active_window().unwrap_or_else(|| {
                let window = AudiosharegtkWindow::new(&*application);
                window.upcast()
            });

            let window_clone = window.downgrade(); // Avoid circular references

            window.connect_close_request(move |_win| {
                if let Some(window) = window_clone.upgrade() {
                    // Call custom "quit" action
                    if let Some(app) = window.application() {
                        app.activate_action("quit", None);
                    }

                    // Prevent default close behavior
                    return gtk::glib::Propagation::Stop;
                }

                return gtk::glib::Propagation::Proceed;
            });

            // Ask the window manager/compositor to present the window
            window.present();

            application.on_start_up();
        }
    }

    impl GtkApplicationImpl for AudiosharegtkApplication {}
    impl AdwApplicationImpl for AudiosharegtkApplication {}
}

glib::wrapper! {
    pub struct AudiosharegtkApplication(ObjectSubclass<imp::AudiosharegtkApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl AudiosharegtkApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property(
                "resource-base-path",
                "/com/subrighteous/audiosharegtk",
            )
            .build()
    }

    pub fn main_window(&self) -> Option<crate::window::AudiosharegtkWindow> {
        self.active_window()
            .and_then(|w| w.downcast::<crate::window::AudiosharegtkWindow>().ok())
    }

    pub fn is_server_active(&self) -> bool {
        self.imp().is_server_active.get()
    }

    pub fn set_server_active(&self, active: bool) {
        self.imp().is_server_active.set(active);
    }

    // Actions go here
    // Actions are functions templates can call and use
    fn setup_gactions(&self) {
        let force_quit_action = gio::ActionEntry::builder("force_quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.action_quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        let settings_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| app.show_settings())
            .build();
        let toggle_server_action = gio::ActionEntry::builder("toggle_server")
            .activate(move |app: &Self, _, _| app.action_toggle_server())
            .build();
        let reset_server_settings = gio::ActionEntry::builder("reset_server_settings")
            .activate(move |app: &Self, _, _| app.action_reset_server_settings())
            .build();
        self.add_action_entries([
            force_quit_action,
            quit_action,
            about_action,
            settings_action,
            toggle_server_action,
            reset_server_settings,
        ]);

        // Setup Keyboard Shortcuts
        self.set_accels_for_action("app.shortcuts", &["<Ctrl><Shift>question"]);
        self.set_accels_for_action("app.toggle_server", &["<Ctrl>E"]);
        self.set_accels_for_action("app.reset_server_settings", &["<Ctrl>R"]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("AudioShareGtk")
            .application_icon("com.subrighteous.audiosharegtk")
            .developer_name("Daniel Rys")
            .version(VERSION)
            .developers(vec!["Daniel Rys"])
            // Translators: Replace "translator-credits" with your name/username, and optionally an email or URL.
            .translator_credits(&gettext("translator-credits"))
            .copyright("© 2025 Daniel Rys")
            .license_type(gtk::License::Gpl30)
            .build();

        about.add_credit_section(Some("mkckr0/audio-share"), &["mkckr0", "Biswa96"]);

        about.add_acknowledgement_section(Some("mkckr0/audio-share"), &["mkckr0", "Biswa96"]);

        about.add_link(
            "AudioShareGtk Github Page",
            "https://github.com/subrighteous/audio-share-linux-gui/",
        );
        about.add_link(
            "mkckr0/audio-share Github Page",
            "https://github.com/mkckr0/audio-share/",
        );
        about.add_link("Configure Firewall Rules on Linux", "https://github.com/mkckr0/audio-share?tab=readme-ov-file#configure-firewall-rules-on-linux");
        about.add_link(
            "About Audio Formats",
            "https://github.com/mkckr0/audio-share?tab=readme-ov-file#about-audio-format",
        );
        about.add_link(
            "Extra Setups for \"No Audio Endpoint\"",
            "https://github.com/mkckr0/audio-share?tab=readme-ov-file#for-linux",
        );
        about.present(Some(&window));
    }

    fn action_quit(&self) {
        if let Some(win) = self.main_window() {
            if let Some(config_ref) = win.imp().config.get() {
                let mut config = config_ref.borrow_mut();
                if config.minimize_on_exit == true {
                    win.minimize();
                    println!("Minimizing Window");
                } else {

                    println!("Server state is {}" , self.is_server_active());
                    config.last_server_state = self.is_server_active();

                    // Save the settings
                    let _ = save_config(&config);

                    // Stop the server
                    self.imp()
                        .audio_share_server_thread
                        .get()
                        .unwrap()
                        .borrow()
                        .stop();

                    self.quit();
                }
            }
        }
    }

    fn show_settings(&self) {
        let builder = gtk::Builder::from_resource(
            "/com/subrighteous/audiosharegtk/preferences_dialog.ui",
        );

        let window = self.active_window().unwrap();
        let preferences: adw::Dialog = builder
            .object("preferences_dialog")
            .expect("Failed to get AwaDialog");

        let close_button: gtk::Button = builder
            .object("close_button")
            .expect("Failed to get close_button");

        let do_nothing_check_button: gtk::CheckButton = builder
            .object("DoNothing_CheckButton")
            .expect("Failed to get DoNothing_CheckButton");

        let start_server_check_button: gtk::CheckButton = builder
            .object("StartServer_CheckButton")
            .expect("Failed to get StartServer_CheckButton");

        let keep_last_state_check_button: gtk::CheckButton = builder
            .object("KeepLastState_CheckButton")
            .expect("Failed to get KeepLastState_CheckButton");

        let exit_checkbutton: gtk::CheckButton = builder
            .object("Exit_CheckButton")
            .expect("Failed to get Exit_CheckButton");

        let minimize_to_tray_checkbutton: gtk::CheckButton = builder
            .object("MinimizeToTray_CheckButton")
            .expect("Failed to get MinimizeToTray_CheckButton");

        if let Some(win) = self.main_window() {
            if let Some(config_ref) = win.imp().config.get() {
                let config = config_ref.borrow();

                // TODO : Replace with actual saved states settings
                do_nothing_check_button
                    .set_active(!(config.auto_start_server || config.keep_last_state));
                keep_last_state_check_button.set_active(config.keep_last_state);
                start_server_check_button.set_active(config.auto_start_server);

                exit_checkbutton.set_active(!(config.minimize_on_exit));
                minimize_to_tray_checkbutton.set_active(config.minimize_on_exit);
            }

            // Clone a strong reference to the window (so we can use it in the closure)
            let window_clone = win.clone();

            close_button.connect_clicked({
                let preferences = preferences.clone();
                move |_| {
                    // TODO: Save the settings picked from preferences dialog
                    if let Some(config_refcell) = window_clone.imp().config.get() {
                        let mut config = config_refcell.borrow_mut();

                        // Only update if the config is different then the ui
                        if config.minimize_on_exit != minimize_to_tray_checkbutton.is_active()
                            || config.keep_last_state != keep_last_state_check_button.is_active()
                            || config.auto_start_server != start_server_check_button.is_active()
                        {
                            config.minimize_on_exit = minimize_to_tray_checkbutton.is_active();
                            config.keep_last_state = keep_last_state_check_button.is_active();
                            config.auto_start_server = start_server_check_button.is_active();

                            let _ = save_config(&config);
                        }
                    } else {
                        println!("No config set yet.");
                    }

                    preferences.close();
                }
            });
        } else {
            // Make sure if all else fails we can still close the window
            close_button.connect_clicked({
                let preferences = preferences.clone();
                move |_| {
                    preferences.close();
                }
            });
        }

        preferences.present(Some(&window));
    }

    fn on_start_up(&self) {
        println!("On Start Up");

        if let Some(win) = self.main_window() {
            if let Ok(config_file) = load_or_create_config() {
                println!("Audio Endpoint : {:?}", config_file.audio_endpoint);
                println!("Audio Encoding : {:?}", config_file.audio_encoding);
                println!("Server IP : {:?}", config_file.server_ip);
                println!("Server Port : {:?}", config_file.server_port);
                println!("minimize_on_exit : {:?}", config_file.minimize_on_exit);
                println!("auto_start_server : {:?}", config_file.auto_start_server);
                println!("keep_last_state : {:?}", config_file.keep_last_state);
                println!("last_server_state : {:?}", config_file.last_server_state);
                println!("Configuration file Path : {:?}", get_config_path());

                win.imp()
                    .server_ip_entry
                    .set_placeholder_text(Some(&config_file.server_ip));
                win.imp()
                    .server_port_entry
                    .set_placeholder_text(Some(&config_file.server_port.to_string()));

                // Store the config values in win
                let _result = win.imp().config.set(RefCell::new(config_file)).unwrap();

            }

            let endpoint_names: Vec<(bool, u16, String)> = audioshare::get_audio_endpoints();
            let endpoint_names_vec: Vec<&str> = endpoint_names
                .iter()
                .map(|(_, _, names)| names.as_str())
                .collect();
            let endpoint_names_array: &[&str] = &endpoint_names_vec;

            // Create endpoint model
            let endpoint_string_list = gtk::StringList::new(&endpoint_names_array);
            let endpoint_model = endpoint_string_list.clone().upcast::<gio::ListModel>();

            let encodings: Vec<(String, String)> = audioshare::get_audio_encoding();
            let encoding_names_vec: Vec<&str> =
                encodings.iter().map(|(_, names)| names.as_str()).collect();
            let encoding_names_array: &[&str] = &encoding_names_vec;

            // Create endpoint model
            let endcoding_string_list = gtk::StringList::new(&encoding_names_array);
            let endcoding_model = endcoding_string_list.clone().upcast::<gio::ListModel>();

            win.imp()
                .audio_endpoint_dropdown
                .set_model(Some(&endpoint_model));
            win.imp()
                .audio_encoding_dropdown
                .set_model(Some(&endcoding_model));

            // Connect the "selected" signal to specific dropdown_change functions
            win.imp().audio_endpoint_dropdown.connect_notify_local(
                Some("selected"),
                glib::clone!(
                    #[strong(rename_to = app)]
                    self,
                    move |dropdown, _| {
                        // Get the selected index
                        let index = dropdown.selected();

                        // If the model is a StringList, get the selected string
                        if let Some(model) = dropdown
                            .model()
                            .and_then(|m| m.downcast::<gtk::StringList>().ok())
                        {
                            if let Some(item) = model.string(index) {
                                app.on_endpoint_dropdown_change(&item.to_string());
                            }
                        }
                        //self.on_endpoint_dropdown_change(index);
                    },
                ),
            );

            win.imp().audio_encoding_dropdown.connect_notify_local(
                Some("selected"),
                glib::clone!(
                    #[strong(rename_to = app)]
                    self,
                    move |dropdown, _| {
                        // Get the selected index
                        let index = dropdown.selected();

                        // If the model is a StringList, get the selected string
                        if let Some(model) = dropdown
                            .model()
                            .and_then(|m| m.downcast::<gtk::StringList>().ok())
                        {
                            if let Some(item) = model.string(index) {
                                app.on_encoding_dropdown_change(&item.to_string());
                            }
                        }
                        //self.on_endpoint_dropdown_change(index);
                    },
                ),
            );

            //
            if let Some(config_data) = win.imp().config.get() {
                let config = config_data.borrow(); // Get Ref<AppConfig>

                // Set the endpoint and encoding dropdowns to the proper value
                let endpoint_pos: u32 = audioshare::get_endpoint_position_in_dropdown(&config.audio_endpoint);
                println!("{} , {}" , endpoint_pos, &config.audio_endpoint);
                win.imp().audio_endpoint_dropdown.set_selected(endpoint_pos.into());

                let encoding_pos: u32 = audioshare::get_encoding_position_in_dropdown(&config.audio_encoding);
                println!("{} , {}" , encoding_pos , &config.audio_encoding);
                win.imp().audio_encoding_dropdown.set_selected(encoding_pos.into());


                if config.auto_start_server || (config.keep_last_state && config.last_server_state) {
                    self.action_toggle_server();
                }
            }
        }
    }

    fn action_stop_server(&self, reason : audioshare::ProcessStopReason){
        if self.is_server_active() == true {
            println!("Stopping the server");
            self.set_server_active(false);

            if let Some(win) = self.main_window() {
                win.imp().toggle_server.set_label("Start");
                win.imp().toggle_server.add_css_class("success");
                win.imp().toggle_server.remove_css_class("error");

                win.imp().server_ip_entry.set_editable(true);
                win.imp().server_port_entry.set_editable(true);

                win.imp().server_ip_entry.set_secondary_icon_name(None);
                win.imp().server_port_entry.set_secondary_icon_name(None);

                if reason == audioshare::ProcessStopReason::Resetting{
                    // Tell server to reset()
                    self.imp()
                        .audio_share_server_thread
                        .get()
                        .unwrap()
                        .borrow()
                        .reset();
                }
                else{
                    // Stop the server
                    self.imp()
                        .audio_share_server_thread
                        .get()
                        .unwrap()
                        .borrow()
                        .stop();
                }

            }
        }
    }

    // Toggle/Start Server
    fn action_toggle_server(&self) {
        if self.is_server_active() == true {
            println!("Stopping the server");
            self.set_server_active(false);

            if let Some(win) = self.main_window() {
                win.imp().toggle_server.set_label("Start");
                win.imp().toggle_server.add_css_class("success");
                win.imp().toggle_server.remove_css_class("error");

                win.imp().server_ip_entry.set_editable(true);
                win.imp().server_port_entry.set_editable(true);

                win.imp().server_ip_entry.set_secondary_icon_name(None);
                win.imp().server_port_entry.set_secondary_icon_name(None);

                // Stop the server
                self.imp()
                    .audio_share_server_thread
                    .get()
                    .unwrap()
                    .borrow()
                    .stop();
            }
        } else {
            println!("Starting the server");
            self.set_server_active(true);

            if let Some(win) = self.main_window() {
                win.imp().toggle_server.set_label("Stop");
                win.imp().toggle_server.remove_css_class("success");
                win.imp().toggle_server.add_css_class("error");

                win.imp().server_ip_entry.set_editable(false);
                win.imp().server_ip_entry.set_secondary_icon_name(Some("changes-prevent-symbolic"));
                win.imp().server_port_entry.set_editable(false);
                win.imp().server_port_entry.set_secondary_icon_name(Some("changes-prevent-symbolic"));

                if win.imp().server_ip_entry.text().is_empty() {
                    if let Some(placeholder) = win.imp().server_ip_entry.placeholder_text() {
                        win.imp().server_ip_entry.set_text(&placeholder);
                    }
                }

                if win.imp().server_port_entry.text().is_empty() {
                    if let Some(placeholder) = win.imp().server_port_entry.placeholder_text() {
                        win.imp().server_port_entry.set_text(&placeholder);
                    }
                }

                // Get the endpoint and encoding settings from the ui
                let endpoint_selected_name =
                    Self::get_selected_string_from_dropdown(&win.imp().audio_endpoint_dropdown);
                println!(
                    "{}",
                    endpoint_selected_name
                        .clone()
                        .expect("selected name is None")
                        .to_string()
                );

                let endpoint_id: u32 = audioshare::get_endpoint_id(
                    &endpoint_selected_name
                        .expect("selected name number is none")
                        .to_string(),
                )
                .expect("selected_names doesn't exist");
                println!("{}", endpoint_id);

                let encoding_selected_name = Self::get_selected_string_from_dropdown(&win.imp().audio_encoding_dropdown);
                println!(
                    "{}",
                    encoding_selected_name
                        .clone()
                        .expect("selected name is None")
                        .to_string()
                );

                let encoding_key: String = audioshare::get_encoding_key(
                    &encoding_selected_name
                        .expect("selected name number is none")
                        .to_string(),
                )
                .expect("selected_names doesn't exist");
                println!("{}", encoding_key);

                // Convert server_port string to u16
                let server_port_string = win.imp().server_port_entry.text().to_string();

                let server_port: u16 = server_port_string
                    .parse()
                    .expect("Failed to convert server port to u16");

                // Enroll the "on_server_error" function into the server stop_event
                let mut rx = self
                    .imp()
                    .audio_share_server_thread
                    .get()
                    .expect("AudioShareServerThread not initialized")
                    .borrow()
                    .subscribe_stop_event();

                // Start Server
                self.imp()
                    .audio_share_server_thread
                    .get()
                    .unwrap()
                    .borrow()
                    .start(
                        win.imp().server_ip_entry.text().to_string(),
                        server_port,
                        endpoint_id,
                        encoding_key,
                    );

                let self_clone = self.clone();

                //
                glib::MainContext::default().spawn_local(async move {
                    if rx.changed().await.is_ok() {
                        if let Some(reason) = rx.borrow().as_ref() {
                            println!("Process stopped: {:?}", reason);
                            // handle reason...
                            if reason != &audioshare::ProcessStopReason::ExitedSuccessfully {
                                self_clone.on_server_error(reason);
                            } else {
                                if let Some(win) = self_clone.main_window() {
                                    if let Some(config_data) = win.imp().config.get() {
                                        let mut config = config_data.borrow_mut(); // Get Ref<AppConfig>
                                        // TODO : After starting the server save config to file
                                        config.server_ip = win.imp().server_ip_entry.text().to_string();
                                        config.server_port = win.imp().server_port_entry.text().to_string().parse().unwrap_or(config.server_port);

                                        let endpoint_selected_name = Self::get_selected_string_from_dropdown(&win.imp().audio_endpoint_dropdown);
                                        config.audio_endpoint = endpoint_selected_name.expect("Failed to get endpoint dropdown string");

                                        let encoding_selected_name = Self::get_selected_string_from_dropdown(&win.imp().audio_encoding_dropdown);
                                        config.audio_encoding = encoding_selected_name.expect("Failed to get encoding dropdown string");

                                        let _ = save_config(&config);
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    fn on_server_error(&self, reason: &audioshare::ProcessStopReason) {

        let notification = gio::Notification::new("audio_share_error");
        notification.set_icon(&gio::ThemedIcon::new(
            "com.subrighteous.audiosharegtk",
        ));

        if reason == &audioshare::ProcessStopReason::InvalidArgument {
            notification.set_title("Invalid Ip Address");
            notification.set_body(Some("Please check the ip address and port then try again."));
        }

        if reason == &audioshare::ProcessStopReason::InvalidBinding {
            notification.set_title("Cannot assign requested address");
            notification.set_body(Some("Please check the ip address and port then try again."));
        }

        self.send_notification(Some("com.subrighteous.audiosharegtk"), &notification);

        //
        if let Some(win) = self.main_window() {
            win.imp().toggle_server.set_label("Start");
            win.imp().toggle_server.add_css_class("success");
            win.imp().toggle_server.remove_css_class("error");

            win.imp().server_ip_entry.set_editable(true);
            win.imp().server_port_entry.set_editable(true);
        }

        self.set_server_active(false);
    }

    fn on_endpoint_dropdown_change(&self, _selected: &String) {
        println!("on_endpoint_dropdown_change : {}", _selected);

        let server_thread = self.imp().audio_share_server_thread.get().unwrap().borrow();

        if server_thread.is_running() {
            // Turn off then on
            self.action_toggle_server();
            self.action_toggle_server();
        }
    }

    fn on_encoding_dropdown_change(&self, _selected: &String) {
        println!("on_encoding_dropdown_change : {}", _selected);

        let server_thread = self.imp().audio_share_server_thread.get().unwrap().borrow();

        if server_thread.is_running() {
            // Turn off then on
            self.action_toggle_server();
            self.action_toggle_server();
        }
    }

    // Reset settings to default
    fn action_reset_server_settings(&self) {
        let server_thread = self.imp().audio_share_server_thread.get().unwrap().borrow();

        if server_thread.is_running() {
            self.action_stop_server(audioshare::ProcessStopReason::Resetting);
        }

        println!("Resetting Settings");

        if let Some(win) = self.main_window() {
            if let Some(config_data) = win.imp().config.get() {
                let config = config_data.borrow(); // Get Ref<AppConfig>

                let server_ip = &config.server_ip;
                let server_port = &config.server_port;
                let audio_endpoint = &config.audio_endpoint;
                let audio_encoding = &config.audio_encoding;

                let pos: u32 = audioshare::get_endpoint_position_in_dropdown(&audio_endpoint);
                win.imp().audio_endpoint_dropdown.set_selected(pos.into());

                let encoding_pos: u32 =
                    audioshare::get_encoding_position_in_dropdown(&audio_encoding);
                win.imp()
                    .audio_encoding_dropdown
                    .set_selected(encoding_pos.into());

                if self.is_server_active() == true {
                    win.imp().server_ip_entry.set_text(server_ip);
                    win.imp()
                        .server_port_entry
                        .set_text(&server_port.to_string());
                } else {
                    win.imp().server_ip_entry.set_text("");
                    win.imp().server_port_entry.set_text("");
                }
            }
        }
    }

    fn get_selected_string_from_dropdown(dropdown: &gtk::DropDown) -> Option<String> {
        let model = dropdown.model()?;
        let selected = dropdown.selected();

        if selected == gtk::INVALID_LIST_POSITION {
            return None;
        }

        let string_list = model.downcast_ref::<gtk::StringList>()?;
        string_list.string(selected as u32).map(|s| s.to_string())
    }
}
