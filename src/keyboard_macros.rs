use rmk::{
    action::KeyAction,
    config::ForksConfig,
    fork::{Fork, StateBits},
    heapless::Vec,
    hid_state::{HidModifiers, HidMouseButtons},
    k,
    light::LedIndicator,
};

fn shift_override(action: KeyAction, override_action: KeyAction) -> Fork {
    Fork::new(
        action,
        action,
        override_action,
        StateBits::new_from(
            HidModifiers::new_from(false, true, false, false, false, false, false, false),
            LedIndicator::default(),
            HidMouseButtons::default(),
        ),
        StateBits::default(),
        HidModifiers::default(),
        false,
    )
}

pub(crate) fn get_forks() -> ForksConfig {
    ForksConfig {
        forks: Vec::from_slice(&[
            shift_override(k!(Backspace), k!(Delete)),
            shift_override(k!(Home), k!(End)),
            shift_override(k!(PageUp), k!(PageDown)),
        ])
        .expect("Some fork is not valid"),
    }
}
