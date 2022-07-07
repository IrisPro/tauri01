#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tauri::api::shell;
use tauri::{
  AboutMetadata, CustomMenuItem, Manager, Menu, MenuEntry, MenuItem, Submenu, WindowBuilder,
  WindowUrl,
};

mod cmd;

fn main() {
  let ctx = tauri::generate_context!();

  // macOS "App Nap" periodically pauses our app when it's in the background.
  // We need to prevent that so our intervals are not interrupted.
  #[cfg(target_os = "macos")]
  macos_app_nap::prevent();

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![])
    .setup(|app| {
      let _win = WindowBuilder::new(app, "main", WindowUrl::default())
        .title("RemindMeAgain")
        .inner_size(800.0, 550.0)
        .min_inner_size(400.0, 200.0)
        .transparent(true)
        .build()
        .expect("Unable to create window");

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
            0.0,
            0.0,
            0.0,
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
          let url = "https://github.com/probablykasper/tauri-template".to_string();
          shell::open(&event.window().shell_scope(), url, None).unwrap();
        }
        _ => {}
      }
    })
    .run(ctx)
    .expect("error while running tauri application");
}
