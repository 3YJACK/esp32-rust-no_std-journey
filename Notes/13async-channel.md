## Channel

embassy异步框架中的`channel`相当于freertos中的`queue`，两者的设计目标和核心机制是几乎一致的。其定义如下：

```rust
pub struct Channel<M, T, const N: usize>
where
    M: RawMutex,
{ /* private fields */ }
```

其中`M`指的是锁的类型，`T`指的是消息数据的类型，`N`指的是`channel`的长度，即可传输数据的数量大小。

```rust
static CHANNEL: Channel<CriticalSectionRawMutex, u32, 8> = Channel::new();

let channel_sender = CHANNEL.sender()；
let channel_receiver = CHANNEL.receiver();



```

同freertos的`queue`一样，`channel`也是**支持多生产者多消费者的多对多通信**，当channel队列里的数据被一个消费者取走时，其他消费者就无法获取到这个数据。

如果是想实现**一发多收的通道队列，则应使用`PubSubChannel`**。
