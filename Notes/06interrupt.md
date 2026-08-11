# 学习目标

使用`esp-generate`创建工程并参考`esp-rs/esp-hal`仓库的`./example/interrupt/gpio`示例，编写代码并实现简单的按键中断控制LED灯亮灭功能。

**前置知识：**

本篇内容建议在掌握了[00语法基础](./00语法基础.md)中`数据结构`小节中的`引用`和`枚举类型`的`Option`之后再进行学习。

# 完整源码

```rust

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

但实际开发中，一个数据常常被多个模块持有并操作。正如本篇的按键中断示例，按键的值既在主函数中创建修改，又在中断服务函数中不断读取。因为中断是随时可以发生的，在编译器看来两者可能同时存在，这违反了rust的借用规则，因此无法通过编译。所以我们需要引用`RefCell`，来避开这个规则限制。

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
    // 程序运行到这里直接崩溃：already mutably borrowed
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
     BUTTON.replace(button);
     LED.replace(led);  
}
```

## interrupt-中断

中断开发的流程如下：

**注册中断→进行中断配置→编写中断服务函数**

首先创建一个IO管理器。因为ESP32的硬件特性——所有的GPIO 共享一个中断源。因此在`esp-hal`中，引脚中断被设计为由IO管理器统一集中管理，需通过IO管理器来设置中断服务函数，在函数内部通过 `is_interrupt_set()`作为中断标志位判断并分发具体的处理逻辑。

```rust
  // 创建IO引脚管理器并设置中断处理程序为handler函数
  let mut io = Io::new(peripherals.IO_MUX);
  io.set_interrupt_handler(handler);
```



同时注意临界区保护和退出中断清除标志时。




