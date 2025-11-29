#![no_main]
#![no_std]

#[macro_use]
mod keymap;
#[macro_use]
mod macros;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Output, Pin, Pull};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::usb::InterruptHandler;
use embassy_rp::{bind_interrupts, Peri};
use embassy_time::Duration;
use panic_probe as _;
use rmk::channel::EVENT_CHANNEL;
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::futures::future::join;
use rmk::matrix::Matrix;
use rmk::run_devices;
use rmk::split::peripheral::run_rmk_split_peripheral;
use rmk::split::rp::uart::{BufferedUart, UartInterruptHandler};
use rmk::split::SPLIT_MESSAGE_MAX_SIZE;
use static_cell::StaticCell;

use crate::keymap::{COLS, ROWS};

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

    let (row_pins, col_pins) = config_matrix_pins_rp!(
        peripherals: p,
        input: [PIN_5, PIN_6, PIN_7, PIN_8, PIN_9],
        output: [PIN_27, PIN_26, PIN_22, PIN_20, PIN_23, PIN_21]
    );

    static RX_BUF: StaticCell<[u8; SPLIT_MESSAGE_MAX_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; SPLIT_MESSAGE_MAX_SIZE])[..];
    let uart_instance = BufferedUart::new_half_duplex(p.PIO0, p.PIN_1, rx_buf, Irqs);

    // Define the matrix
    let debouncer = DefaultDebouncer::<ROWS, COLS>::new();
    let mut matrix = Matrix::<_, _, _, ROWS, COLS, true>::new(row_pins, col_pins, debouncer);

    // Initialize the OLED display
    // let mut display =
    //     init_oled_terminal(p.I2C0, p.PIN_16, p.PIN_17, DisplayRotation::Rotate180).await;
    // let _ = display.write_str("Lily58").await;

    join(
        run_devices!((matrix) => EVENT_CHANNEL),
        run_rmk_split_peripheral(uart_instance),
    )
    .await;
}

fn is_key_pressed<R: Pin, C: Pin>(row_pin: Peri<R>, col_pin: Peri<C>) -> bool {
    let mut row = Output::new(row_pin, embassy_rp::gpio::Level::Low);
    let col = Input::new(col_pin, Pull::Up);

    row.set_low();
    embassy_time::block_for(Duration::from_millis(1));
    !col.is_high()
}
