# 学习目标

1. 通过官方包管理器`Cargo`添加第三方库依赖包OLED显示屏的`ssd1306`驱动

2. 使用`esp-generate`创建工程，因`esp-rs/esp-hal`仓库的示例中并没有i2c的相关演示，因此参考官方文档[esp_hal - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/index.html)来编写示例并实现对OLED显示屏的驱动与i2c通信

**前置知识：**

# 完整源码

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
