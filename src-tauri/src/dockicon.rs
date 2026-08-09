#![cfg(target_os = "macos")]

use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

const LIGHT: &[u8] = include_bytes!("../icons/dock-light.png");
const DARK: &[u8] = include_bytes!("../icons/dock-dark.png");

pub fn is_dark() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let app = NSApplication::sharedApplication(mtm);
    let name = app.effectiveAppearance().name();
    name.to_string().contains("Dark")
}

pub fn apply(dark: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let bytes = if dark { DARK } else { LIGHT };
    let data = NSData::with_bytes(bytes);
    let image = NSImage::initWithData(NSImage::alloc(), &data);
    let Some(image) = image else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.setApplicationIconImage(Some(&image)) };
}
