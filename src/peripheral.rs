#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Output},
    peripherals::{PIO0, USB},
    usb::InterruptHandler,
};
use panic_probe as _;
use rmk::{
    channel::EVENT_CHANNEL,
    debounce::default_debouncer::DefaultDebouncer,
    futures::future::join,
    matrix::Matrix,
    run_devices,
    split::{
        peripheral::run_rmk_split_peripheral,
        rp::uart::{BufferedUart, UartInterruptHandler},
        SPLIT_MESSAGE_MAX_SIZE,
    },
};
use static_cell::StaticCell;

use crate::keymap::{COLS, ROWS};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
    PIO0_IRQ_0 => UartInterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("RMK start!");
    // Initialize peripherals
    let p = embassy_rp::init(Default::default());

    // Pin config
    let (input_pins, output_pins) = config_matrix_pins_rp!(
        peripherals: p,
        input: [PIN_5, PIN_6, PIN_7, PIN_8, PIN_9],
        output: [PIN_27, PIN_26, PIN_22, PIN_20, PIN_23, PIN_21]
    );

    static RX_BUF: StaticCell<[u8; SPLIT_MESSAGE_MAX_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; SPLIT_MESSAGE_MAX_SIZE])[..];
    let uart_instance = BufferedUart::new_half_duplex(p.PIO0, p.PIN_1, rx_buf, Irqs);

    // Define the matrix
    let debouncer = DefaultDebouncer::<ROWS, COLS>::new();
    let mut matrix = Matrix::<_, _, _, ROWS, COLS>::new(input_pins, output_pins, debouncer);

    // Start
    join(
        run_devices!((matrix) => EVENT_CHANNEL), // Peripheral uses EVENT_CHANNEL to send events to central
        run_rmk_split_peripheral(uart_instance),
    )
    .await;
}
