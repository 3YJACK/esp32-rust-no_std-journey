# 学习目标

1. 通过官方包管理器`Cargo`添加第三方库依赖包OLED显示屏的`ssd1306`驱动

2. 使用`esp-generate`创建工程，因`esp-rs/esp-hal`仓库的示例中并没有i2c的相关演示，因此参考官方文档[esp_hal - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/index.html)来编写示例并实现对OLED显示屏的驱动与i2c通信

**前置知识：**

# 添加依赖

添加第三方依赖包过程可以概括为三步：**找库**、**添加**、**使用**。

1. **找库**

Rust的官方库仓库是 [**crates.io**](https://crates.io/)，可以在这里搜索所需要的软件包。其中每个软件包都会配有详细文档和GitHub源码链接，很多软件包在文档中就说明了添加方法及指令。

2. **添加**

根据文档说明，通过下面的指令添加依赖包：

```powershell
cargo add ssd1306
```

这个命令会自动将最新版本的 `ssd1306` 添加到 `Cargo.toml` 的 `[dependencies]` 部分

或者也直接在Cargo.toml文件中手动添加：

```powershell
ssd1306 = "0.10.0"
```

3. **使用**

依赖添加并保存后，Cargo 会在下次构建时自动下载并编译，然后就可以在代码中引入`ssd1306`驱动库并根据文档说明进行使用了。

```rust
use ssd1306::{......}；
```

# 完整源码

本篇示例内容分两部分，初始化i2c和通过`ssd1306`的驱动库操作OLED显示屏。其中`ssd1306`

```rust

```

**引脚连接参照表：**

|     |     |
| --- | --- |
|     |     |

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

## 平台无关驱动

在传统的嵌入式开发中，设备的驱动库往往依赖于具体厂商的HAL库函数，硬件平台一旦更换，驱动也要跟着重写。但本篇示例中，我们使用`SSD1306`驱动库时，几乎是直接使用，并没有关心其底层实现是否支持ESP32平台。

倘若你打开这个驱动库的源码，你会发现在源码中**根本没有提到任何具体的硬件平台**，它既不认识ESP32，也不认识STM32，但是它偏偏就是可以直接运行在这些硬件平台上。

这就是Rust嵌入式一个强大的核心设计理念——**平台无关驱动**。

为了实现平台无关驱动，Rust设计了一个名为 **[`embedded-hal`](https://crates.io/crates/embedded-hal)** 的库。它的核心思想非常简单：**定义一套标准接口（Trait），驱动只依赖这套接口，芯片厂商为这套接口提供具体实现**。

用一句话概括：`embedded-hal`是一套“**接口协议**”，**但仅仅是接口定义**，没有任何具体实现。而这个具体实现，则是由芯片厂商负责，开发者只需要在这个接口协议的基础上专注于驱动逻辑即可。

因此在`embedded-hal`生态中，设备驱动和芯片厂商的角色分工非常明确：

| 角色         | 做什么                           | 例子                           |
| ---------- | ----------------------------- | ---------------------------- |
| **芯片HAL库** | 为具体芯片实现`embedded-hal`的trait   | `esp-hal`、`stm32f4xx-hal`    |
| **设备驱动**   | 只依赖`embedded-hal`的trait，不关心芯片 | `ssd1306`、`bmp180`、`mpu6050` |

**芯片HAL库**：如`esp-hal`，负责把ESP32的硬件寄存器操作，封装成`embedded_hal::i2c::I2c` trait的实现。

**设备驱动**：如`ssd1306`，只认`embedded_hal::i2c::I2c`这个trait，**完全不关心**底层是ESP32还是STM32。

而我们在使用时只需要把芯片HAL库创建的I2C实例，注入到设备驱动里即可，这也就是所谓的"**依赖注入**"。

结合代码分析，在该驱动库的源码中，I2C 入口长这样：

```rust
pub fn new<I>(i2c: I) -> I2CInterface<I>
where
    I: embedded_hal::i2c::I2c,
{
    Self::new_custom_address(i2c, 0x3C)
}
```

它只要求：给我一个实现了 `embedded_hal::i2c::I2c` 的类型。`esp-hal` 的 `I2c` 实现了这个 trait，STM32、nRF、RP2040 的 HAL 也实现了，所以**同一份驱动能用在不同芯片上**。

当你把ESP32实现了`embedded_hal::i2c::I2c` 类型的实例注入驱动：

```rust
let i2c = esp_hal::i2c::master::I2c::new(...);

let interface = I2CDisplayInterface::new(i2c);

let mut display = Ssd1306::new(interface,......）;
```

编译器会把泛型里的 `I` 替换成具体类型：

```rust
Ssd1306<esp_hal::i2c::master::I2c<'_, Blocking>>
```

然后调用esp_hal的具体实现，再往底层便是ESP的寄存器操作，换作其他硬件平台也如出一辙。

**与Linux对比——相同思想不同实现：**

通过**标准接口驱动设备，隔离底层实现**？熟悉Linux的读者，应该都会跟我一样觉得这个机制跟Linux的非常类似。

确实，两者在设计理念上是一致的，但是具体实现却不一样。简单来说，**Linux是运行时的硬件抽象，Rust是编译时的运行抽象**。

- **Linux 的做法**： `read(fd, buffer, size)`，内核中的 VFS（虚拟文件系统）会把它转发给具体的 ext4、FAT32 驱动，或者转发给具体的 NVMe、SD 卡驱动。应用层代码**不关心**硬盘是 SATA 还是 USB。

- **Rust `embedded-hal` 的做法**： `i2c.write(address, data)`，这个 Trait 方法在编译时会被替换成具体的 `esp_hal::i2c::I2C` 或 `stm32_hal::i2c::I2C` 的实现。驱动代码**不关心**硬件平台用的是 ESP32 还是 STM32。

|          | **Linux 抽象**                | **Rust `embedded-hal` 抽象**                     |
| -------- | --------------------------- | ---------------------------------------------- |
| **发生时机** | **运行时**                     | **编译时**                                        |
| **实现机制** | 函数指针、结构体虚表（Vtable）、设备树动态匹配。 | 泛型单态化（Monomorphization）、静态分发（Static Dispatch）。 |
| **性能开销** | 有运行时开销（函数指针跳转、指令缓存污染）。      | **零开销**，编译后生成的代码。                              |
| **灵活性**  | 可以在不重启系统的情况下加载/卸载驱动模块。      | 驱动在编译时就已完全固定。换一颗芯片，需要重新编译整个固件。                 |
