#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;
mod keyboard_macros;
mod vial;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    flash::{Async, Flash},
    gpio::{Input, Output},
    peripherals::{PIO0, USB},
    usb::{Driver, InterruptHandler},
};
use panic_probe as _;
use rmk::{
    channel::EVENT_CHANNEL,
    config::{
        BehaviorConfig, ControllerConfig, KeyboardUsbConfig, RmkConfig, StorageConfig, VialConfig,
    },
    debounce::default_debouncer::DefaultDebouncer,
    futures::future::join4,
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
use static_cell::StaticCell;
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

use crate::{
    keyboard_macros::get_forks,
    keymap::{COLS, ROWS},
};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
    PIO0_IRQ_0 => UartInterruptHandler<PIO0>;
});

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const ROW_OFFSET: usize = ROWS;
const COL_OFFSET: usize = 0;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("RMK start!");
    // Initialize peripherals
    let p = embassy_rp::init(Default::default());

    // Create the usb driver, from the HAL
    let driver = Driver::new(p.USB, Irqs);

    // Pin config
    let (input_pins, output_pins) = config_matrix_pins_rp!(
        peripherals: p,
        input: [PIN_5, PIN_6, PIN_7, PIN_8, PIN_9],
        output: [PIN_27, PIN_26, PIN_22, PIN_20, PIN_23, PIN_21]
    );

    // Use internal flash to emulate eeprom
    let flash = Flash::<_, Async, FLASH_SIZE>::new(p.FLASH, p.DMA_CH0);

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

    // Start
    join4(
        run_devices! (
            (matrix) => EVENT_CHANNEL,
        ),
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
