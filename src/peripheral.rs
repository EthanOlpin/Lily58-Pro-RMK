#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;

mod oled;
use embassy_executor::Spawner;
use embassy_rp::Peripheral;
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Output, Pin, Pull},
    peripherals::{PIO0, USB},
    usb::InterruptHandler,
};
use embassy_time::{block_for, Duration};
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
use ssd1306::prelude::DisplayRotation;
use static_cell::StaticCell;

use crate::keymap::{COLS, ROWS};
use crate::oled::init_oled_terminal;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
    PIO0_IRQ_0 => UartInterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize peripherals
    let p = embassy_rp::init(Default::default());

    unsafe {
        let row4_pin = p.PIN_9.clone_unchecked();
        let col5_pin = p.PIN_21.clone_unchecked();
        if is_key_pressed(row4_pin, col5_pin) {
            embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        }
    }

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

    // Initialize the OLED display
    let mut display =
        init_oled_terminal(p.I2C0, p.PIN_16, p.PIN_17, DisplayRotation::Rotate180).await;
    let _ = display.write_str("Lily58").await;

    join(
        run_devices!((matrix) => EVENT_CHANNEL),
        run_rmk_split_peripheral(uart_instance),
    )
    .await;
}

fn is_key_pressed(
    row_pin: impl Peripheral<P = impl Pin>,
    col_pin: impl Peripheral<P = impl Pin>,
) -> bool {
    let mut row = Output::new(row_pin, embassy_rp::gpio::Level::Low);
    let col = Input::new(col_pin, Pull::Up);

    row.set_low();
    block_for(Duration::from_millis(1));
    !col.is_high()
}
