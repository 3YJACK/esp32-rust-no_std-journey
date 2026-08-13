# 学习目标

使用`esp-generate`创建工程，因`esp-rs/esp-hal`仓库的示例中并没有定时器相关演示，因此参考官方文档[esp_hal - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/index.html)和AI辅助下编写代码并实现定时器中断及PWM输出功能。

**前置知识：**

# 完整源码

```rust

```

**IO连接对照表：**

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

## 定时器

`esp-hal`中定时器有两种——`OneShotTimer`和`PeriodicTimer`，既一次性定时器和周期性定时器。定时器由定时器组`timg`进行管理，例如`ESP32S3`中一个定时器组由两个通用定时器和一个看门狗定时器组成，因此一个定时器的创建过程如下：

```rust
    // 先获取一个定时器组
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    // 再将一个定时器组的通用定时器实例化为周期性定时器
    let mut prd_timer = PeriodicTimer::new(timg0.timer0);
```
