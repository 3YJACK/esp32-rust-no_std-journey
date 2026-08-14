#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;
use critical_section::Mutex;

use esp_hal::{
    main,
    handler,
    ram,
    clock::CpuClock,
    gpio::*,
    time::*,
    timer::{*, timg::*},
    // ledc::*,
};

use esp_backtrace as _;
use log::info;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

static LED: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o esp32s3-wroom-1-octal-psram -o unstable-hal -o alloc -o stack-smashing-protection -o log -o esp-backtrace -o vscode

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

    // 先获取一个定时器组
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    // 再将一个定时器组的通用定时器实例化为周期型定时器
    let mut prd_timer = PeriodicTimer::new(timg0.timer0);
    // 设置中断处理程序为 timer_handler 函数
    prd_timer.set_interrupt_handler(timer_handler);
    // 监听定时器中断
    prd_timer.listen();
    // 设置定时器周期为 1000 ms并启动定时器
    prd_timer.start(Duration::from_millis(1000));

    // 初始化 LED1 用于定时器中断控制其闪烁
    let mut led1 = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());
    critical_section::with(|cs| {
        LED.borrow_ref_mut(cs)
            .replace(led1);
    });

    // // 2) PWM：另一个 LED 作为呼吸灯
    // let mut ledc = Ledc::new(peripherals.LEDC);
    // ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    // let mut pwm_timer = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    // pwm_timer
    //     .configure(timer::config::Config {
    //         duty: timer::config::Duty::Duty8Bit,
    //         clock_source: timer::LSClockSource::APBClk,
    //         frequency: Rate::from_hz(1000),
    //     })
    //     .unwrap();

    // let mut pwm_led = ledc.channel(channel::Number::Channel0, peripherals.GPIO4);
    // pwm_led
    //     .configure(channel::config::Config {
    //         timer: &pwm_timer,
    //         duty_pct: 0,
    //         drive_mode: DriveMode::PushPull,
    //     })
    //     .unwrap();

    loop {
        // prd_timer.wait();
        // blink_led.toggle();

        // pwm_led.start_duty_fade(0, 100, 1000).unwrap();
        // while pwm_led.is_duty_fade_running() {}

        // pwm_led.start_duty_fade(100, 0, 1000).unwrap();
        // while pwm_led.is_duty_fade_running() {}
        info!("PROGRAM RUNNING...");
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}

#[handler]
#[ram]
fn timer_handler() {
    critical_section::with(|cs| {
        LED.borrow_ref_mut(cs)
            .as_mut()
            .expect("LED not initialized")
            .toggle();
    });
}