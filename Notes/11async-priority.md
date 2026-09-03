# 学习目标

使用`esp-generate`创建工程(注意开启embassy异步框架)，并参考`esp-rs/esp-hal`仓库的`./example/async/embassy_multiprio`示例，编写代码并完成优先级抢占示例。

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

    // 创建一个中断执行器，用于处理高优先级任务，StaticCell 用于静态初始化
    static EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();
    let executor = InterruptExecutor::new(sw_interrupt.software_interrupt2);
    let executor = EXECUTOR.init(executor);

    // 设置优先级并启动任务调度
    let high_prio_spawner = executor.start(Priority::Priority3);
    
    // 挂载高优先级任务到高优先级执行器的调度队列
    high_prio_spawner.spawn(high_prio_task().expect("Failed to spawn high priority task"));

    // 挂载低优先级任务到低优先级执行器的调度队列
    low_prio_spawner.spawn(low_prio_task().expect("Failed to spawn low priority task"));

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

**预期效果：**

# 代码讲解

# 任务优先级

rust的异步框架中也有任务优先级的划分，但是实现方式和传统的 RTOS不太一样。Embassy 的优先级机制，本质上是**通过创建多个不同优先级的执行器（Executor）实例来完成的**。

- **一个执行器，一个优先级**：每个执行器实例都被赋予一个固定的优先级。

- **高优先级抢占低优先级**：高优先级的执行器可以**抢占**正在运行的低优先级执行器。

- **同执行器内，任务公平轮转**：在同一个执行器内部，所有任务依然是**协作式**调度的，大家轮流运行，不会有一个任务独占 CPU。

为了实现这种抢占，Embassy 提供了两种主要的执行器类型：

- **`Executor`（线程模式）**：运行在最低优先级，通常在后台处理常规任务。

- **`InterruptExecutor`（中断模式）**：运行在更高的中断优先级上，能抢占 `Executor`。甚至可以创建多个 `InterruptExecutor`，并分配不同的中断优先级，实现更精细的优先级控制。

在 Embassy 的优先级设计中，**由 `#[esp_rtos::main]` 创建的默认执行器是`Executor`，其优先级是最低的。**

因此高优先级任务调度的实现如下：

**创建中断执行器→设置优先级并挂载相应的调度器→挂载对应优先级的任务以加入调度**

```rust
async fn main(low_prio_spawner: Spawner){
    // 创建一个中断执行器，用于处理高优先级任务，StaticCell 用于静态初始化
    static EXECUTOR: StaticCell<InterruptExecutor<2>> = StaticCell::new();
    let executor = InterruptExecutor::new(sw_interrupt.software_interrupt2);
    let executor = EXECUTOR.init(executor);

    // 设置优先级并启动任务调度
    let high_prio_spawner = executor.start(Priority::Priority3);
    
    // 挂载高优先级任务到高优先级执行器的调度队列
    high_prio_spawner.spawn(high_prio_task().expect("Failed to spawn high priority task"));

    // 挂载低优先级任务到低优先级执行器的调度队列
    low_prio_spawner.spawn(low_prio_task().expect("Failed to spawn low priority task"));
}
```

 需要注意的是，**Embassy 并没有设计优先级继承的机制来避免优先级反转问题**，因此在开发时需主动通过代码设计来避免这个问题。

## StaticCell

示例源码中，中断执行器的创建使用到了`StaticCell`作为一个静态变量的延迟初始化容器。

**`StaticCell`是专为静态初始化而生的容器**，它在编译时只占用内存空间，在运行时才填充具体的值，在初始化之后就无法再进行初始化，否则会导致系统崩溃(仅针对于`StaticCell`这一容器而言，容器内部的数据还是可以修改的)。

这听起来很类似于我们之前学习过的用`Option`做占位再替换为具体的值的延迟初始化方法，但不同的是这个方法并不是专门用于静态初始化的，它更针对于“在程序启动后，**仍可能需要替换或修改**的共享资源”的场景。

**如何选择？**

- **`StaticCell`**：直接给你一个`&mut T`，修改时没有任何运行时检查开销，且这个引用是整个程序唯一的，资源被独占，它只适用于“单线程/单任务”上下文。

- **`Mutex<RefCell<Option<T>>>`**：你获取修改权限时必须经过**运行时检查**（`.borrow_mut()` 检查是否已被借用，`.lock()` 检查是否被其他任务抢占）。资源可以共享，它适用于“多任务并发”上下文。


