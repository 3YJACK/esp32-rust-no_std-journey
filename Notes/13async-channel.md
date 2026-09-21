# 学习目标

使用`esp-generate`创建工程(注意启用embassy异步框架)，并参考embassy同步通信模块的官方文档[embassy_sync - Rust](https://docs.rs/embassy-sync/latest/embassy_sync/)，编写代码并使用`Channel` 完成日志打印的多对一同步的应用示例。

# 完整源码

需先在终端中通过下面命令添加对应依赖，才能导入emabssy的同步通信模块：

```powershell
cargo add embassy-sync
```

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
    channel_receiver: Receiver<'static, CriticalSectionRawMutex, &'static str, CHANNEL_CAPACITY>,
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

```

**引脚连接参照表：**

| 外设  | 对应引脚  |
| --- | ----- |
| 按键  | GPIO6 |

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

每两秒主循环任务往通道发送消息，打印任务接收消息并打印出来

# 代码讲解

## Channel

embassy异步框架中的`channel`相当于freertos中的`queue`，都是用于在异步任务之间传递多组数据的队列，两者的设计目标和核心机制是高度相似的。其定义如下：

```rust
pub struct Channel<M, T, const N: usize>
where
    M: RawMutex,
{ /* private fields */ }
```

其中`M`指的是锁的类型，`T`指的是消息数据的类型，`N`指的是`channel`的长度，即可传输数据的数量大小。

下面是`channel`创建及使用的简单示例：

```rust
static CHANNEL: Channel<CriticalSectionRawMutex, u32, 8> = Channel::new();

let channel_sender = CHANNEL.sender();
let channel_receiver = CHANNEL.receiver();

// 异步发送，若channel已满会挂起直到有位置才发送 
channel_sender.send(42).await;
if !channel_sender.is_full(){
    // 同步非阻塞发送，若channel已满则立即返回
    channel_sender.try_send(42);
}
// 异步接收，若channel为空会挂起直到有数据才接收 
channel_receiver.receive().await;
// 同步非阻塞接收，若channel内没有数据则立即返回
 if !channel_sender.is_empty(){
     channel_receiver.try_receive();
}
```

同freertos的`queue`一样，`channel`遵循**先进先出**的数据输出顺序，也**支持多生产者多消费者的多对多通信**，当通道队列里的数据被一个消费者取走时，其他消费者就无法获取到这个数据。

如果需要**高优先级消息先出通道队列，则可以使用`PriorityChannel`**，如果是想实现**一发多收的通道队列，则应使用`PubSubChannel`**，发布订阅通道的消息数据可以让所有的接收者都能获取，其内部持有一个计数器，**数据在所有接收者都收到后才移出通道队列**。优先级通道和发布订阅通道的用法与普通通道`channel`类似，具体也可以查阅官方文档进一步了解，这里就不多介绍了。
