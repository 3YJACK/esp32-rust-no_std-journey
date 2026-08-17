# 学习目标

使用`esp-generate`创建工程，因`esp-rs/esp-hal`仓库的示例中并没有定时器相关演示，因此参考官方文档[esp_hal - Rust](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/index.html)和AI辅助下编写代码并实现定时器中断及PWM输出功能。

# 完整源码

```rust

```

**IO连接对照表：**

| 外设            | 对于IO  |
| ------------- | ----- |
| LED1(定时器中断控制) | GPIO4 |
| LED2(PWM控制)   | GPIO6 |

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

## 定时器及其中断

`esp-hal`中定时器有两种——`OneShotTimer`和`PeriodicTimer`，既一次性定时器和周期性定时器。定时器由定时器组`timg`进行管理，例如`ESP32S3`中一个定时器组由两个通用定时器和一个看门狗定时器组成，因此一个定时器的创建过程如下：

```rust
    // 先获取一个定时器组
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    // 再将一个定时器组的通用定时器实例化为周期性定时器
    let mut prd_timer = PeriodicTimer::new(timg0.timer0);
```

 然后也是配置和编写定时器中断函数即可：

```rust
    // 设置中断处理程序为 timer_handler 函数
    prd_timer.set_interrupt_handler(timer_handler);    
    // 监听定时器中断
    prd_timer.listen();
    // 设置定时器周期为 1000 ms并启动定时器
    prd_timer.start(Duration::from_millis(1000));

    #[handler]
    #[ram]
    fn timer_handler() {
    // ......
    }
```

## LEDC和MCPWM

`esp-hal`中主要有两种PWM外设——`LEDC`和`MCPWM`，前者是专为简单的PWM应用(如LED控制)设计的通用PWM模块，后者则是功能强大的更为复杂的电机控制和电源应用设计的专用PWM模块。(注意：并非所有ESP芯片都支持`MCPWM`，需结合芯片手册和官方文档进行确认)

本篇示例中仅演示`LEDC`，`MCPWM`可参考官方文档自行实现。

LEDC的实现流程为：**创建→配置定时器和PWM输出通道→应用**。如下所示，代码较为简单，这里就不多解释，需要注意的是，`LEDC`的`channel`的相关方法是定义在其`Trait`——`ChannelIFace`中的。

```rust
    // 创建 LED2 用于 PWM 控制其亮度，采用 LEDC 外设
    let mut led2 = Ledc::new(peripherals.LEDC);
    // 根据官方文档可知 LEDC 使用时钟源为 APB 
    led2.set_global_slow_clock(ledc::LSGlobalClkSource::APBClk);

    // 创建一个低速定时器实例用于 LED2 的 PWM 控制
    let mut pwm_timer = led2.timer::<ledc::LowSpeed>(ledc::timer::Number::Timer0);
    // 占空比位数和频率只会影响到呼吸灯效果，不影响其实现，不用纠结具体配置参数
    pwm_timer.configure(ledc::timer::config::Config {
            duty: ledc::timer::config::Duty::Duty8Bit,
            clock_source: ledc::timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(1000),
        }).expect("Failed to configure PWM timer");

    // 将 LED2 的 GPIO6 引脚配置为 PWM 输出通道
    let mut pwm_channel = led2.channel(channel::Number::Channel0, peripherals.GPIO6);
    pwm_channel.configure(channel::config::Config {
            timer: &pwm_timer,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        }).expect("Failed to configure PWM channel");
        
    loop {
        pwm_channel.start_duty_fade(0, 100, 1000).unwrap();
        while pwm_channel.is_duty_fade_running() {}

        pwm_channel.start_duty_fade(100, 0, 1000).unwrap();
        while pwm_channel.is_duty_fade_running() {}
    }    
```

## 关于导入

不同于之前示例中通过通配符`*`简单无脑地将导入模块内所有成员，本篇示例中显式且明确地导入了所使用的模块及其成员，这是更合乎规范的导入方式。

```rust
use esp_hal::{
    main,
    handler,
    ram,
    Blocking,
    clock::CpuClock,
    gpio::{Output, Level, OutputConfig, DriveMode},
    time::{Duration, Rate},
    timer::{PeriodicTimer, timg::TimerGroup},
    ledc::{self, Ledc, timer::{self, TimerIFace}, channel::{self, ChannelIFace}},
};
```

同时还需要指出，对于通配符导入的一种常见认知错误——误以为通配符导入是递归性的，这也是我在写本篇示例教程时犯过的错误。

在本篇示例的构建初期，对于导入方式，如之前一样使用的还是通配符导入，如下代码所示，但是在编译时却出现了意想不到的错误，编译器显示未能找到结构体`TimerGroup`。

```rust
use  esp_hal::{
    // ......
    timer::*,
};

// 编译报错
fn main(){
    let timg0 = TimerGroup::new(peripherals.TIMG0);
}
```

通过阅读官方文档或`esp_hal`的源码可知，`timer`模块的目录结构是这样的：

```
src/
└── timer/
    ├── mod.rs      // 模块入口
    └── timg.rs     // 子模块，定义了 TimerGroup
    └── ....        // 其他子模块
```

`TimerGroup`确实是`timer`模块的内部成员，只不过是在`timg`模块里面，那为什么编译器会报错呢？

在把导入方式改成下示代码之后，编译便顺利通过了。

```rust
use  esp_hal::{
    // ......
    timer::{*, timg::*},
};   

fn main(){
    // 编译成功
    let timg0 = TimerGroup::new(peripherals.TIMG0);
}
```

于是可以推测，原因在于通配符的导入有误，我将通配符默认为是递归的，但实际上可能是通配符导入可以导入`timer`模块的所有公开成员及其导入的公开子模块，但是不会递归导入子模块`timg`内的公开成员，因此无法直接使用`TimerGroup`结构体。在此推测上，也可以推断出下示代码也应能通过编译：

```rust
use  esp_hal::{
    // ......
    timer::*,
};

fn main() {
    // 编译成功
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0);
}
```

尝试编译后发现确实如此，再结合网上查询资料可以确认推测是正确的——**rust的通配符导入是非递归的**。

很多人在学习rust时可能都会跟我一样，误以为通配符导入是递归的，毕竟在其他一些编程语言中导入行为确实是递归的。

但在rust中，**通配符导入只导入当前模块的直接公开成员，不会递归展开其子模块的内部成员。**

这是rust语言有意为之的设计规则，为了避免把库内部所有的子模块成员一股脑倒进当前命名空间。因为这会导致两个问题：一是**命名冲突的概率大大增加**，你根本不知道 `*` 到底引入了什么；二是代码的**可读性和可维护性下降**，光看命名读者可能无法一眼看出来自哪个子模块。

所以在rust的编码规范中，应该**尽量避免使用通配符导入这样简单无脑的导入方式**，而是应该**显式地清晰地写出所用成员的完整导入路径**，正如本篇示例最终采用的导入示例那般。

这里顺便讲一下导入模块自身`self`，模块成员和通配符的区别，以`ledc`模块为例：

| 特性           | **模块自身 (`self`)**                                 | **具体成员 (`Item`)**                                    | **通配符 (`*`)**                                        |
| ------------ | ------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- |
| **语法示例**     | `ledc::{self}`                                    | `ledc::Ledc`                                         | ` ledc::*`                                           |
| **引入作用域的内容** | 模块本身                                              | 模块内的特定成员                                             | 模块内所有公开的成员                                           |
| **代码中调用方式**  | 必须通过模块名调用<br/>`ledc::Ledc::xxx`✅<br/>`Ledc::xxx`❌ | 可通过已导入的模块内的成员名调用<br/>`Ledc::xxx`✅<br/> `timer::xxx`❌ | 可通过模块内任意公开成员名直接调用<br/>`Ledc::xxx`✅<br/>`timer::xxx`✅ |
| **命名冲突风险**   | ✅ **极低**（必须带模块前缀，路径清晰）                            | ⚠️ **中等**（若导入同名类型会冲突，例如`timer`模块再引入自身的话会引发冲突）        | ❌ **极高**（极易污染命名空间）                                   |
| **代码可读性**    | ✅ **高**（读者能立刻看出引用路径）                              | ⚠️ **中等**（需看头部 `use` 才能确定来源）                         | ❌ **低**（难以判断引用自哪个模块）                                 |

应根据实际场景，选择合适的导入方式！
