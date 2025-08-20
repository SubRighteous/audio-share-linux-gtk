use gtk::{gio};

use adw::prelude::*;

const APP_ID:&str = "com.subrighteous.audiosharegtk";

// pub fn show_info_notification<App: IsA<gio::Application>>(window: &App, title: &str, message: &str){
//     let notification = gio::Notification::new("audio_share_info");
    // notification.set_icon(&gio::ThemedIcon::new(
    //     APP_ID,
    // ));

//     notification.set_title(title);
//     notification.set_body(Some(&message));
//     let icon = gio::ThemedIcon::new("dialog-information-symbolic");
//     notification.set_icon(&icon);

//     window.send_notification(Some(APP_ID), &notification);
// }

pub fn show_connection_notification<App: IsA<gio::Application>>(window: &App, title: &str, message: &str, connected: &bool){
    let notification = gio::Notification::new("audio_share_info");
    // notification.set_icon(&gio::ThemedIcon::new(
    //     APP_ID,
    // ));

    notification.set_title(title);
    notification.set_body(Some(&message));
    if *connected{
        let icon = gio::ThemedIcon::new("network-connect");
        notification.set_icon(&icon);
    }else{
        let icon = gio::ThemedIcon::new("network-disconnect");
        notification.set_icon(&icon);
    }


    window.send_notification(Some(APP_ID), &notification);
}

pub fn show_error_notification<App: IsA<gio::Application>>(window: &App, title: &str, message: &str){
    let notification = gio::Notification::new("audio_share_error");
    // notification.set_icon(&gio::ThemedIcon::new(
    //     APP_ID,
    // ));

    notification.set_title(title);
    notification.set_body(Some(&message));

    let icon = gio::ThemedIcon::new("action-unavailable-symbolic");
    notification.set_icon(&icon);

    window.send_notification(Some(APP_ID), &notification);
}

pub fn show_alert_dialog<App: IsA<gtk::Widget>>(window: &App, title: &str, message: &str){
    // Create a new AlertDialog instance.
    let dialog = adw::AlertDialog::builder()
        .heading(title)
        .body(message) // Use the text from our label as the body
        .build();

    // Add a primary "OK" button. The ID is a string that will be returned on click.
    dialog.add_response("ok", "OK");

    dialog.present(Some(window));
}
