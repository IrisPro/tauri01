#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::sync::Mutex;
use std::thread;

use notifications::{Data, Group, Instance};
use tauri::api::{dialog, shell};
use tauri::{
  command, AboutMetadata, CustomMenuItem, Manager, Menu, MenuEntry, MenuItem, Submenu, Window,
  WindowBuilder, WindowUrl,
};
use tokio;

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

mod notifications;

#[tokio::main]
async fn main() {
  let ctx = tauri::generate_context!();

  // macOS "App Nap" periodically pauses our app when it's in the background.
  // We need to prevent that so our intervals are not interrupted.
  #[cfg(target_os = "macos")]
  macos_app_nap::prevent();

  tauri::async_runtime::set(tokio::runtime::Handle::current());

  let groups = vec![
    Group {
      title: "Rabbit stuff".to_string(),
      description: "Yo".to_string(),
      enabled: true,
      id: 0,
      next_date: Some(0),
      cron: "0 0 0 * * *".to_string(),
    },
    Group {
      title: "Things".to_string(),
      description: "yea".to_string(),
      enabled: true,
      id: 1,
      next_date: Some(0),
      cron: "0 0 1,13 1-15 Jan-Nov Mon,Wed,Fri".to_string(),
    },
  ];
  let mut instance = Instance {
    scheduler: None,
    groups,
  };
  let instance_result = instance.start();

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      error_popup,
      notifications::get_groups,
      notifications::new_group,
    ])
    .manage(Data(Mutex::new(instance)))
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .setup(|app| {
      let _win = WindowBuilder::new(app, "main", WindowUrl::default())
        .title("Remind Me Again")
        .inner_size(400.0, 550.0)
        .min_inner_size(400.0, 200.0)
        .transparent(true)
        .build()
        .expect("Unable to create window");

      match instance_result {
        Ok(_) => {}
        Err(e) => error_popup(e, _win.clone()),
      }

      #[cfg(target_os = "macos")]
      {
        use cocoa::appkit::NSWindow;
        let nsw = _win.ns_window().unwrap() as cocoa::base::id;
        unsafe {
          // manual implementation for now (PR https://github.com/tauri-apps/tauri/pull/3965)
          {
            nsw.setTitlebarAppearsTransparent_(cocoa::base::YES);

            // tauri enables fullsizecontentview by default, so disable it
            let mut style_mask = nsw.styleMask();
            style_mask.set(
              cocoa::appkit::NSWindowStyleMask::NSFullSizeContentViewWindowMask,
              false,
            );
            nsw.setStyleMask_(style_mask);
          }

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
            // 34.0 / 255.0 * 0.5,
            // 38.0 / 255.0 * 0.5,
            // 45.5 / 255.0 * 0.5,
            // 1.0,
            8.0 / 255.0,
            9.0 / 255.0,
            13.0 / 255.0,
            1.0,
          );
          nsw.setBackgroundColor_(bg_color);
        }
      }
      Ok(())
    })
    .menu(Menu::with_items([
      #[cfg(target_os = "macos")]
      MenuEntry::Submenu(Submenu::new(
        &ctx.package_info().name,
        Menu::with_items([
          MenuItem::About(ctx.package_info().name.clone(), AboutMetadata::default()).into(),
          MenuItem::Separator.into(),
          MenuItem::Services.into(),
          MenuItem::Separator.into(),
          MenuItem::Hide.into(),
          MenuItem::HideOthers.into(),
          MenuItem::ShowAll.into(),
          MenuItem::Separator.into(),
          MenuItem::Quit.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "File",
        Menu::with_items([MenuItem::CloseWindow.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Edit",
        Menu::with_items([
          MenuItem::Undo.into(),
          MenuItem::Redo.into(),
          MenuItem::Separator.into(),
          MenuItem::Cut.into(),
          MenuItem::Copy.into(),
          MenuItem::Paste.into(),
          #[cfg(not(target_os = "macos"))]
          MenuItem::Separator.into(),
          MenuItem::SelectAll.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "View",
        Menu::with_items([MenuItem::EnterFullScreen.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Window",
        Menu::with_items([MenuItem::Minimize.into(), MenuItem::Zoom.into()]),
      )),
      // You should always have a Help menu on macOS because it will automatically
      // show a menu search field
      MenuEntry::Submenu(Submenu::new(
        "Help",
        Menu::with_items([CustomMenuItem::new("Learn More", "Learn More").into()]),
      )),
    ]))
    .on_menu_event(|event| {
      let event_name = event.menu_item_id();
      match event_name {
        "Learn More" => {
          let url = "https://github.com/probablykasper/remind-me-again".to_string();
          shell::open(&event.window().shell_scope(), url, None).unwrap();
        }
        _ => {}
      }
    })
    .run(ctx)
    .expect("error while running tauri application");
}
