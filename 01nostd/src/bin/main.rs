// 这里的 no_std 和 no_main 表示这是一个“没有标准库、没有默认 main 入口”的嵌入式程序。
// 对初学者来说，可以把它理解为：这个程序要直接跑在芯片上，而不是普通电脑上运行的 Linux/Windows 程序。
#![no_std]
#![no_main]

// 这里的 #![deny(...)] 表示“把某些不安全/不推荐的写法当成错误来处理”。
// 这样做的好处是：编译器会更严格地提醒你，避免在嵌入式程序里写出容易出问题的代码。
//
// clippy::mem_forget：提醒你不要随便用 mem::forget，
// 因为它可能让某些包含缓冲区的硬件对象被“忘记释放/清理”，从而导致问题。
//
// clippy::large_stack_frames：提醒你不要在 main 函数里放太大的栈帧，
// 因为嵌入式设备的栈空间通常很小。
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use log::info;

use esp_backtrace as _;

// alloc 允许在嵌入式程序里使用堆内存分配。
extern crate alloc;

// 这个宏会生成一个默认的应用描述信息，供 ESP-IDF bootloader 使用。
// 你现在可以先把它当成“让程序能被引导加载器识别”的固定写法。
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // 这里是由生成器自动生成的版本和参数说明，暂时不用关心具体内容。
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o esp32s3-wroom-1-octal-psram -o unstable-hal -o alloc -o embassy -o stack-smashing-protection -o vscode -o esp-backtrace -o log

    // 初始化日志系统，这样程序运行时可以把信息打印出来。
    esp_println::logger::init_logger_from_env();

    // 设置 CPU 时钟频率为最大值，并初始化硬件外设。
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // 这些 GPIO 引脚在某些模块上是“保留”或“被系统占用”的，
    // 这里先把它们拿出来，避免后面误用。
    // 你可以把它们理解为：先声明“这些引脚我会用到/我知道它们存在”。
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

    // 为堆分配器配置一块可用的 RAM 空间。
    // 这一步让程序可以在嵌入式环境里安全地使用 alloc 相关功能。
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    // 创建定时器组和软件中断控制器，随后启动 RTOS 运行环境。
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    // 输出一条日志，确认 Embassy 初始化已经完成。
    info!("Embassy initialized!");

    // 这里先把 spawner 变量“用一下”，避免编译器警告。
    // 以后你可以用它来创建异步任务。
    let _ = spawner;

    // 无限循环：每隔 1 秒打印一次 Hello world!
    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // 如果你想继续学习，可以参考这里的示例：
    // https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
