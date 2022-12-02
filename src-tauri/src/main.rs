#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use notifications::{Data, Instance};
use std::sync::Mutex;
use std::thread;
use tauri::api::{dialog, shell};
#[cfg(target_os = "macos")]
use tauri::AboutMetadata;
use tauri::{
  command, AppHandle, CustomMenuItem, Manager, Menu, MenuEntry, MenuItem, State, Submenu,
  SystemTray, SystemTrayEvent, Window, WindowBuilder, WindowUrl,
};

#[macro_export]
macro_rules! throw {
  ($($arg:tt)*) => {{
    return Err(format!($($arg)*))
  }};
}

#[command]
fn error_popup(msg: String, win: Window) {
  eprintln!("Error: {}", msg);
  thread::spawn(move || {
    dialog::message(Some(&win), "Error", msg);
  });
}

mod data;
mod notifications;

fn main() {
  let ctx = tauri::generate_context!();

  // macOS "App Nap" periodically pauses our app when it's in the background.
  // We need to prevent that so our intervals are not interrupted.
  #[cfg(target_os = "macos")]
  macos_app_nap::prevent();

  let app_paths = data::AppPaths::from_tauri_config(ctx.config());
  let mut error_msg = None;
  let reminders_file = match data::RemindersFile::load(&app_paths) {
    Ok(reminders_file) => reminders_file,
    Err(e) => {
      error_msg = Some(e);
      data::RemindersFile { groups: Vec::new() }
    }
  };
  let instance = Instance {
    file: reminders_file,
    app_paths,
    scheduler: None,
    bundle_identifier: ctx.config().tauri.bundle.identifier.clone(),
  };

  let app = tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      error_popup,
      notifications::new_group,
      notifications::get_groups,
      notifications::update_group,
      notifications::delete_group,
    ])
    .manage(Data(Mutex::new(instance)))
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .setup(|app| {
      let win = create_window(&app.app_handle());
      match error_msg {
        Some(error_msg) => error_popup(error_msg, win),
        None => {}
      }
      Ok(())
    })
    .system_tray(SystemTray::new())
    .on_system_tray_event(|app, event| match event {
      SystemTrayEvent::LeftClick { .. } => {
        let window = match app.get_window("main") {
          Some(window) => match window.is_visible().expect("winvis") {
            true => {
              // hide the window instead of closing due to processes not closing memory leak: https://github.com/tauri-apps/wry/issues/590
              window.hide().expect("winhide");
              // window.close().expect("winclose");
              set_is_accessory_policy(true);
              return;
            }
            false => window,
          },
          None => create_window(&app.app_handle()),
        };
        set_is_accessory_policy(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
        #[cfg(not(target_os = "macos"))]
        {
          window.show().unwrap();
        }
        window.set_focus().unwrap();
      }
      _ => {}
    })
    .build(ctx)
    .expect("error while running tauri application");
  {
    let data: State<Data> = app.state();
    let mut x = data.0.lock().unwrap();
    x.start();
  }

  app.run(|app_handle, e| match e {
    tauri::RunEvent::ExitRequested { api, .. } => {
      api.prevent_exit();
    }
    tauri::RunEvent::WindowEvent { event, .. } => match event {
      tauri::WindowEvent::CloseRequested { api, .. } => {
        api.prevent_close();
        let window = app_handle.get_window("main").expect("getwin");
        // hide the window instead of closing due to processes not closing memory leak: https://github.com/tauri-apps/wry/issues/590
        window.hide().expect("winhide");
        // window.close().expect("winclose");
        set_is_accessory_policy(true);
      }
      _ => {}
    },
    _ => {}
  });
}

fn create_window(app: &AppHandle) -> Window {
  let win = WindowBuilder::new(app, "main", WindowUrl::default())
    .title("云创客户端")
    .inner_size(1400.0, 900.0)
    .min_inner_size(1280.0, 768.0)
    .visible(false) // tauri_plugin_window_state reveals window
    .skip_taskbar(true);

  #[cfg(target_os = "macos")]
  let win = win
    .transparent(true)
    .title_bar_style(tauri::TitleBarStyle::Transparent);

  let win = win.build().expect("Unable to create window");

  #[cfg(target_os = "macos")]
  {
    use cocoa::appkit::NSWindow;
    let nsw = win.ns_window().unwrap() as cocoa::base::id;
    unsafe {
      nsw.setTitleVisibility_(cocoa::appkit::NSWindowTitleVisibility::NSWindowTitleHidden);

      // set window to always be dark mode
      use cocoa::appkit::NSAppearanceNameVibrantDark;
      use objc::*;
      let appearance: cocoa::base::id = msg_send![
        class!(NSAppearance),
        appearanceNamed: NSAppearanceNameVibrantDark
      ];
      let () = msg_send![nsw, setAppearance: appearance];

      // set window background color
      let bg_color = cocoa::appkit::NSColor::colorWithRed_green_blue_alpha_(
        cocoa::base::nil,
        // also used in App.svelte
        255.0,
        255.0,
        255.0,
        1.0,
      );
      nsw.setBackgroundColor_(bg_color);
    }
  }
  win
}

#[allow(unused_variables)]
fn set_is_accessory_policy(accessory: bool) {
  #[cfg(target_os = "macos")]
  {
    use cocoa::appkit::NSApplication;
    use cocoa::appkit::NSApplicationActivationPolicy::{
      NSApplicationActivationPolicyAccessory, NSApplicationActivationPolicyRegular,
    };
    use objc::*;

    let cls = objc::runtime::Class::get("NSApplication").unwrap();
    let app: cocoa::base::id = unsafe { msg_send![cls, sharedApplication] };
    unsafe {
      if accessory {
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
      } else {
        app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
      }
    }
  }
}
