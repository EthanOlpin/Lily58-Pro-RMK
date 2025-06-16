use core::fmt::Write;
use embassy_rp::{
    i2c::{Async as I2CAsync, I2c},
    peripherals::I2C0,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::{Baseline, Text, TextStyleBuilder},
    Drawable,
};
use rmk::{
    channel::{ControllerSub, CONTROLLER_CHANNEL},
    controller::{Controller, EventController, PollingController},
    event::ControllerEvent,
    futures::future::join,
    heapless::String,
};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
};

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
            ControllerEvent::Key(_key) => {
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

#[embassy_executor::task]
pub async fn oled_controller_task(i2c0: embassy_rp::i2c::I2c<'static, I2C0, I2CAsync>) {
    let mut state_controller = StateController::new();
    let mut display_controller = DisplayController::new(i2c0);
    join(
        state_controller.event_loop(),
        display_controller.polling_loop(),
    )
    .await;
}
