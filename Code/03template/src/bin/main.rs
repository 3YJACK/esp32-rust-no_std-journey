// no_std 和 no_main 表示这是一个“没有标准库、没有默认 main 入口”的嵌入式程序。
#![no_std]
#![no_main]

// 这里的 #![deny(...)] 表示“把某些不安全/不推荐的写法当成错误来处理”。
// 这样做的好处是：编译器会更严格地提醒你，避免在嵌入式程序里写出容易出问题的代码。
//
// clippy::mem_forget：提醒你不要随便用 mem::forget，
// 因为它可能让某些包含缓冲区的硬件对象被“忘记释放/清理”，从而导致问题。
//
// clippy::large_stack_frames：提醒你不要在 main 函数里放太大的栈数据，嵌入式设备的栈空间通常很有限。
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{
    clock::CpuClock,
    main,
    time::{Duration, Instant},
};

use log::info;

// 引入 esp_backtrace 库，它可以在程序崩溃时提供回溯信息，帮助调试。通过 as _ 表示不绑定可用名称，即不在代码中直接使用该库。
use esp_backtrace as _;

// 引入alloc库，用于动态内存分配
extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// 对于mian函数，通过#[allow]来允许使用大型栈数据以通过编译
#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
//  #[main] 是 esp_hal 提供的一个过程宏属性，作用是为 no_std/no_main 程序生成真正的入口点。
#[main]
fn main() -> ! {    // ！表示这个函数永远不会返回
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o esp32s3-wroom-1-octal-psram -o unstable-hal -o alloc -o stack-smashing-protection -o log -o esp-backtrace -o vscode

    // 初始化 log 日志系统为 esp_println::logger 
    esp_println::logger::init_logger_from_env();

    // 初始化芯片配置，设置 CPU 时钟为最大频率
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO3
    // - GPIO45
    // - GPIO46
    // These GPIO pins are in use by some feature of the module and should not be used.
    // 丢弃不可用的 GPIO 引脚所有权避免后续被使用
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

    // 初始化全局堆分配器，分配一块大小为 73744 字节的堆内存（放在由 esp_hal::ram(reclaimed) 指定的 RAM 区域）
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    loop {
        // 使用 log crate 的 info! 宏打印日志信息，非标准库情况下无法使用 println!    
        info!("Hello world!");
        
        // 获取此刻时间值给 delay_start 
        let delay_start = Instant::now();
        // elapsed() 返回从 delay_start 到现在的时间间隔，Duration::from_millis(500) 创建一个500毫秒的时间间隔对象。
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
