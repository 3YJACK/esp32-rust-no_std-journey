> 本篇使用`esp-generate`创建工程并参考`esp-rs/esp-hal`仓库的`./example/interrupt/uart`示例，编写代码并实现最简UART通信功能。 

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
    uart::*, // includes Uart Module 
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

    // uart initialization
    let uart_config = Config::default()
        .with_baudrate(115200)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1);

    let mut uart = Uart::new(peripherals.UART1, uart_config)
        .expect("Failed to initialize UART")    
        .with_tx(peripherals.GPIO43)
        .with_rx(peripherals.GPIO44);   

    let message = b"Hello, UART!\n";
    
    // delay initialization
    let delay = esp_hal::delay::Delay::new();

    loop {
        uart.write(message);
        delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
```

# 烧录运行

# 代码讲解

## UART

UART的初始化流程与上一篇的GPIO如出一辙：

**创建默认配置→修改配置结构体字段→创建串口对象，绑定引脚**

```rust
    // uart initialization
    let uart_config = Config::default()
        .with_baudrate(115200)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1);

    let mut uart = Uart::new(peripherals.UART1, uart_config)
        .expect("Failed to initialize UART")    
        .with_tx(peripherals.GPIO43)
        .with_rx(peripherals.GPIO44);   
```

完成初始化即可操作UART收发数据，相关的函数方法，数据类型等待都可在官方文档[esp_hal::uart - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/uart/index.html)中查阅。

## Result类型及其处理

在上示的UART初始化代码片段中，需要注意的是`Uart::new()`返回的是一个`Reslut`类型，必须对其处理才能得到`Uart`对象。

```rust
    let mut uart = Uart::new(peripherals.UART1, uart_config)
        .expect("Failed to initialize UART")    
```

`Reslut`类型是对返回结果的一层封装，可能是成功(OK)也可能是失败(err)。这里的处理方法是`.expect()`，即成功的话就取出其返回值也就是`Uart`对象，失败的话也会返回值但这里对于失败的处理是打印错误信息并停止程序运行。

除此之外，对于`Reslut`类型处理方法，还有`.unwrap()`，`?`操作符，`match`匹配等。

**1.`.unwrap()`终止程序**

跟`.expect()`一样，当`Reslut`类是错误时直接终止程序。但不同的是`.unwrap()` 在终止程序不能打印指定的日志信息，而`.expect()`可以。

**2.`?`向上传播**

`?` 是 Rust 的**错误传播语法**，加在返回 `Result` / `Option` 的表达式后面。

带`?`的表达式在运行出错时会将错误值返回给该表达式的调用者，`?`操作符的报错信息还可以向上沿着调用链冒泡传播，直到传播过程中错误信息被处理或最终传到`main`函数。

以下面代码为例：

```rust
// 为了演示，将源码的uart初始化过程封装为一个返回result类型的函数
fn uart_init()-> Result<Uart, Error>
{
   let uart_config = Config::default()
        .with_baudrate(115200)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1);

    let mut uart = Uart::new(peripherals.UART1, uart_config)?  
        .with_tx(peripherals.GPIO43)
        .with_rx(peripherals.GPIO44);  
}

fn main()-> Result<()> 
{
    // ......
    
    let mut uart = uart_init()?；
    
    OK(())
}
```

在该示例中，如果`Uart::new()`对象创建失败，`?`操作符会将错误值会返回给`uart_init()`，而`uart_init()`也因`？`操作符继续将错误值向上传播给`main()`，最终程序退出并将错误值打印出来。

相比传统写法中对每个可能出错的函数都进行结果判断并处理，`？`操作符用起来要方便简洁的多。

**注意：**`?` 只能在**返回 `Result` 或 `Option` 的函数**中对**返回 `Result` / `Option` 的表达式**使用。

**3.`match`模式匹配**

显式处理 `Ok` 和 `Err` 两个分支，示例代码如下：

```rust
let uart = match Uart::new(peripherals.UART1, uart_config) 
{
    Ok(u) => u.with_tx(...)
              .with_rx(...),
    Err(e) => panic!("Failed to initialize UART: {:?}", e);
};
```

## 字节切片

```rust
let message = b"Hello, UART!\n";
```

这里在`Hello UART!`前面加个`b`是将字符串切片`&str`转换成字节切片`&[u8]`，等效于下示代码：

```rust
let msg:&str = "Hello";
let bytes: &[u8] = msg.as_bytes(); // 得到 [72, 101, 108, 108, 111]
```

两者的区别在于：

- **`&str`（字符串切片）**：**必须**是有效的 **UTF-8** 编码。它只能指向合法的、符合 UTF-8 标准的字符序列。

- **`&[u8]]`（字节切片）**：**可以是任何数据**。它只是一段内存的字节集合，不关心这些字节代表什么。它可以是 UTF-8 文本、图片的二进制数据、传感器原始读数，或者是乱码。

串口通信时要求使用的是字节切片`&[u8]`，而其本身传输的数据也是字节流。


