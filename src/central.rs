#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;
mod keyboard_macros;
mod vial;
use core::{
    fmt::Write,
    ops::{BitAnd, BitOr, BitXor},
};
use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    bind_interrupts,
    flash::{Async as FlashAsync, Flash},
    gpio::{Input, Output},
    i2c::{self, Async as I2CAsync, Config as I2CConfig, InterruptHandler as I2CInterruptHandler},
    multicore::{spawn_core1, Stack},
    peripherals::{I2C0, PIO0, USB},
    usb::{Driver, InterruptHandler as USBInterruptHandler},
};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::{Baseline, Text, TextStyleBuilder},
    Drawable,
};
use panic_probe as _;
use rmk::{
    action::KeyAction,
    channel::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, EVENT_CHANNEL},
    config::{
        BehaviorConfig, ControllerConfig, KeyboardUsbConfig, RmkConfig, StorageConfig, VialConfig,
    },
    debounce::default_debouncer::DefaultDebouncer,
    event::Event,
    futures::future::join4,
    initialize_keymap_and_storage,
    input_device::{InputDevice, Runnable},
    keyboard::Keyboard,
    light::LightController,
    run_devices, run_rmk,
    split::{
        central::{run_peripheral_manager, CentralMatrix},
        rp::uart::{BufferedUart, UartInterruptHandler},
        SPLIT_MESSAGE_MAX_SIZE,
    },
};
use ssd1306::{
    mode::DisplayConfig, prelude::DisplayRotation, size::DisplaySize128x32, I2CDisplayInterface,
    Ssd1306,
};
use static_cell::StaticCell;
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

use crate::{
    keyboard_macros::get_forks,
    keymap::{COLS, ROWS},
};

bind_interrupts!(struct DisplayIrqs {
    I2C0_IRQ => I2CInterruptHandler<I2C0>;
});

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => USBInterruptHandler<USB>;
    PIO0_IRQ_0 => UartInterruptHandler<PIO0>;
});

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const ROW_OFFSET: usize = ROWS;
const COL_OFFSET: usize = 0;

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

static KEY_EVENT_OBSERVER_CHANNEL: Channel<CriticalSectionRawMutex, Event, 16> = Channel::new();

struct KeyEventBridge<T: InputDevice> {
    underlying: T,
}

impl<T: InputDevice> InputDevice for KeyEventBridge<T> {
    async fn read_event(&mut self) -> Event {
        let event = self.underlying.read_event().await;
        if KEY_EVENT_OBSERVER_CHANNEL.is_full() {
            let _ = KEY_EVENT_OBSERVER_CHANNEL.receive().await;
        }
        KEY_EVENT_OBSERVER_CHANNEL.send(event).await;
        event
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("RMK start!");
    // Initialize peripherals
    let p = embassy_rp::init(Default::default());

    let mut i2c0_cfg = i2c::Config::default();
    i2c0_cfg.frequency = 400_000; // 400 kHz = fast mode
    let sda = p.PIN_16;
    let scl = p.PIN_17;
    let i2c0 = i2c::I2c::new_async(p.I2C0, scl, sda, DisplayIrqs, I2CConfig::default());

    // Create the usb driver, from the HAL
    let driver = Driver::new(p.USB, Irqs);

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
    let matrix =
        CentralMatrix::<_, _, _, 0, 0, ROWS, COLS>::new(input_pins, output_pins, debouncer);

    let mut bridge = KeyEventBridge { underlying: matrix };

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| unwrap!(spawner.spawn(core1_task(i2c0))));
        },
    );

    let mut keyboard = Keyboard::new(&keymap);

    // Initialize the light controller
    let mut light_controller: LightController<Output> =
        LightController::new(ControllerConfig::default().light_config);

    // Start
    join4(
        run_devices! ((bridge) => EVENT_CHANNEL),
        keyboard.run(),
        run_peripheral_manager::<ROWS, COLS, ROW_OFFSET, COL_OFFSET, _>(0, uart_receiver),
        run_rmk(
            &keymap,
            driver,
            &mut storage,
            &mut light_controller,
            rmk_config,
        ),
    )
    .await;
}

#[embassy_executor::task]
async fn core1_task(i2c0: embassy_rp::i2c::I2c<'static, I2C0, I2CAsync>) {
    let keymap = keymap::get_default_keymap();
    let interface = I2CDisplayInterface::new(i2c0);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();
    display.clear_buffer();
    display.flush().unwrap();

    // Draw a counter that increments every second
    let style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut text_buffer = rmk::heapless::String::<256>::new();
    let mut layer = 0;

    loop {
        let Event::Key(key_event) = KEY_EVENT_OBSERVER_CHANNEL.receive().await else {
            continue;
        };

        let row = key_event.row as usize;
        let col = key_event.col as usize;

        let key_action = keymap[layer][row][col];

        let action = match key_action {
            KeyAction::Single(action)
            | KeyAction::Tap(action)
            | KeyAction::TapHold(action, _)
            | KeyAction::ModifierTapHold(action, _)
            | KeyAction::OneShot(action)
            | KeyAction::LayerTapHold(action, _)
            | KeyAction::WithModifier(action, _) => action,
            KeyAction::No | KeyAction::Transparent => continue,
        };

        // not accurate, it would be better to use the actual keymap's layer state
        layer = match action {
            rmk::action::Action::LayerOn(activated) if key_event.pressed => {
                layer.bitor(activated as usize)
            }
            rmk::action::Action::LayerOn(deactivated) => layer.bitand(!deactivated as usize),
            rmk::action::Action::LayerOff(deactivated) if key_event.pressed => {
                layer.bitand(!deactivated as usize)
            }
            rmk::action::Action::LayerOff(deactivated) => layer.bitor(deactivated as usize),
            // Toggle only on release
            rmk::action::Action::LayerToggle(toggled) if !key_event.pressed => {
                layer.bitxor(toggled as usize)
            }
            rmk::action::Action::LayerToggleOnly(overriden) if !key_event.pressed => {
                layer.bitxor(overriden as usize).bitand(overriden as usize)
            }
            _ => layer,
        };

        display.clear_buffer();
        text_buffer.clear();

        write!(
            text_buffer,
            "{key_action:?}\nPressed: {}\nLayer: {}",
            key_event.pressed, layer
        )
        .unwrap();

        Text::with_text_style(
            text_buffer.as_str(),
            Point::new(0, 0),
            style,
            TextStyleBuilder::new().baseline(Baseline::Top).build(),
        )
        .draw(&mut display)
        .unwrap();

        display.flush().unwrap();
    }
}
