#![cfg(target_os = "macos")]

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSToolbar, NSWindow, NSWindowDidExitFullScreenNotification, NSWindowStyleMask,
    NSWindowTabbingMode, NSWindowToolbarStyle, NSWindowWillEnterFullScreenNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSString};

const TOOLBAR_ID: &str = "packweave";

pub fn style_window(window: &tauri::WebviewWindow) {
    let (Some(mtm), Ok(ptr)) = (MainThreadMarker::new(), window.ns_window()) else {
        return;
    };
    let ns_window = unsafe { &*ptr.cast::<NSWindow>() };
    ns_window.setTabbingMode(NSWindowTabbingMode::Disallowed);
    add_toolbar(ns_window, mtm);
}

pub fn follow_fullscreen() {
    let center = NSNotificationCenter::defaultCenter();
    let entering = RcBlock::new(|note: NonNull<NSNotification>| {
        if let Some(window) = our_window(note) {
            window.setToolbar(None);
        }
    });
    let exited = RcBlock::new(|note: NonNull<NSNotification>| {
        if let (Some(window), Some(mtm)) = (our_window(note), MainThreadMarker::new()) {
            add_toolbar(&window, mtm);
        }
    });
    unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWindowWillEnterFullScreenNotification),
            None,
            None,
            &entering,
        );
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWindowDidExitFullScreenNotification),
            None,
            None,
            &exited,
        );
    }
}

fn add_toolbar(window: &NSWindow, mtm: MainThreadMarker) {
    let full_screen = window.styleMask().contains(NSWindowStyleMask::FullScreen);
    if full_screen || window.toolbar().is_some() {
        return;
    }
    let toolbar =
        NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), &NSString::from_str(TOOLBAR_ID));
    window.setToolbar(Some(&toolbar));
    window.setToolbarStyle(NSWindowToolbarStyle::UnifiedCompact);
}

fn our_window(note: NonNull<NSNotification>) -> Option<Retained<NSWindow>> {
    let note = unsafe { note.as_ref() };
    let window = note.object()?.downcast::<NSWindow>().ok()?;
    let overlay = window.titlebarAppearsTransparent()
        && window
            .styleMask()
            .contains(NSWindowStyleMask::FullSizeContentView);
    overlay.then_some(window)
}
