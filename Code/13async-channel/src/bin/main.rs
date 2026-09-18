#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{
    clock::CpuClock,
    timer::timg::TimerGroup,
    gpio::{Input, InputConfig, Pull},
};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_sync::{     // 导入emabssy同步通信模块
    blocking_mutex::raw::CriticalSectionRawMutex, 
    channel::{Channel, Receiver, Sender},
};

use log::info;

use esp_backtrace as _;

extern crate alloc;


// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const CHANNEL_CAPACITY: usize = 10;

#[embassy_executor::task]
async fn print_task(
    mut channel_receiver: Receiver<'static, CriticalSectionRawMutex, &'static str, CHANNEL_CAPACITY>,
) {
    loop {
        let msg = channel_receiver.receive().await;
        info!("{}", msg);
    }
}

#[embassy_executor::task]
async fn button_sender(
    button: Input<'static>,
    channel_sender: Sender<'static,CriticalSectionRawMutex, &'static str, CHANNEL_CAPACITY>,
) {
    loop {
        if button.is_low() {
            channel_sender.send("message from sender1").await;
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o esp32s3-wroom-1-octal-psram -o unstable-hal -o alloc -o embassy -o stack-smashing-protection -o log -o esp-backtrace -o vscode

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO3
    // - GPIO45
    // - GPIO46
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;
    let _ = peripherals.GPIO31;
    let _ = peripherals.GPIO32;
    let _ = peripherals.GPIO33;
    let _ = peripherals.GPIO34;
    let _ = peripherals.GPIO35;
    let _ = peripherals.GPIO36;
    let _ = peripherals.GPIO37;


    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    static CHANNEL: Channel<CriticalSectionRawMutex, &'static str, CHANNEL_CAPACITY> = Channel::new();
    
    let channel_sender0 = CHANNEL.sender();
    let channel_sender1 = CHANNEL.sender();
    let channel_receiver = CHANNEL.receiver();

    let config = InputConfig::default().with_pull(Pull::Up);
    let button = Input::new(peripherals.GPIO6, config);

    spawner.spawn(print_task(channel_receiver).expect("Failed to spawn print task"));
    spawner.spawn(button_sender(button, channel_sender1).expect("Failed to spawn button sender"));

    loop {
        channel_sender0.send("message from sender 0").await;
        Timer::after(Duration::from_millis(2000)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
