use rmk::{
    config::ForksConfig,
    fork::{Fork, StateBits},
    heapless::Vec,
    k,
    types::{
        action::KeyAction, led_indicator::LedIndicator, modifier::ModifierCombination,
        mouse_button::MouseButtons,
    },
};

fn shift_override(action: KeyAction, override_action: KeyAction) -> Fork {
    Fork::new(
        action,
        action,
        override_action,
        StateBits::new_from(
            ModifierCombination::new_from(false, true, false, false, false),
            LedIndicator::default(),
            MouseButtons::default(),
        ),
        StateBits::default(),
        ModifierCombination::default(),
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
