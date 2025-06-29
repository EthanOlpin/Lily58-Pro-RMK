use embassy_rp::{
    bind_interrupts,
    i2c::{self, Async, I2c, SclPin, SdaPin},
    peripherals::I2C0,
    Peripheral,
};
use ssd1306::{
    mode::{BasicMode, BufferedGraphicsModeAsync, DisplayConfigAsync, TerminalModeAsync},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306Async,
};

const DISPLAY_SIZE: DisplaySize128x32 = DisplaySize128x32;
type DisplayInterface = I2CInterface<I2c<'static, I2C0, Async>>;
pub type Oled<Mode> = Ssd1306Async<DisplayInterface, DisplaySize128x32, Mode>;

bind_interrupts!(struct DisplayIrqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

pub fn init_oled(
    i2c0: I2C0,
    sda: impl Peripheral<P = impl SdaPin<I2C0>> + 'static,
    scl: impl Peripheral<P = impl SclPin<I2C0>> + 'static,
    rotation: DisplayRotation,
) -> Oled<BasicMode> {
    let mut i2c0_cfg = i2c::Config::default();
    i2c0_cfg.frequency = 400_000; // 400 kHz = fast mode
    let i2c = I2c::new_async(i2c0, scl, sda, DisplayIrqs, i2c0_cfg);
    let interface = I2CDisplayInterface::new(i2c);
    Ssd1306Async::new(interface, DISPLAY_SIZE, rotation)
}

#[allow(dead_code)]
pub async fn init_oled_terminal(
    i2c0: I2C0,
    sda: impl Peripheral<P = impl SdaPin<I2C0>> + 'static,
    scl: impl Peripheral<P = impl SclPin<I2C0>> + 'static,
    rotation: DisplayRotation,
) -> Oled<TerminalModeAsync> {
    let mut display = init_oled(i2c0, sda, scl, rotation).into_terminal_mode();
    display.init().await.unwrap();
    display.clear().await.unwrap();
    display
}

#[allow(dead_code)]
pub async fn init_oled_graphics(
    i2c0: I2C0,
    sda: impl Peripheral<P = impl SdaPin<I2C0>> + 'static,
    scl: impl Peripheral<P = impl SclPin<I2C0>> + 'static,
    rotation: DisplayRotation,
) -> Oled<BufferedGraphicsModeAsync<DisplaySize128x32>> {
    let mut display = init_oled(i2c0, sda, scl, rotation).into_buffered_graphics_mode();
    display.init().await.unwrap();
    display.clear_buffer();
    display.flush().await.unwrap();
    display
}
