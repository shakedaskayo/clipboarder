//! macOS native helpers: window panel-style tweaks + frontmost-app detection.
//!
//! Uses the (deprecated) `cocoa` crate for stability. The successor
//! `objc2-app-kit` has shifted its API between versions; pinning to the
//! mature cocoa crate is more reliable for now.
#![cfg(target_os = "macos")]
#![allow(deprecated)]
// The objc crate's `sel_impl!` macro internally uses
// `cfg(feature = "cargo-clippy")` — an old idiom. Suppress the lint at the
// boundary so we don't have to fork or patch upstream.
#![allow(unexpected_cfgs)]

use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use tauri::WebviewWindow;

pub fn configure_window(win: &WebviewWindow) {
    let Ok(ptr) = win.ns_window() else { return; };
    if ptr.is_null() { return; }
    let ns_window: id = ptr as id;
    unsafe {
        // NSFloatingWindowLevel = 3, above the normal window level.
        let _: () = msg_send![ns_window, setLevel: 3i64];
        let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary;
        ns_window.setCollectionBehavior_(behavior);
        // Auto-hide the moment the user clicks into any other app. AppKit
        // handles this transition synchronously — no perceptible delay.
        // Use objc::runtime::YES (BOOL = 1) so the bridge knows the exact width.
        let yes: objc::runtime::BOOL = objc::runtime::YES;
        let _: () = msg_send![ns_window, setHidesOnDeactivate: yes];
    }
}

/// Hide the entire app (all windows) and reactivate the previously-active
/// app. This is what makes paste-back land in the right window — a bare
/// NSWindow.orderOut: leaves clipboarder still "active" so synthesized
/// ⌘V events bounce off our own (now invisible) window.
pub fn hide_app() {
    unsafe {
        let app: id = msg_send![class!(NSApplication), sharedApplication];
        if app == nil { return; }
        let _: () = msg_send![app, hide: nil];
    }
}

/// Returns the bundle identifier of the frontmost application, e.g.
/// "com.apple.Safari". Returns None when nothing is frontmost or the call
/// fails — callers should treat that as "no exclusion match".
pub fn frontmost_bundle_id() -> Option<String> {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil { return None; }
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil { return None; }
        let bundle: id = msg_send![app, bundleIdentifier];
        if bundle == nil { return None; }
        let utf8: *const std::os::raw::c_char = msg_send![bundle, UTF8String];
        if utf8.is_null() { return None; }
        Some(std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }
}

/// Extract a 32x32 PNG of the app icon for the given bundle identifier.
/// Returns None when the app can't be located or rendering fails.
pub fn extract_app_icon_png(bundle_id: &str, side: u32) -> Option<Vec<u8>> {
    if bundle_id.is_empty() { return None; }
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil { return None; }

        let bundle_ns = NSString::alloc(nil).init_str(bundle_id);
        // NSWorkspace's modern API: -URLForApplicationWithBundleIdentifier:
        let url: id = msg_send![workspace, URLForApplicationWithBundleIdentifier: bundle_ns];
        let path: id = if url != nil {
            msg_send![url, path]
        } else {
            // Fallback: deprecated -absolutePathForAppBundleWithIdentifier:
            msg_send![workspace, absolutePathForAppBundleWithIdentifier: bundle_ns]
        };
        if path == nil { return None; }

        let icon: id = msg_send![workspace, iconForFile: path];
        if icon == nil { return None; }

        let target = NSSize::new(side as f64, side as f64);
        let _: () = msg_send![icon, setSize: target];

        let tiff: id = msg_send![icon, TIFFRepresentation];
        if tiff == nil { return None; }

        let bmp_rep: id = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        if bmp_rep == nil { return None; }

        // NSBitmapImageFileTypePNG = 4
        let png_data: id = msg_send![bmp_rep, representationUsingType: 4u64 properties: nil];
        if png_data == nil { return None; }

        let bytes_ptr: *const u8 = msg_send![png_data, bytes];
        let length: usize = msg_send![png_data, length];
        if bytes_ptr.is_null() || length == 0 { return None; }

        Some(std::slice::from_raw_parts(bytes_ptr, length).to_vec())
    }
}

/// Returns the localized name of the frontmost application, for the UI.
pub fn frontmost_app_name() -> Option<String> {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil { return None; }
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil { return None; }
        let name: id = msg_send![app, localizedName];
        if name == nil { return None; }
        let utf8: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if utf8.is_null() { return None; }
        Some(std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }
}

