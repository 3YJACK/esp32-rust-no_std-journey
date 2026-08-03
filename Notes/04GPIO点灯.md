> 本篇使用`esp-generate`创建工程并参考`esp-rs/esp-hal`仓库的`./example/interrupt/gpio`示例，编写代码并实现GPIO点灯闪烁功能。

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
    main,
    time::{Duration, Instant},
    gpio::*,   // 导入gpio模块里所有的公开类型和函数
};

use log::info;

use esp_backtrace as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

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
    // 外设统一初始化
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

    // led initialization 
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let delay = esp_hal::delay::Delay::new();

    // led blink
    led.set_high();
    delay.delay_ms(1000);

    led.set_low();
    delay.delay_ms(1000);

    loop {
       led.toggle();
       delay.delay_ms(1000);
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

**预期运行效果：**



# 代码讲解

## GPIO

在rust中，所有外设的使用都需要先经过一个统一的初始化，将所有片上外设的所有权一次性全部拿走，然后再通过一个对象统一管理外设，按需分配使用。 

```rust
let peripherals = esp_hal::init(config);
```

经过统一初始化后，`peripherals`获得全部外设的所有权，GPIO从`peripherals`中拿走所需引脚并进行初始化配置：

```rust
let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
```

这里是将引脚2配置为输出模式，初始电平为低电平，引脚配置使用库提供的默认配置。

阅读官方文档[esp-rs Documentation and Resources](https://docs.espressif.com/projects/rust/)中`esp-hal`的章节可知，输出配置结构体的成员及默认配置如下：

Output pin configuration

This struct is used to configure the drive mode, drive strength, and pull direction of an output pin. By default, the configuration is set to:

- Drive mode: `DriveMode::PushPull`
- Drive strength: `DriveStrength::_20mA`
- Pull direction: `Pull::None` (no pull resistors connected)

因此，自行配置输出模式并初始化的流程为：

**创建配置→修改配置结构体字段→创建`Output/Input`对象**

```rust
let config = OutputConfig::default()
    .with_drive_mode(DriveMode::PushPull)
    .with_pull(Pull::None);
let mut led = Output::new(peripherals.GPIO2, Level::Low, config);
```

然后就可以调用输出模式的相关驱动函数，详情可见官方文档[Output in esp_hal::gpio - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/gpio/struct.Output.html)。

## Delay


