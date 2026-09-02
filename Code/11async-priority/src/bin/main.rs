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
    interrupt::{software::SoftwareInterruptControl, Priority},
    timer::timg::TimerGroup,
    delay::Delay,
};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_rtos::embassy::InterruptExecutor;
use static_cell::StaticCell;

use log::info;

use esp_backtrace as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// 声明一个高优先级异步任务，使用 embassy_executor::task 属性标记
#[embassy_executor::task]
async fn high_prio_task() {
    loop {
        info!("High priority task running!");
        Timer::after(Duration::from_millis(1000)).await;
    }
}

// 声明一个低优先级异步任务，通过阻塞等待模拟其运行
#[embassy_executor::task]
async fn low_prio_task() {
    let delay = Delay::new();
    loop {
        info!("Low priority task running!");
        delay.delay_millis(3000);
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(low_prio_spawner: Spawner){
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

    // 定时器0作为embassy的时间源，软件中断驱动任务调度
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // 启动异步调度
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // 创建一个中断执行器，用于处理高优先级任务
    // StaticCell 用于在静态存储区存放运行时创建的中断执行器实例
    static EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();
    let executor = InterruptExecutor::new(sw_interrupt.software_interrupt2);
    let executor = EXECUTOR.init(executor);

    // 将高优先级调度器挂载到中断执行器上
    let high_prio_spawner = executor.start(Priority::Priority3);
    
    // 挂载高优先级任务到高优先级调度器
    high_prio_spawner.spawn(high_prio_task().expect("Failed to spawn high priority task"));

    // 挂载低优先级任务到低优先级调度器
    low_prio_spawner.spawn(low_prio_task().expect("Failed to spawn low priority task"));

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}