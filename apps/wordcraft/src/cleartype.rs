//! The system's ClearType setting: page text is drawn with subpixel edges, in the screen's
//! stripe order, when Windows draws its own text that way (Settings › Personalization › Fonts,
//! or "Adjust ClearType text"), and with grey edges when it is off.

use wordcraft_ui_egui::canvas::SubpixelOrder;

/// The subpixel order Windows draws text in, `None` for grey edges (and on other systems).
pub fn system_order() -> Option<SubpixelOrder> {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        let Ok(desktop) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\Desktop") else {
            return from_values(None, None, None);
        };
        let smoothing: Option<String> = desktop.get_value("FontSmoothing").ok();
        let kind: Option<u32> = desktop.get_value("FontSmoothingType").ok();
        let orientation: Option<u32> = desktop.get_value("FontSmoothingOrientation").ok();
        from_values(smoothing.as_deref(), kind, orientation)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// The order from the values under `HKCU\Control Panel\Desktop`: `FontSmoothing` ("0" off, "2"
/// on), `FontSmoothingType` (1 standard, grey; 2 ClearType) and `FontSmoothingOrientation`
/// (0 BGR, 1 RGB). A missing value is Windows' default: ClearType, RGB. An orientation Windows
/// doesn't document gets grey edges rather than fringes of the wrong colour.
#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
pub fn from_values(smoothing: Option<&str>, kind: Option<u32>, orientation: Option<u32>) -> Option<SubpixelOrder> {
    if smoothing.is_some_and(|s| s.trim() == "0") || kind.is_some_and(|k| k != 2) {
        return None;
    }
    match orientation {
        None | Some(1) => Some(SubpixelOrder::Rgb),
        Some(0) => Some(SubpixelOrder::Bgr),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleartype_settings_pick_the_stripe_order() {
        assert_eq!(from_values(None, None, None), Some(SubpixelOrder::Rgb), "Windows' default");
        assert_eq!(from_values(Some("2"), Some(2), Some(1)), Some(SubpixelOrder::Rgb));
        assert_eq!(from_values(Some("2"), Some(2), Some(0)), Some(SubpixelOrder::Bgr));
        assert_eq!(from_values(Some("2"), Some(1), Some(1)), None, "standard smoothing: grey");
        assert_eq!(from_values(Some("0"), Some(2), Some(1)), None, "smoothing off");
        assert_eq!(from_values(Some("2"), Some(2), Some(7)), None, "unknown order");
    }
}
