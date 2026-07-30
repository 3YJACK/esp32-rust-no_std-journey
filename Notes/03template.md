根据上篇笔记`02工程创建`使用`esp-generate`按以下配置创建一个简单工程模板。

```powershell
 ✅ Enable unstable HAL features. 
 ✅ Enable allocations via the esp-alloc crate. 
 ✅ Enable stack smashing protection.  
 Flashing, logging and debugging (espflash)
     ✅ Use the log crate to print messages. 
     ✅ Use esp-backtrace as the panic handler. 
 Optional editor integration 
     ✅ Add settings for Visual Studio Code        
```

# 完整源码

下面为带了个人注释的工程模板源码：

```rust
// no_std 和 no_main 表示这是一个“没有标准库、没有默认 main 入口”的嵌入式程序。
#![no_std]
#![no_main]

// 这里的 #![deny(...)] 表示“把某些不安全/不推荐的写法当成错误来处理”。
// 这样做的好处是：编译器会更严格地提醒你，避免在嵌入式程序里写出容易出问题的代码。
//
// clippy::mem_forget：提醒你不要随便用 mem::forget，
// 因为它可能让某些包含缓冲区的硬件对象被“忘记释放/清理”，从而导致问题。
//
// clippy::large_stack_frames：提醒你不要在 main 函数里放太大的栈数据，嵌入式设备的栈空间通常很有限。
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
```

# 烧录运行

# 代码讲解

## 属性

在模板代码的头部，可以看到几行`#![]`包围的代码，在 Rust 中，`#[]` 和 `#![]` 统称为**属性（Attributes）**。

可以把它们理解为**写给编译器的“特殊指令”或“元数据”**，用来告诉编译器对代码进行特定的处理，比如条件编译、自动生成代码、设置入口点等。

两者的区别在于**作用范围（作用域）**不同：

- **`#[]`**：**外部属性**——放在修饰区域的外部，修饰它正下方紧挨着的那个代码块（函数、结构体、变量等）。

- **`#![]`**：**内属性**——放在修饰区域的外部，修饰它所在的整个文件、模块或 Crate（软件包）。

源码开头的内部属性，就是作用在整个文件中，声明裸机嵌入式环境，禁止随意使用大型栈数据和`mem::forget`（`mem::forget`会让 Rust 跳过该对象的析构函数（Drop），也就是“忘记”释放该对象占用的资源）。

```rust
// no_std 和 no_main 表示这是一个“没有标准库、没有默认 main 入口”的嵌入式程序。
#![no_std]
#![no_main]

// 这里的 #![deny(...)] 表示“把某些不安全/不推荐的写法当成错误来处理”。
// 这样做的好处是：编译器会更严格地提醒你，避免在嵌入式程序里写出容易出问题的代码。
//
// clippy::mem_forget：提醒你不要随便用 mem::forget，
// 因为它可能让某些包含缓冲区的硬件对象被“忘记释放/清理”，从而导致问题。
//
// clippy::large_stack_frames：提醒你不要在 main 函数里放太大的栈数据，嵌入式设备的栈空间通常很有限。
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
```

而在`mian`函数前面，又使用了外部属性，也就是仅作用于`main`函数的局部允许使用大型栈数据，不会触发编译器警告。原因在代码中也给出：

> it's not unusual to allocate larger buffers etc. in main
> 
> 在 main 函数里分配较大的缓冲区是很常见的。

```rust
// 对于mian函数，通过#[allow]来允许使用大型栈数据以通过编译
#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
//  #[main] 是 esp_hal 提供的一个过程宏属性，作用是为 no_std/no_main 程序生成真正的入口点。
#[main]
fn main() -> ! {
}
```


