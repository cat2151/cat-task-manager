//! Selects the Windows Terminal tab titled `cat-task-manager` through UI
//! Automation, which also brings that Windows Terminal window to the
//! foreground. `wt focus-tab` is not used because it only reaches the window
//! it is launched from.

#[cfg(windows)]
use windows::{
    core::BSTR,
    Win32::{
        System::{
            Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED},
            Variant::VARIANT,
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationSelectionItemPattern, TreeScope_Children,
            TreeScope_Descendants, UIA_ClassNamePropertyId, UIA_ControlTypePropertyId,
            UIA_NamePropertyId, UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
        },
    },
};

use crate::terminal::TITLE;

#[cfg(windows)]
const TERMINAL_WINDOW_CLASS: &str = "CASCADIA_HOSTING_WINDOW_CLASS";

/// Errors when no tab is titled exactly `cat-task-manager`.
#[cfg(windows)]
pub fn activate() -> Result<(), String> {
    match select_tab(TITLE) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "Windows Terminalに「{TITLE}」という名前のタブが見つかりませんでした"
        )),
        Err(err) => Err(format!(
            "Windows Terminalの「{TITLE}」タブを選べませんでした: {err}"
        )),
    }
}

#[cfg(not(windows))]
pub fn activate() -> Result<(), String> {
    Err(format!("「{TITLE}」タブのactive化はWindows専用です"))
}

/// Returns whether a tab whose title equals `name` exactly was selected.
#[cfg(windows)]
fn select_tab(name: &str) -> windows::core::Result<bool> {
    // SAFETY: COM is initialized for this thread before any UI Automation
    // call, and every interface pointer comes from windows-rs wrappers that
    // release themselves on drop.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let terminal = automation.CreatePropertyCondition(
            UIA_ClassNamePropertyId,
            &VARIANT::from(BSTR::from(TERMINAL_WINDOW_CLASS)),
        )?;
        let tab = automation.CreateAndCondition(
            &automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_TabItemControlTypeId.0),
            )?,
            &automation
                .CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(BSTR::from(name)))?,
        )?;
        let windows = automation
            .GetRootElement()?
            .FindAll(TreeScope_Children, &terminal)?;
        for index in 0..windows.Length()? {
            let Ok(item) = windows
                .GetElement(index)?
                .FindFirst(TreeScope_Descendants, &tab)
            else {
                continue;
            };
            item.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                UIA_SelectionItemPatternId,
            )?
            .Select()?;
            return Ok(true);
        }
        Ok(false)
    }
}
