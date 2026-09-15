# 学习目标

使用`esp-generate`创建工程(注意启用embassy异步框架)，并参考`esp-rs/esp-hal`仓库的`./example/async/embassy_multicore`示例和embassy同步通信的官方文档[embassy_sync - Rust](https://docs.rs/embassy-sync/latest/embassy_sync/)，编写代码并完成用`Singal`与LED0进行一对一同步和用`Watch`与LED1和串口打印进行一对多同步的通信示例。

需先在终端中通过下面命令添加依赖，才能导入emabssy的同步通信模块：

```powershell
cargo add embassy-sync
```

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
    interrupt::{software::SoftwareInterruptControl},
    timer::timg::TimerGroup,
    gpio::{Output, Level, OutputConfig, Input, InputConfig, Pull},
};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_sync::{     // 导入emabssy同步通信模块
    blocking_mutex::raw::CriticalSectionRawMutex, 
    signal::Signal, 
    watch::{self, Watch},
};

use static_cell::StaticCell;

use log::info;

use esp_backtrace as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const WATCH_RCV_NUM: usize = 2;

#[embassy_executor::task]   // 创建按键监控任务，作为watch的发送者，按键按下时发送信号
async fn button_monitor(
    button: Input<'static>,
    watch_send: watch::Sender<'static, CriticalSectionRawMutex, bool, WATCH_RCV_NUM>,
) {
    loop {
        if button.is_low() {
            watch_send.send(true);
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]   // 创建watch的接收任务，按键按下时打印信息
async fn rcv0_print(
    mut watch_rev0: watch::Receiver<'static, CriticalSectionRawMutex, bool, WATCH_RCV_NUM>,
) {
    loop {
        if watch_rev0.changed().await { 
            info!("button pressed!");
        }
    }
}

#[embassy_executor::task]   // 创建watch的接收任务，按键按下时翻转LED电平
async fn rcv1_led(
    mut led1: Output<'static>,
    mut watch_rev1: watch::Receiver<'static, CriticalSectionRawMutex, bool, WATCH_RCV_NUM>
){
    loop {
        if watch_rev1.changed().await {
            led1.toggle();
        }
    }
}


#[embassy_executor::task]   // 创建signal接收任务，等待信号以控制LED的开关
async fn led_control(
    mut led: Output<'static>,
    ctrl_signal: &'static Signal<CriticalSectionRawMutex, bool>,
) {
    loop {
        // 等待信号量，直到有任务发出信号
        if ctrl_signal.wait().await {
            led.set_high();
        }
        else {
            led.set_low();
        }
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner){
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
    let sw_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    let led0 = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());

    // 创建一个信号，用于控制LED的开关
    static LED_CTRL_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, bool>> = StaticCell::new();
    let ctrl_signal = LED_CTRL_SIGNAL.init(Signal::new());

    spawner.spawn(led_control(led0, ctrl_signal).expect("Failed to spawn led_control task"));

    let config = InputConfig::default().with_pull(Pull::Up);
    let button = Input::new(peripherals.GPIO6, config);

    let led1 = Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default());

    // 创建一个 watch，用于监控按钮的状态
    static WATCH: Watch<CriticalSectionRawMutex, bool, WATCH_RCV_NUM> = Watch::new();

    let  watch_send = WATCH.sender();
    let  watch_rev0 = WATCH.receiver().expect("Failed to create watch receiver 0");
    let  watch_rev1 = WATCH.receiver().expect("Failed to create watch receiver 1");

    spawner.spawn(button_monitor(button, watch_send).expect("Failed to spawn button_monitor task"));
    spawner.spawn(rcv0_print(watch_rev0).expect("Failed to spawn rcv0_print task"));
    spawner.spawn(rcv1_led(led1, watch_rev1).expect("Failed to spawn rcv1_led task"));

    loop {
        // 每隔1秒发送一次信号，控制LED的开关
        ctrl_signal.signal(true);
        Timer::after(Duration::from_millis(1000)).await;
        ctrl_signal.signal(false);
        Timer::after(Duration::from_millis(1000)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
```

**引脚连接参照表：**

| 外设   | 对应引脚  |
| ---- | ----- |
| LED0 | GPIO4 |
| 按键   | GPIO6 |
| LED1 | GPIO7 |

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

LED0每秒翻转一次电平，主循环任务每秒发送一次`Signal`控制LED0周期性亮灭；按键按下时串口打印日志信息且LED1电平翻转，`button_monitor`任务发送`Watch`给串口打印任务和LED1控制任务，分别进行串口打印和LED1的亮灭控制。

# 代码讲解

## Signal-信号

`Signal`是用于**一对一传递单个数据**的同步通信方式，进行发送操作时新值会覆盖旧值，适合接收方只关心最新值的状态同步任务。

`Signal` 的定义如下：

```rust
pub struct Signal<M, T>
where
    M: RawMutex,
{ /* private fields */ }
```

- **`T`：这是 `Signal` 携带的数据类型**。只能携带一个数据且**只保留最新的值**。

- **`M`：实现同步的“锁”**：这是一个类型参数，它必须实现 `RawMutex` trait。这个“锁”**不保护你的数据**，它保护的是 `Signal` 内部的**状态机**（是 `None`、`Waiting` 还是 `Signaled`）。`Signal` 内部所有的操作，比如发送信号或等待信号，都需要先通过这个“锁”来安全地修改内部状态。

在锁的选择上，传入`CriticalSectionRawMutex` 是最安全稳妥、最不容易出错的选择，它会通过屏蔽中断实现来保证操作的原子性，因此适用于任何场景。

下面是`Signal`创建和使用的简单示例：

```rust
static SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();

// 发出信号并携带数据
SIGNAL.signal(value:u32)；
// 等待信号并返回信号携带值
SIGNAL.wait().await；
```

`Signal`主要使用的方法是`.signal()`和`.wait()`，更多详情可以查阅官方文档了解。

**与FreeRTOS的对比：**

Embassy的`Signal`虽然叫做信号，但是本质上跟freertos的信号量`Semaphore`完全不同，信号的本质是**最新数据缓存，关心的是数据的具体值**，信号量的本质是**资源计数器，关心的是资源的有无而非其具体内容**。在功能上，信号更接近于freertos的任务通知`Task Notifications`。

| 特性维度      | **Embassy 信号 Signal** | **FreeRTOS 任务通知 Task Notifications** |
| --------- | --------------------- | ------------------------------------ |
| **核心机制**  | 最新状态缓存，仅保留最新值。        | 每个任务内置的通知数组，包含状态和值。                  |
| **数据负载**  | 可携带**任意类型** `T` 的数据。  | 携带一个**32位无符号整数**值。                   |
| **行为语义**  | **覆盖**，新数据无条件覆盖旧数据。   | **灵活可配置**：支持覆盖、不覆盖、置位、递增等多种更新方式。     |
| **消费者数量** | **单消费者**，专为特定任务设计。    | **单消费者**，通知是直接发送给指定任务。               |
| **使用场景**  | 传递传感器读数、状态机状态等“最新状态”。 | 可作为轻量级二值/计数信号量、数据传递等任务同步。            |

## Watch

简单来说，`Watch`就是升级版的`Signal`，同样是传递单个最新的值，但是`Watch`**支持一对多通信**。定义如下：

```rust
pub struct Watch<M: RawMutex, T: Clone, const N: usize> 
```

其中`M`和`T`的含义与`Signal`一致，`N`指的是数据消费者的数量。

`Watch`及其发送方和接收方的创建示例如下：

```rust
static WATCH: Watch<CriticalSectionRawMutex, bool, 2> = Watch::new();

let  watch_send = WATCH.sender();
let  watch_rev0 = WATCH.receiver().expect("Failed to create watch receiver 0");
let  watch_rev1 = WATCH.receiver().expect("Failed to create watch receiver 1");
```

`Watch`的使用示例如下：

```rust
// 发送watch并携带数据
watch_send.send(ture)；
// 等待watch更新并获取最新值
watch_rev0.changed().await；
// 不等待watch更新直接获取当前数值
watch_rev1.get().await; 
```

`Watch`主要使用到的方法如实例所示，更多详情可以查阅官方文档了解。
