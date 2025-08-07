/* window.rs
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

use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use std::cell::RefCell;
use once_cell::sync::OnceCell;
use crate::configfile::AppConfig;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/subrighteous/audiosharegtk/window.ui")]
    pub struct AudiosharegtkWindow {
        // Configuration File Data
        pub config: OnceCell<RefCell<AppConfig>>,

        // Template widgets

        // Start/Stop Server Button
        #[template_child(id = "toggle_server")]
        pub toggle_server: TemplateChild<gtk::Button>,

        #[template_child(id = "ResetServerButton")]
        pub reset_server: TemplateChild<gtk::Button>,

        // Server Input Widgets
        #[template_child(id = "server_ip_entry")]
        pub server_ip_entry: TemplateChild<gtk::Entry>,

        #[template_child(id = "server_port_entry")]
        pub server_port_entry: TemplateChild<gtk::Entry>,

        // Drop down Widgets
        #[template_child(id = "AudioEndpoint_Dropdown")]
        pub audio_endpoint_dropdown: TemplateChild<gtk::DropDown>,

        #[template_child(id = "AudioEncoding_Dropdown")]
        pub audio_encoding_dropdown: TemplateChild<gtk::DropDown>,

        #[template_child(id = "AudioEncoding_Box")]
        pub audio_encoding_box: TemplateChild<gtk::Box>,

        //pub label: TemplateChild<gtk::Label>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AudiosharegtkWindow {
        const NAME: &'static str = "AudiosharegtkWindow";
        type Type = super::AudiosharegtkWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AudiosharegtkWindow {}
    impl WidgetImpl for AudiosharegtkWindow {}
    impl WindowImpl for AudiosharegtkWindow {}
    impl ApplicationWindowImpl for AudiosharegtkWindow {}
    impl AdwApplicationWindowImpl for AudiosharegtkWindow {}
}

glib::wrapper! {
    pub struct AudiosharegtkWindow(ObjectSubclass<imp::AudiosharegtkWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl AudiosharegtkWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
