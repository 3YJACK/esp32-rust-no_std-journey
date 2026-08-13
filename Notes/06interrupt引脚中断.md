# 学习目标

使用`esp-generate`创建工程并参考`esp-rs/esp-hal`仓库的`./example/interrupt/gpio`示例，编写代码并实现简单的按键中断控制LED灯亮灭功能。

**前置知识：**

本篇内容建议在掌握了[00语法基础](./00语法基础.md)中`数据结构`小节中的`引用`和`枚举类型`的`Option`之后再进行学习。

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

use core::cell::RefCell;

use critical_section::Mutex;

use esp_hal::{
    clock::CpuClock,
    main,
    time::{Duration, Instant},
    gpio::*,
    handler,
    ram,
    delay::*,
};

use log::info;

use esp_backtrace as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// 为了让中断处理程序能够访问，定义为全局变量，再包装为Mutex临界区互斥锁和RefCell内部可变器，同时使用Option包装Input类型，进行延迟初始化
// 因为static变量在编译时就是确定值，而Input类型的对象实例化在运行时才能完成，所以先使用Option类型包装并赋值为None进行占位，完成实例化后再替换
static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

static LED: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));

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

    // 创建IO引脚管理器并设置中断处理程序为handler函数
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(handler);

    let mut led = Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default());

    let config = InputConfig::default().with_pull(Pull::Up);
    let mut button = Input::new(peripherals.GPIO6, config);

    // 临界区保护
    critical_section::with(|cs| {
        // 设置按钮引脚为下降沿触发中断
        button.listen(Event::FallingEdge);
        // 获取BUTTON的临界区互斥锁并借用可变引用
        BUTTON.borrow_ref_mut(cs)   
              .replace(button);   // 将BUTTON全局变量替换为实例化的button对象，这样中断处理程序就可以访问到按钮引脚

        LED.borrow_ref_mut(cs)
           .replace(led);   
    });

    let delay = Delay::new();

    loop {
        info!("Waiting for button press...");
        delay.delay_millis(5000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}

#[handler]
// 中断处理程序，必须使用#[ram]属性将其放置在RAM中，以便在中断发生时能够快速响应
#[ram]
fn handler() {
    critical_section::with(|cs| {
        // borrow_ref_mut返回的RefMut<Option<Input>>类型，这是一个指向Option的智能指针，
        // as_mut()方法从RefMut<Option<Input>>中取出Option<&mut Input>，
        // expect()方法解封装Option，如果Option是Some，则返回其中的值&mut Input，如果是None，则会panic。 
        let mut button_ref = BUTTON.borrow_ref_mut(cs); 
        let button = button_ref.as_mut()
                            .expect("Button not initialized");

        if button.is_interrupt_set() {
            LED.borrow_ref_mut(cs)
                .as_mut()
                .expect("LED not initialized")
                .toggle();

            info!("Button was the source of the interrupt");
        } 

        // 清除中断标志
        button.clear_interrupt();
    });
}
```

**IO连接对照表：**

| 外设   | 对应IO  |
| ---- | ----- |
| 按键   | GPIO6 |
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

烧录成功后，每5秒输出日志信息"Waiting for button press..."，按下按键后，LED小灯的电平状态翻转，同时输出日志信息"Button was the source of the interrupt"。

# 代码讲解

## RefCell-内部可变性

在rust的借用规则中，**一个值只能有多个不可变引用`&` 或一个可变引用 `&mut` ，两种引用不能同时存在**。

但实际开发中，一个数据常常被多个模块持有并操作。正如本篇的按键中断示例，按键的值既在主函数中创建修改，又在中断服务函数中不断读取。因为中断是随时可以发生的，在编译器看来两者可能同时存在，这违反了rust的借用规则，是无法通过编译的。所以我们需要引用`RefCell`，来避开这个规则限制。

```rust
use core::cell::RefCell;
```

`RefCell`则是把这个规则从**编译期**改为了**运行时**（仅对于该类型的值而非全局）。既然编译期的静态检查无法应对这种异步并发的场景，那么就在运行时维护数据的读写同步，若运行时违反了规则，那么程序会直接`panic`崩溃。

其工作机制如下：

`RefCell`提供了两个主要方法：

- **`.borrow()`**：申请“读权限”。可以同时存在多个（因为只读不写）。

- **`.borrow_mut()`**：申请“写权限”。只能存在一个。

当你调用这两个方法时，`RefCell` 内部会维护一个**计数器**：

- 记录当前有几个在读（`borrow`）。

- 或者有没有一个在写（`borrow_mut`）。

**运行时规则：**

申请 `borrow_mut()` 时，如果此时计数器显示已经有 `borrow` 或 `borrow_mut` 在占用，`RefCell` 会立刻引发 `panic!`。

`RefCell`把**可变权限隐藏在数据类型内部**，允许一个对外声明不可变的值，在其内部进行可变修改，因此这个特性也称之为**内部可变性**。

下面是一个简单的示例：

```rust
use core::cell::RefCell;

fn main() {
    let data = RefCell::new(5); // data 对外不可变，但内部可变

    // 1. 同时多个读取data（允许）
    let r1 = data.borrow();
    let r2 = data.borrow();
    println!("读到的值: {} {}", r1, r2);
    // r1, r2 离开作用域，释放读取状态

    // 2. 申请写权限（允许，data既没有读取也没有给出唯一的写权限）
    let mut w = data.borrow_mut();
    *w += 1; // 修改内部的值
    println!("修改后: {}", w);
    // w 离开作用域，释放写权限（实际工程中建议用{}显式标出作用域）

    // 3. 危险操作（运行时崩溃！）
    let r3 = data.borrow();     // 处于读取状态
    let w2 = data.borrow_mut(); // 试图申请写权限
    // 程序运行到这里直接崩溃
}
```

## Mutex-互斥锁

对于我们的按键中断示例，RefCell只解决了静态编译时的语法限制，而非真正在程序中避免了读写冲突，因为中断可能破坏读写操作的原子性，因此我们还需要引用临界区模块的互斥锁：

```rust
use critical_section::Mutex;
```

互斥锁在临界区内会屏蔽全局中断，从而避免主函数跟中断可能对同一数据的并发抢占。

**临界区使用方法：**

将需要临界区保护的代码块放入`critical_section::with()`即可。

```rust
critical_section::with(|cs| {
    // the code should be protected by critical_section 
});
```

其中`|cs|`是临界区令牌，用于保证数据访问时是处于临界区状态，因此所有试图“访问”或“修改”受临界区保护的内部数据的方法都需要传入`|cs|`。

## Option的延时初始化

`Option`除了用于处理可能的空值情况，还能用于延时初始化。

在rust中，全局变量用`static`修饰且要求在编译期就确定其值，因此对于实际值要运行时才能确定的全局变量，可以使用`Option`的空值进行占位以通过编译，然后在运行时使用`replace`方法去替换，传入参数为要替换的值，如本篇示例所示：

```rust
static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));
static LED: Mutex<RefCell<Option<Output>>> = Mutex::new(RefCell::new(None));

fn main() -> !{
    critical_section::with(|cs| {
        BUTTON.replace(button);
        LED.replace(led);  
    });
}
```

## interrupt-中断

中断开发的流程如下：

**注册中断→进行中断配置→编写中断服务函数**

1. **注册中断**

首先创建一个IO管理器。因为ESP32的硬件特性——所有的GPIO 共享一个中断源。因此在`esp-hal`中，引脚中断被设计为由IO管理器统一集中管理，需通过IO管理器来设置中断服务函数，在函数内部通过 `is_interrupt_set()`作为中断标志位判断并分发具体的处理逻辑。

```rust
  // 创建IO引脚管理器并设置中断处理程序为handler函数
  let mut io = Io::new(peripherals.IO_MUX);
  io.set_interrupt_handler(handler);
```

2. **中断配置**

设置中断的触发条件，完成中断需要访问的全局变量的延迟初始化替换等等，如本篇示例代码所示：

```rust
   critical_section::with(|cs| {
        // 设置按钮引脚为下降沿触发中断
        button.listen(Event::FallingEdge);
        // 获取BUTTON的临界区互斥锁并借用可变引用
        BUTTON.borrow_ref_mut(cs)
        // 将BUTTON全局变量替换为实例化的button对象，这样中断处理程序就可以访问到按钮引脚   
              .replace(button);   

        LED.borrow_ref_mut(cs)
           .replace(led);   
    });
```

3. **中断服务函数**

在rust中，对于中断服务函数通常使用`#[interrupt]`属性来标记，但在`esp-hal`库中则将其封装为了集成度更高、功能更强的`#[handler]`属性，例如可以在属性中直接指定中断优先级#`[handler(priority = esp_hal::interrupt::Priority::Priority2)]`。

除此之外，`esp-hal`还提供了`#[ram]`属性，用于将中断服务函数的代码存储在读取速度更快的RAM上，而默认情况下代码是存储在Flash的，对于中断服务函数这种时间敏感型任务，可能会引发严重问题。

```rust
#[handler]
// 中断处理程序，必须使用#[ram]属性将其放置在RAM中，以便在中断发生时能够快速响应
#[ram]
fn handler() {
    critical_section::with(|cs| {
        // borrow_ref_mut返回的RefMut<Option<Input>>类型，这是一个指向Option的智能指针，
        // as_mut()方法从RefMut<Option<Input>>中取出Option<&mut Input>，
        // expect()方法解封装Option，如果Option是Some，则返回其中的值&mut Input，如果是None，则会panic。 
        let mut button_ref = BUTTON.borrow_ref_mut(cs);
        let button = button_ref.as_mut()
                            .expect("Button not initialized");

        if button.is_interrupt_set() {
            LED.borrow_ref_mut(cs)
                .as_mut()
                .expect("LED not initialized")
                .toggle();

            info!("Button was the source of the interrupt");
        } 

        // 清除中断标志
        button.clear_interrupt();
    });
}
```

在中断服务函数中，通过`.is_interrupt_set()`方法来判断对应引脚的中断是否触发，在执行完中断服务后，退出函数前切记使用`.clear_interrupt()`方法清除中断标志位，避免重复中断。

## Refmut守卫

在中断服务函数的代码中，LED只操纵一次，所以可以链式调用、用完即弃，而BUTTON既需要查询中断标志位又需要清除标志位，如果写作调用链形式，则需多次借用，略显繁琐：

```rust
if BUTTON.borrow_ref_mut(cs)
            .as_mut()   
            .unwrap()
            .is_interrupt_set()
{
   // ......             
}

    BUTTON.borrow_ref_mut(cs)
        .as_mut()   
        .unwrap()
        .clear_interrupt()            
```

如果想要一次借用，多次操作，则需要通过赋值将借用后`BUTTON`的可变引用保存下来，那么这里你可能会跟我一样疑问，为什么在源码中要赋值两次而不是像下示代码那样一次搞定呢？

```rust
let button = BUTTON.borrow_ref_mut(cs)
                    .as_mut()
                    .expect("Button not initialized");

if button.is_interrupt_set() {
        LED.borrow_ref_mut(cs)
            .as_mut()
            .expect("LED not initialized")
            .toggle();

        info!("Button was the source of the interrupt");
} 

    // 清除中断标志
    button.clear_interrupt();
```

这是因为`BUTTON.borrow_ref_mut()`返回的是`RefMut`类型数据，`RefMut` 是一个**守卫对象**，它内部包含一个指向 `BUTTON` 数据的引用。`.as_mut()`拿到的是对`RefMut`守卫内部的引用，`button`只保留了这个引用，但这个引用的生命周期是跟守卫绑定的。

而上示代码中，守卫在调用链结束后随即被释放了，`button`所保存的引用也跟着失效，但往下又调用了`button.clear_interrupt()`，因此编译时会报错`creates a temporary value which is freed while still in use`，大意是使用了已经被释放的临时值`button`。

因此需要先有一个中间值将守卫给保存下来，将其生命周期从当前语句延长到当前代码块，然后再取出其内部的可变引用赋值给一个对象，才能实现一次借用，多次操作。

守卫的存在是为了实现互斥锁，其内部不仅包含有一个数据的内部可变引用，同时负责维护借用计数器，因此这个引用跟守卫的生命周期是绑定的。当守卫存活时，其他地方无法借用。当守卫离开作用域时，借用计数器清零，引用也应该随之失效，否则在其他地方又申请了可变引用时，会存在两个可变引用，这违反了rust的借用规则。
