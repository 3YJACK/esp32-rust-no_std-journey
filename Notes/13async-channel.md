## Channel

embassy异步框架中的`channel`相当于freertos中的`queue`，都是用于在异步任务之间传递多组数据的队列，两者的设计目标和核心机制是高度相似的。其定义如下：

```rust
pub struct Channel<M, T, const N: usize>
where
    M: RawMutex,
{ /* private fields */ }
```

其中`M`指的是锁的类型，`T`指的是消息数据的类型，`N`指的是`channel`的长度，即可传输数据的数量大小。

下面是`channel`创建及使用的简单示例：

```rust
static CHANNEL: Channel<CriticalSectionRawMutex, u32, 8> = Channel::new();

let channel_sender = CHANNEL.sender();
let channel_receiver = CHANNEL.receiver();

// 异步发送，若channel已满会挂起直到有位置才发送 
channel_sender.send(42).await;
if !channel_sender.is_full(){
    // 同步非阻塞发送，若channel已满则立即返回
    channel_sender.try_send(42);
}
// 异步接收，若channel为空会挂起直到有数据才接收 
channel_receiver.receive().await;
// 同步非阻塞接收，若channel内没有数据则立即返回
 if !channel_sender.is_empty(){
     channel_receiver.try_receive();
}
```

同freertos的`queue`一样，`channel`遵循**先进先出**的数据输出顺序，也**支持多生产者多消费者的多对多通信**，当通道队列里的数据被一个消费者取走时，其他消费者就无法获取到这个数据。

如果需要**高优先级消息先出通道队列，则可以使用`PriorityChannel`**，如果是想实现**一发多收的通道队列，则应使用`PubSubChannel`**，发布订阅通道的消息数据可以让所有的接收者都能获取，其内部持有一个计数器，**数据在所有接收者都收到后才移出通道队列**。优先级通道和发布订阅通道的用法与普通通道`channel`类似，具体也可以查阅官方文档进一步了解，这里就不多介绍了。
