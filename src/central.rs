#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;
mod keyboard_macros;
mod vial;
use core::fmt::Write;
use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::{Executor, Spawner};
use embassy_rp::{
    bind_interrupts,
    flash::{Async as FlashAsync, Flash},
    gpio::{Input, Output},
    i2c::{
        self, Async as I2CAsync, Config as I2CConfig, I2c, InterruptHandler as I2CInterruptHandler,
    },
    multicore::{spawn_core1, Stack},
    peripherals::{I2C0, PIO0, USB},
    usb::{Driver, InterruptHandler as USBInterruptHandler},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::{Baseline, Text, TextStyleBuilder},
    Drawable,
};
use panic_probe as _;
use rmk::{
    channel::{ControllerSub, CONTROLLER_CHANNEL, EVENT_CHANNEL},
    config::{
        BehaviorConfig, ControllerConfig, KeyboardUsbConfig, RmkConfig, StorageConfig, VialConfig,
    },
    controller::{Controller, EventController, PollingController},
    debounce::default_debouncer::DefaultDebouncer,
    event::ControllerEvent,
    futures::future::{join, join4},
    heapless::String,
    initialize_keymap_and_storage,
    input_device::Runnable,
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
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
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

#[derive(Clone)]
struct State {
    layer: usize,
    key_event_count: usize,
}

static STATE_SIGNAL: Signal<CriticalSectionRawMutex, State> = Signal::new();

struct StateController<'a> {
    state: State,
    subscriber: ControllerSub<'a>,
}

impl<'a> StateController<'a> {
    fn new() -> Self {
        let subscriber = CONTROLLER_CHANNEL.subscriber().unwrap();
        Self {
            state: State {
                layer: 0,
                key_event_count: 0,
            },
            subscriber,
        }
    }
}

impl<'a> Controller for StateController<'a> {
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        match event {
            ControllerEvent::Layer(layer) => {
                self.state.layer = layer as usize;
                STATE_SIGNAL.signal(self.state.clone());
            }
            ControllerEvent::Key(key) => {
                self.state.key_event_count += 1;
                STATE_SIGNAL.signal(self.state.clone());
            }
            _ => {}
        }
    }

    async fn next_message(&mut self) -> Self::Event {
        self.subscriber.next_message_pure().await
    }
}

impl<'a> EventController for StateController<'a> {}

struct DisplayController<'a> {
    display: Ssd1306<
        I2CInterface<I2c<'a, I2C0, I2CAsync>>,
        DisplaySize128x32,
        BufferedGraphicsMode<DisplaySize128x32>,
    >,
    text_buffer: String<128>,
}

impl<'a> DisplayController<'a> {
    fn new(i2c0: I2c<'a, I2C0, I2CAsync>) -> Self {
        let interface = I2CDisplayInterface::new(i2c0);
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate90)
            .into_buffered_graphics_mode();

        display.init().unwrap();
        display.clear_buffer();
        display.flush().unwrap();

        Self {
            display,
            text_buffer: String::new(),
        }
    }
}

impl<'a> Controller for DisplayController<'a> {
    type Event = State;

    async fn process_event(&mut self, event: Self::Event) {
        self.text_buffer.clear();
        write!(
            self.text_buffer,
            "{}\n{}",
            event.layer, event.key_event_count
        )
        .unwrap();
    }

    async fn next_message(&mut self) -> Self::Event {
        STATE_SIGNAL.wait().await
    }
}

impl<'a> PollingController for DisplayController<'a> {
    const INTERVAL: embassy_time::Duration = embassy_time::Duration::from_hz(30);

    async fn update(&mut self) {
        self.display.clear_buffer();

        Text::with_text_style(
            &self.text_buffer,
            Point::new(0, 0),
            MonoTextStyleBuilder::new()
                .font(&FONT_10X20)
                .text_color(BinaryColor::On)
                .build(),
            TextStyleBuilder::new()
                .alignment(embedded_graphics::text::Alignment::Left)
                .baseline(Baseline::Top)
                .build(),
        )
        .draw(&mut self.display)
        .unwrap();

        self.display.flush().unwrap();
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
    let mut matrix =
        CentralMatrix::<_, _, _, 0, 0, ROWS, COLS>::new(input_pins, output_pins, debouncer);

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                unwrap!(spawner.spawn(oled_controller_task(i2c0)));
            });
        },
    );

    let mut keyboard = Keyboard::new(&keymap);

    // Initialize the light controller
    let mut light_controller: LightController<Output> =
        LightController::new(ControllerConfig::default().light_config);

    // Start
    join4(
        run_devices! ((matrix) => EVENT_CHANNEL),
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
async fn oled_controller_task(i2c0: embassy_rp::i2c::I2c<'static, I2C0, I2CAsync>) {
    let mut state_controller = StateController::new();
    let mut display_controller = DisplayController::new(i2c0);
    join(
        state_controller.event_loop(),
        display_controller.polling_loop(),
    )
    .await;
}
