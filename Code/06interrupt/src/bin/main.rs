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

