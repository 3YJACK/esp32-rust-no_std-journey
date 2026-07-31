根据上篇笔记`02工程创建`使用`esp-generate`按以下配置创建一个简单工程模板。

```powershell
 ✅ Enable unstable HAL features. 
 ✅ Enable allocations via the esp-alloc crate. 
 ✅ Enable stack smashing protection.  
 Flashing, logging and debugging (espflash)
     ✅ Use the log crate to print messages. 
     ✅ Use esp-backtrace as the panic handler. 
 Optional editor integration 
     ✅ Add settings for Visual Studio Code        
```

# 完整源码

下面为带了个人注释笔记的工程模板源码（相应的工程文件放在`./Code`）：

```rust
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
```

# 烧录运行

使用下列命令进行编译：

```powershell
cargo build 
```

使用下列命令进行烧录运行：

```powershell
cargo espflash flash --monitor
```

`--monitor`在烧录完成后自动打开串口监视器查看日志输出。

**预期运行效果：**

# 代码讲解

## 属性

在模板代码的头部，可以看到几行`#![]`包围的代码，在 Rust 中，`#[]` 和 `#![]` 统称为**属性（Attributes）**。

可以把它们理解为**写给编译器的“特殊指令”或“元数据”**，用来告诉编译器对代码进行特定的处理，比如条件编译、自动生成代码、设置入口点等。

两者的区别在于**作用范围**不同：

- **`#[]`**：**外部属性**——放在修饰区域的外部，修饰它正下方紧挨着的那个代码块（函数、结构体、变量等）。

- **`#![]`**：**内部属性**——放在修饰区域的外部，修饰它所在的整个文件、模块或 Crate（软件包）。

源码开头的内部属性，就是作用在整个文件中，声明裸机嵌入式环境，禁止随意使用大型栈数据和`mem::forget`（`mem::forget`会让 Rust 跳过该对象的析构函数（Drop），也就是“忘记”释放该对象占用的资源）。

```rust
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
```

而在`mian`函数前面，又使用了外部属性，也就是仅作用于`main`函数的局部允许使用大型栈数据，不会触发编译器警告。原因在代码中也给出：

> it's not unusual to allocate larger buffers etc. in main
> 
> 在 main 函数里分配较大的缓冲区是很常见的。

```rust
// 对于mian函数，通过#[allow]来允许使用大型栈数据以通过编译
#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
//  #[main] 是 esp_hal 提供的一个过程宏属性，作用是为 no_std/no_main 程序生成真正的入口点。
#[main]
fn main() -> ! {
}
```

## 日志打印

`log`crate是rust社区中广泛使用的日志工具之一。

源码中`esp_println`提供了日志工具的具体实现，初始化了日志系统并让ESP的logger成为`log`日志系统的后端，这样使用`info!`打印宏等日志消息都会通过ESP的logger进行输出（通常是输出到串口）。

```rust
use log::info;

// 初始化 log 日志系统为 esp_println::logger 
esp_println::logger::init_logger_from_env();
```
