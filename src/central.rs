#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;
mod keyboard_macros;
mod oled;
mod vial;
use crate::{
    keyboard_macros::get_forks,
    keymap::{COLS, ROWS},
    oled::{init_oled_terminal, Oled},
};
use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    flash::{Async as FlashAsync, Flash},
    gpio::{Input, Output},
    peripherals::{PIO0, USB},
    usb::{Driver, InterruptHandler as USBInterruptHandler},
};
use panic_probe as _;
use rmk::{
    action::{Action, KeyAction},
    channel::{CONTROLLER_CHANNEL, EVENT_CHANNEL},
    config::{
        BehaviorConfig, ControllerConfig, KeyboardUsbConfig, RmkConfig, StorageConfig, VialConfig,
    },
    debounce::default_debouncer::DefaultDebouncer,
    futures::future::join4,
    heapless::String,
    initialize_keymap_and_storage,
    input_device::Runnable,
    keyboard::Keyboard,
    keycode::KeyCode,
    light::LightController,
    run_devices, run_rmk,
    split::{
        central::{run_peripheral_manager, CentralMatrix},
        rp::uart::{BufferedUart, UartInterruptHandler},
        SPLIT_MESSAGE_MAX_SIZE,
    },
};
use ssd1306::{mode::TerminalModeAsync, prelude::DisplayRotation};
use static_cell::StaticCell;
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => USBInterruptHandler<USB>;
    PIO0_IRQ_0 => UartInterruptHandler<PIO0>;
});

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const ROW_OFFSET: usize = ROWS;
const COL_OFFSET: usize = 0;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize peripherals
    let p = embassy_rp::init(Default::default());

    // Create the usb driver, from the HAL
    let usb_driver = Driver::new(p.USB, Irqs);

    // Pin config
    let (input_pins, output_pins) = config_matrix_pins_rp!(
        peripherals: p,
        input: [PIN_5, PIN_6, PIN_7, PIN_8, PIN_9],
        output: [PIN_27, PIN_26, PIN_22, PIN_20, PIN_23, PIN_21]
    );

    // Use internal flash to emulate eeprom
    let flash = Flash::<_, FlashAsync, FLASH_SIZE>::new(p.FLASH, p.DMA_CH0);

    let keyboard_usb_config = KeyboardUsbConfig {
        vid: 0x4c4b,
        pid: 0x4643,
        manufacturer: "Lily58 Pro",
        product_name: "Lily58 Pro",
        serial_number: "vial:f64c2b3c:000001",
    };

    let vial_config = VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF);

    let rmk_config = RmkConfig {
        usb_config: keyboard_usb_config,
        vial_config,
        ..Default::default()
    };

    static RX_BUF: StaticCell<[u8; SPLIT_MESSAGE_MAX_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; SPLIT_MESSAGE_MAX_SIZE])[..];
    let uart_receiver = BufferedUart::new_half_duplex(p.PIO0, p.PIN_1, rx_buf, Irqs);

    // Initialize the storage and keymap
    let mut default_keymap = keymap::get_default_keymap();

    let behavior_config = BehaviorConfig {
        fork: get_forks(),
        ..BehaviorConfig::default()
    };
    let storage_config = StorageConfig {
        start_addr: 0,
        num_sectors: 2,
        clear_storage: true,
    };
    let (keymap, mut storage) =
        initialize_keymap_and_storage(&mut default_keymap, flash, &storage_config, behavior_config)
            .await;

    // Initialize the matrix + keyboard
    let debouncer = DefaultDebouncer::<ROWS, COLS>::new();
    let mut matrix =
        CentralMatrix::<_, _, _, 0, 0, ROWS, COLS>::new(input_pins, output_pins, debouncer);

    let mut keyboard = Keyboard::new(&keymap);

    // Initialize the light controller
    let mut light_controller: LightController<Output> =
        LightController::new(ControllerConfig::default().light_config);

    // initialize the OLED display
    let display = init_oled_terminal(p.I2C0, p.PIN_16, p.PIN_17, DisplayRotation::Rotate90).await;
    spawner.spawn(key_display_task(display)).unwrap();

    // Start
    join4(
        run_devices! ((matrix) => EVENT_CHANNEL),
        keyboard.run(),
        run_peripheral_manager::<ROWS, COLS, ROW_OFFSET, COL_OFFSET, _>(0, uart_receiver),
        run_rmk(
            &keymap,
            usb_driver,
            &mut storage,
            &mut light_controller,
            rmk_config,
        ),
    )
    .await;
}

fn display_key_action(key_action: &KeyAction, buffer: &mut String<12>) {
    fn display_key_code(key_code: &KeyCode) -> Option<&'static str> {
        let s = match key_code {
            KeyCode::A => " A ",
            KeyCode::B => " B ",
            KeyCode::C => " C ",
            KeyCode::D => " D ",
            KeyCode::E => " E ",
            KeyCode::F => " F ",
            KeyCode::G => " G ",
            KeyCode::H => " H ",
            KeyCode::I => " I ",
            KeyCode::J => " J ",
            KeyCode::K => " K ",
            KeyCode::L => " L ",
            KeyCode::M => " M ",
            KeyCode::N => " N ",
            KeyCode::O => " O ",
            KeyCode::P => " P ",
            KeyCode::Q => " Q ",
            KeyCode::R => " R ",
            KeyCode::S => " S ",
            KeyCode::T => " T ",
            KeyCode::U => " U ",
            KeyCode::V => " V ",
            KeyCode::W => " W ",
            KeyCode::X => " X ",
            KeyCode::Y => " Y ",
            KeyCode::Z => " Z ",
            KeyCode::Kc1 => " 1 ",
            KeyCode::Kc2 => " 2 ",
            KeyCode::Kc3 => " 3 ",
            KeyCode::Kc4 => " 4 ",
            KeyCode::Kc5 => " 5 ",
            KeyCode::Kc6 => " 6 ",
            KeyCode::Kc7 => " 7 ",
            KeyCode::Kc8 => " 8 ",
            KeyCode::Kc9 => " 9 ",
            KeyCode::Kc0 => " 0 ",
            KeyCode::Enter => "ENT",
            KeyCode::Escape => "ESC",
            KeyCode::Backspace => "BSP",
            KeyCode::Tab => "TAB",
            KeyCode::Space => "SPC",
            KeyCode::Minus => " - ",
            KeyCode::Equal => " = ",
            KeyCode::LeftBracket => " [ ",
            KeyCode::RightBracket => " ] ",
            KeyCode::Backslash => "\\",
            KeyCode::Semicolon => " ; ",
            KeyCode::Quote => " ' ",
            KeyCode::Grave => " ` ",
            KeyCode::Comma => " , ",
            KeyCode::Dot => " . ",
            KeyCode::Slash => " / ",
            KeyCode::F1 => " F1",
            KeyCode::F2 => " F2",
            KeyCode::F3 => " F3",
            KeyCode::F4 => " F4",
            KeyCode::F5 => " F5",
            KeyCode::F6 => " F6",
            KeyCode::F7 => " F7",
            KeyCode::F8 => " F8",
            KeyCode::F9 => " F9",
            KeyCode::F10 => "F10",
            KeyCode::F11 => "F11",
            KeyCode::F12 => "F12",
            KeyCode::Right => "RGT",
            KeyCode::Left => "LFT",
            KeyCode::Down => "DWN",
            KeyCode::Up => "UP",
            KeyCode::F13 => "F13",
            KeyCode::F14 => "F14",
            KeyCode::F15 => "F15",
            KeyCode::F16 => "F16",
            KeyCode::F17 => "F17",
            KeyCode::F18 => "F18",
            KeyCode::F19 => "F19",
            KeyCode::F20 => "F20",
            KeyCode::F21 => "F21",
            KeyCode::F22 => "F22",
            KeyCode::F23 => "F23",
            KeyCode::F24 => "F24",
            _ => return None,
        };
        Some(s)
    }

    fn display_action(action: &Action) -> Option<&'static str> {
        match action {
            Action::Key(key_code) => display_key_code(key_code),
            Action::Modifier(_) => None,
            Action::LayerOn(layer) => Some(match layer {
                0 => "BSE",
                1 => "LOW",
                2 => "RAI",
                _ => "HUH",
            }),
            _ => None,
        }
    }

    let s = match key_action {
        KeyAction::Single(action) => display_action(action),
        KeyAction::Tap(action) => display_action(action),
        KeyAction::OneShot(action) => display_action(action),
        KeyAction::LayerTapHold(action, _) => display_action(action),
        KeyAction::WithModifier(action, _) => display_action(action),
        KeyAction::ModifierTapHold(action, _) => display_action(action),
        _ => None,
    };

    if let Some(s) = s {
        buffer.clear();
        let _ = writeln!(buffer, "{s}");
    }
}

#[embassy_executor::task]
async fn key_display_task(mut display: Oled<TerminalModeAsync>) {
    let mut subscriber = CONTROLLER_CHANNEL.subscriber().unwrap();
    let mut text_buffer = String::<12>::new();
    loop {
        let event = subscriber.next_message_pure().await;
        if let rmk::event::ControllerEvent::Key(key_event, key_action) = event {
            if key_event.pressed {
                display_key_action(&key_action, &mut text_buffer);
                let _ = display.write_str(&text_buffer).await;
            }
        }
    }
}
