# 学习目标

使用`esp-generate`创建工程(注意启用embassy异步框架)，并参考`esp-rs/esp-hal`仓库的`./example/async/embassy_multicore`示例和embassy同步通信的官方文档[embassy_sync - Rust](https://docs.rs/embassy-sync/latest/embassy_sync/)，编写代码并实现

需先在终端中通过下面命令添加依赖：

```powershell
cargo add embassy-sync
```

# 完整源码

```rust

```

**引脚连接参照表：**

| 外设  | 对应引脚 |
| --- | ---- |
|     |      |

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

## Signal-信号

`Signal`是用于**一对一传递单个数据**的同步通信方式，进行发送操作时新值会覆盖旧值，适合接收方只关心最新值的状态同步任务。

`Signal` 的定义如下：

```rust
pub struct Signal<M, T>
where
    M: RawMutex,
{ /* private fields */ }
```

- **`T`：这是 `Signal` 携带的数据类型**。只能携带一个数据且**只保留最新的值**。

- **`M`：实现同步的“锁”**：这是一个类型参数，它必须实现 `RawMutex` trait。这个“锁”**不保护你的数据**，它保护的是 `Signal` 内部的**状态机**（是 `None`、`Waiting` 还是 `Signaled`）。`Signal` 内部所有的操作，比如发送信号或等待信号，都需要先通过这个“锁”来安全地修改内部状态。

在锁的选择上，传入`CriticalSectionRawMutex` 是最安全稳妥、最不容易出错的选择，它会通过屏蔽中断实现来保证操作的原子性，因此适用于任何场景。

---

**与FreeRTOS的对比：**

Embassy的`Singal`虽然叫做信号，但是本质上跟freertos的信号量`Semaphore`完全不同，信号的本质是**最新数据缓存，关心的是数据的具体值**，信号量的本质是**资源计数器，关心的是资源的有无而非其具体内容**。在功能上，信号更接近于freertos的任务通知`Task Notifications`。

| 特性维度      | **Embassy 信号 Signal** | **FreeRTOS 任务通知 Task Notifications** |
| --------- | --------------------- | ------------------------------------ |
| **核心机制**  | 最新状态缓存，仅保留最新值。        | 每个任务内置的通知数组，包含状态和值。                  |
| **数据负载**  | 可携带**任意类型** `T` 的数据。  | 携带一个**32位无符号整数**值。                   |
| **行为语义**  | **覆盖**，新数据无条件覆盖旧数据。   | **灵活可配置**：支持覆盖、不覆盖、置位、递增等多种更新方式。     |
| **消费者数量** | **单消费者**，专为特定任务设计。    | **单消费者**，通知是直接发送给指定任务。               |
| **使用场景**  | 传递传感器读数、状态机状态等“最新状态”。 | 可作为轻量级二值/计数信号量、数据传递等任务同步。            |

---
