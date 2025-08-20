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
use crate::apputils;
use crate::config::VERSION;
use crate::configfile::{get_config_path, load_or_create_config, save_config};
use crate::AudiosharegtkWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AudiosharegtkApplication {
        pub is_server_active: Cell<bool>,
        pub audio_share_server_thread: OnceCell<RefCell<audioshare::AudioShareServerThread>>,
        pub test_firewall_thread: OnceCell<RefCell<audioshare::FirewallTestThread>>,
        pub test_firewall_button: RefCell<Option<gtk::Button>>,
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
            self.test_firewall_thread
                .set(RefCell::new(audioshare::FirewallTestThread::new()))
                .expect("test_firewall_thread already set");
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

    pub fn set_test_firewall_button(&self, button: gtk::Button) {
        *self.imp().test_firewall_button.borrow_mut() = Some(button);
    }

    pub fn get_test_firewall_button(&self) -> Option<gtk::Button> {
        self.imp().test_firewall_button.borrow().clone()
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
        let test_firewall = gio::ActionEntry::builder("test_firewall")
            .activate(move |app: &Self, _,_| app.on_test_firewall())
            .build();
        self.add_action_entries([
            force_quit_action,
            quit_action,
            about_action,
            settings_action,
            toggle_server_action,
            reset_server_settings,
            test_firewall,
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
            "https://github.com/SubRighteous/audio-share-linux-gtk",
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
            else{
                // Make sure if we can't load the config we can still quit
                self.quit();
            }
        }
    }

    fn show_settings(&self) {
        let builder = gtk::Builder::from_resource(
            "/com/subrighteous/audiosharegtk/preferences_dialog.ui",
        );

        let window = self.active_window().unwrap();
        let preferences: adw::PreferencesDialog = builder
            .object("preferences_dialog")
            .expect("Failed to get AwaPreferencesDialog");

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

        let notifications_errors_switch: adw::SwitchRow = builder
            .object("notifications_errors")
            .expect("Failed to get notifications_errors");

        let notifications_connection_switch: adw::SwitchRow = builder
            .object("notifications_connection")
            .expect("Failed to get notifications_connection");

        let notifications_disconnection_switch: adw::SwitchRow = builder
            .object("notifications_disconnection")
            .expect("Failed to get notifications_disconnection");

        let test_firewall_button: gtk::Button = builder
            .object("test_firewall_button")
            .expect("test_firewall_button not found");

        self.set_test_firewall_button(test_firewall_button.clone());

        if let Some(win) = self.main_window() {
             if let Some(config_ref) = win.clone().imp().config.get() {
                 let config = config_ref.borrow();


                 do_nothing_check_button
                 .set_active(!(config.auto_start_server || config.keep_last_state));

                 keep_last_state_check_button.set_active(config.keep_last_state);
                 start_server_check_button.set_active(config.auto_start_server);

                 exit_checkbutton.set_active(!(config.minimize_on_exit));
                 minimize_to_tray_checkbutton.set_active(config.minimize_on_exit);

                 notifications_errors_switch.set_active(config.notification_error);
                 notifications_connection_switch.set_active(config.notification_device_connect);
                 notifications_disconnection_switch.set_active(config.notification_device_disconnect);

                preferences.connect_closed(move |_|{
                    // Clone a strong reference to the window (so we can use it in the closure)
                    let window_clone = win.clone();


                    if let Some(config_refcell) = window_clone.imp().config.get() {
                        let mut config = config_refcell.borrow_mut();

                        //Only update if the config is different then the ui
                        if config.minimize_on_exit != minimize_to_tray_checkbutton.is_active()
                            || config.keep_last_state != keep_last_state_check_button.is_active()
                            || config.auto_start_server != start_server_check_button.is_active()
                            || config.notification_error != notifications_errors_switch.is_active()
                            || config.notification_device_connect != notifications_connection_switch.is_active()
                            || config.notification_device_disconnect != notifications_disconnection_switch.is_active()
                        {
                            config.minimize_on_exit = minimize_to_tray_checkbutton.is_active();
                            config.keep_last_state = keep_last_state_check_button.is_active();
                            config.auto_start_server = start_server_check_button.is_active();
                            config.notification_error = notifications_errors_switch.is_active();
                            config.notification_device_connect = notifications_connection_switch.is_active();
                            config.notification_device_disconnect = notifications_disconnection_switch.is_active();

                            let _ = save_config(&config);
                        }
                    } else {
                        println!("No config set yet.");
                    }
                });
            }
        }

        preferences.present(Some(&window));
    }

    fn on_test_firewall(&self){

        if let Some(win) = self.main_window() {
            if let Some(config_ref) = win.imp().config.get() {
                let config = config_ref.borrow();
                let config = config.clone();

                if self.imp()
                    .audio_share_server_thread
                    .get()
                    .unwrap()
                    .borrow().is_running(){
                    let message:String = gettext("AudioShare Server is running in the background.")
                    + " " + &gettext("Please turn the server off then run the firewall test again.");

                    apputils::show_error_notification(self, &gettext("Server is Running"), &message);

                    return;
                }

                if self.imp()
                    .test_firewall_thread
                    .get()
                    .unwrap()
                    .borrow().is_running(){

                    self.imp()
                    .test_firewall_thread
                    .get()
                    .unwrap()
                    .borrow()
                    .stop();

                    if let Some(test_firewall_button) = self.get_test_firewall_button() {
                        test_firewall_button.set_label("Begin Test");
                        test_firewall_button.remove_css_class("error");
                    }


                    return;
                }

                println!("Testing Connection at {}:{}", &config.server_ip, &config.server_port);

                // Start Server
                self.imp()
                    .test_firewall_thread
                    .get()
                    .unwrap()
                    .borrow()
                    .start(
                        config.server_ip,
                        config.server_port,
                );

                if let Some(test_firewall_button) = self.get_test_firewall_button() {
                        test_firewall_button.set_label("Stop Test");
                        test_firewall_button.add_css_class("error");
                }


            }
        }

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

                    },
                ),
            );


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

            // Spawn Listener Tasks Here
            let mut result_rx = self
                .imp()
                .test_firewall_thread
                .get()
                .expect("test_firewall_thread not initialized")
                .borrow()
                .subscribe_result_event();


            // Enroll the "on_server_error" function into the server stop_event
            let mut rx = self
                .imp()
                .audio_share_server_thread
                .get()
                .expect("AudioShareServerThread not initialized")
                .borrow()
                .subscribe_stop_event();

            let mut device_rx = self
                .imp()
                .audio_share_server_thread
                .get()
                .expect("AudioShareServerThread not initialized")
                .borrow()
                .subscribe_device_event();

            let self_clone = self.clone();
            let app = self.clone();
            let alert_dialog_title_pass = gettext("Firewall Test Passed");
            let alert_dialog_title_fail = gettext("Firewall Test Failed");

                // Assign an async function when the server process stoppped
                // or device_connected_notifier broadcasts
                glib::MainContext::default().spawn_local(async move {

                    loop{
                        tokio::select! {

                            Ok(result) = result_rx.recv() => {
                                if let Some(thread_cell) = app.imp().test_firewall_thread.get() {
                                    if let Some(win) = app.main_window() {
                                        if let Some(config_ref) = win.imp().config.get() {
                                            let config = config_ref.borrow();
                                            let config = config.clone();

                                            if result {
                                                apputils::show_alert_dialog(&win, &alert_dialog_title_pass, "Success, clients should be able to connect.");
                                            }else{
                                                let message = gettext("Could not retrieve connection from outside clients.")
                                                + " " +  &gettext("Make sure your app is trying to connect to the server.")
                                                + " " + &gettext("Check your firewall settings and allow tcp and ucp at")
                                                + " " + &config.server_ip + ":" + &config.server_port.to_string();

                                                apputils::show_alert_dialog(&win, &alert_dialog_title_fail, &message);
                                            }

                                            if let Some(test_firewall_button) = app.get_test_firewall_button() {
                                                test_firewall_button.set_label("Begin Test");
                                                test_firewall_button.remove_css_class("error");
                                            }
                                        }
                                    }

                                    thread_cell.borrow_mut().stop();
                                }

                            }

                            Ok((device_ip, connect_status)) = device_rx.recv() => {
                                self_clone.on_device_connect(device_ip, connect_status);
                            }

                            Ok(_) = rx.changed() => {
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
                        }
                    }

                });
        }
    }

    fn action_stop_server(&self, reason : audioshare::ProcessStopReason){
        if self.is_server_active() == true {
            println!("Stopping the server");
            self.set_server_active(false);

            if let Some(win) = self.main_window() {
                win.imp().toggle_server.set_label(&gettext("Start"));
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
                win.imp().toggle_server.set_label(&gettext("Start"));
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
            if let Some(win) = self.main_window(){
            //if let Ok(_) = self.main_window().expect("idk and don't care").imp().server_port_entry.text().parse::<u16>() || {
            if let Ok(_) = win.imp().server_port_entry.text().parse::<u16>(){
                println!("Valid 1");
            }else{

                if !win.imp().server_port_entry.text().is_empty(){
                    apputils::show_error_notification(
                        self,
                        &gettext("Invalid Port"),
                        &gettext("Please enter a number between 0 and 65535."),
                    );
                    return;
                }

                if let Some(placeholder) = win.imp().server_port_entry.placeholder_text(){

                    if let Ok(_) = placeholder.to_string().parse::<u16>(){
                        win.imp().server_port_entry.set_text(&placeholder.to_string());
                    }
                    else{
                        return;
                    }
                }
                else{
                    return;
                }
            }




            println!("Starting the server");
            self.set_server_active(true);

            if let Some(win) = self.main_window() {
                win.imp().toggle_server.set_label(&gettext("Stop"));
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

            }
        }
        }
    }

    fn on_device_connect(&self, device_ip: String , connected: bool){

        let notification = gio::Notification::new("audio_share_info");
        notification.set_icon(&gio::ThemedIcon::new(
            "com.subrighteous.audiosharegtk",
        ));
        let message;
        let title;

        if connected {
            title = gettext("Device Connected");
            message = device_ip.clone() + " " + &gettext("connected from the server");
        }
        else{
            title = gettext("Device Disconnected");
            message = device_ip.clone() + " " + &gettext("disconnected from the server");
        }

        if let Some(win) = self.main_window() {

            if let Some(config_data) = win.imp().config.get() {
                let config = config_data.borrow_mut(); // Get Ref<AppConfig>
                if config.notification_device_connect && connected{
                    apputils::show_connection_notification(self, &title , &message, &connected);
                }
                if config.notification_device_disconnect && !connected{
                    apputils::show_connection_notification(self, &title , &message, &connected);
                }

            }

        }


    }

    fn on_server_error(&self, reason: &audioshare::ProcessStopReason) {
        let mut title: String = String::new();
        let mut message: String = String::new();

        if reason == &audioshare::ProcessStopReason::InvalidArgument {
            title = gettext("Invalid ip address");
            message = gettext("Please check the ip address and port then try again.");
        }

        if reason == &audioshare::ProcessStopReason::InvalidBinding {
            let title_text  = gettext("Cannot assign requested address");
            title = title_text;
            message = gettext("Please check the ip address and port then try again.");
        }

        //
        if let Some(win) = self.main_window() {

            if let Some(config_data) = win.imp().config.get() {
                let config = config_data.borrow_mut(); // Get Ref<AppConfig>
                if config.notification_error{

                    apputils::show_error_notification(self, &title, &message);
                    //self.send_notification(Some("com.subrighteous.audiosharegtk"), &notification);
                }

            }

            win.imp().toggle_server.set_label("Start");
            win.imp().toggle_server.add_css_class("success");
            win.imp().toggle_server.remove_css_class("error");

            win.imp().server_ip_entry.set_editable(true);
            win.imp().server_port_entry.set_editable(true);

            win.imp().server_ip_entry.set_secondary_icon_name(None);
            win.imp().server_port_entry.set_secondary_icon_name(None);
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

                let server_ip = &audioshare::get_local_ipv4();
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
