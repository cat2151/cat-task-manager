//! Detects an already running cat-task-manager by its Windows Terminal tab
//! title and selects that tab through UI Automation, which also brings that
//! Windows Terminal window to the foreground. `wt focus-tab` is not used
//! because it only reaches the window it is launched from.
//!
//! Must run before this process sets its own title, or it finds its own tab.

#[cfg(windows)]
use windows::{
    core::BSTR,
    Win32::{
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_MULTITHREADED,
            },
            Variant::VARIANT,
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationSelectionItemPattern, TreeScope_Children,
            TreeScope_Descendants, UIA_ClassNamePropertyId, UIA_ControlTypePropertyId,
            UIA_NamePropertyId, UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
        },
    },
};

#[cfg(windows)]
use crate::terminal::TITLE;

#[cfg(windows)]
const TERMINAL_WINDOW_CLASS: &str = "CASCADIA_HOSTING_WINDOW_CLASS";

/// Returns whether a running instance's tab was selected. A UI Automation
/// failure counts as "not running" so that startup is never blocked by it.
#[cfg(windows)]
pub fn activate_running_instance() -> bool {
    // SAFETY: CoUninitialize is paired with a successful CoInitializeEx, and
    // every COM interface created in select_tab is dropped before it returns.
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            return false;
        }
        let selected = select_tab(TITLE).unwrap_or(false);
        CoUninitialize();
        selected
    }
}

#[cfg(not(windows))]
pub fn activate_running_instance() -> bool {
    false
}

/// Returns whether a tab whose title equals `name` exactly was selected.
#[cfg(windows)]
unsafe fn select_tab(name: &str) -> windows::core::Result<bool> {
    let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
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
        item.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)?
            .Select()?;
        return Ok(true);
    }
    Ok(false)
}
