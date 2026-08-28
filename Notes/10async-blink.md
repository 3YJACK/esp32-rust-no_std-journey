# 学习目标

使用`esp-generate`创建工程（创建时需注意开启`embassy`异步框架）并参考`esp-rs/esp-hal`仓库的`./example/async/multicore`示例，编写代码并实现串口日志输出和LED灯每秒翻转电平的并发运行。

# 完整源码

```rust
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
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
    gpio::{Output, Level, OutputConfig},
};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use log::info;

use esp_backtrace as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// 声明一个异步任务，使用 embassy_executor::task 属性标记
#[embassy_executor::task]
async fn blink(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(1000)).await;
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

    let led = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());

    // 定时器0作为embassy的时间源，软件中断驱动任务调度
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // 启动异步调度
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // 创建异步任务blink并将其加入到执行器的调度队列中
    spawner.spawn(blink(led).expect("Failed to spawn blink task"));

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
```

**引脚连接参照表：**

| 外设   | 对应引脚  |
| ---- | ----- |
| LED灯 | GPIO4 |

# 烧录运行

使用下列命令进行编译：

```powershell
cargo build 
```

使用下列命令进行烧录运行：

```powershell
cargo espflash flash --monitor
```

**预期效果：**

烧录运行程序后，串口日志每秒输出一次'Hello world!"，同时LED灯每秒翻转一次电平，以一秒为周期进行闪烁。两个任务互不阻塞，异步运行。

# 代码讲解

## esp_rtos

示例源码中出现的`esp_rtos`并不是指freertos之类的实时操作系统，而是专门为 `esp-hal` 在异步框架下提供运行时支持的**任务调度器**，是为了支持`embassy`框架在esp平台运行的集成组件，其本身并不提供类似于任务创建这种传统rtos的API。

## embassy

异步调度的初始化如下所示，需要一个定时器作为时间源和一个软件中断用于任务调度。

```rust
    // 定时器0作为embassy的时间源，软件中断驱动任务调度
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // 启动异步调度
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
```

异步任务的创建如下所示，用`#[embassy_executor::task]`属性声明异步任务，再调用`spawner`将该任务加入执行器进行调度。

```rust
#[embassy_executor::task]
async fn blink(mut led: Output<'static>) {
    // .......
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // ......
    
    // 创建异步任务blink并将其加入到执行器的调度队列中
    spawner.spawn(blink(led).expect("Failed to spawn blink task"));
    
    // ......
}
```

本篇示例中，两个异步任务的运行流程如下，每执行一次就挂起一秒，当定时器到期后才进行唤醒，然后触发软件中断并由执行器进行调度再次运行。两个任务独立运行，互不阻塞。

```
main:
    输出 Hello world!
    等待 1 秒，暂时挂起

blink:
    翻转 GPIO4
    等待 1 秒，暂时挂起

1 秒后:
    main 被唤醒，输出日志
    blink 被唤醒，翻转 LED
    两者再次进入等待
```

## 'static生命周期

在异步任务blink中，传入的参数是`mut led: Output<'static>`，如果改为`mut led: Output`，则会编译报错。其中，`'static`是对生命周期的表示，表明`led`会存活于程序的整个生命周期。

要求任务参数具有 `'static` 生命周期，并非Rust语言的强制规定，而是 `Embassy` **执行器设计的一个核心且必要的特性**。

在程序启动时，`main` 函数会创建任务并将其加入调度。但是，**`main` 函数本身是会返回的**，其内部的局部变量也会随之失效。然而，被生成的任务却会可能仍继续在后台独立运行。这意味着，**任务的生命周期完全独立于生成它的上下文**。

如果任务持有对 `main` 函数中某个局部变量的引用，当 `main` 返回后，该变量将被销毁，任务却依然存活，这就造成了一个**悬垂引用（dangling reference）**，这在 Rust 中是严格禁止的内存安全问题。

在语义移动的情况下，这种问题不会发生。但是在借用的情况下，则有悬垂引用的风险。

因此`embassy`要求任务函数都具有 `'static` 生命周期。这确保了**任务所拥有的任何数据都不会在其执行期间被销毁。**

## 可变性的"绑定"属性

可能会有人会疑惑，`led`定义时是不可变的，为什么可以传入要求可变值作为参数的函数？

这是在学习rust时很容易混淆的一个概念。其核心在于，**可变性是一个绑定的外在属性，而非值本身固有的内在属性**。

`led`在主函数定义时，绑定的是不可变属性，因此在主函数内部是不可以进行可变操作的。但是当它发生语义移动，传入blink任务时，旧的不可变属性就"解绑"了，它在blink函数内部可以被重新声明为可变的。**每当一个值发生语义移动时，都能对其可变性进行重新声明**。但是在借用的情况下，则会有严格的规则限制。


