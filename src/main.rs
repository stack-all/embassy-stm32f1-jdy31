#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::usart::{Config as UsartConfig, Uart, UartRx, UartTx};
use embassy_stm32::mode::Async as AsyncMode;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::rcc::{Hse, HseMode, Pll, PllMul, PllPreDiv, PllSource, AHBPrescaler, APBPrescaler};
use embassy_stm32::Config as Stm32Config;

use embedded_io_async::Write as _;
use embassy_time::Timer;

use {defmt_rtt as _, panic_probe as _};

embassy_stm32::bind_interrupts!(struct Irqs {
    USART1 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART1>;
    USART2 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART2>;
});

#[embassy_executor::task]
async fn bluetooth_to_washer_task(
    mut rx_from_bluetooth: UartRx<'static, AsyncMode>, // 从蓝牙接收
    mut tx_to_washer: UartTx<'static, AsyncMode>      // 向洗衣机发送
) {
    defmt::info!("Bluetooth to Washer forwarder task started.");
    let mut buf = [0u8; 128];

    loop {
        match rx_from_bluetooth.read_until_idle(&mut buf).await {
            Ok(len) if len > 0 => {
                defmt::debug!("Received {} bytes from Bluetooth: {:?}", len, &buf[0..len]);
                if let Err(e) = tx_to_washer.write_all(&buf[0..len]).await {
                    defmt::error!("Failed to write to washer UART: {:?}", defmt::Debug2Format(&e));
                } else {
                    defmt::debug!("Successfully forwarded to washer.");
                }
            }
            Ok(_) => { /* Idle timeout or 0 bytes */ }
            Err(e) => {
                defmt::error!("Bluetooth UART read error: {:?}", defmt::Debug2Format(&e));
                Timer::after_millis(20).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut stm32_config = Stm32Config::default();

    stm32_config.rcc.hse = Some(Hse {
        freq: Hertz(8_000_000),
        mode: HseMode::Oscillator,
    });
    stm32_config.rcc.pll = Some(Pll {
        src: PllSource::HSE,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL9,
    });
    // 基于之前Sysclk的错误信息和提供的枚举定义，我们将sys设置为HSE，
    // 并依赖init函数在PLL稳定后切换到PLL。
    stm32_config.rcc.sys = embassy_stm32::rcc::Sysclk::HSE; 
    stm32_config.rcc.ahb_pre = AHBPrescaler::DIV1;
    stm32_config.rcc.apb1_pre = APBPrescaler::DIV2;
    stm32_config.rcc.apb2_pre = APBPrescaler::DIV1;

    let p = embassy_stm32::init(stm32_config);
    defmt::info!("System Initialized. Target SYSCLK: 72 MHz (assumed from config).");

    let mut config_uart1 = UsartConfig::default();
    config_uart1.baudrate = 9600;
    let uart_jdy = Uart::new(
        p.USART1, p.PA10, p.PA9, Irqs, p.DMA1_CH4, p.DMA1_CH5, config_uart1
    ).unwrap();
    defmt::info!("USART1 (JDY-31 / Bluetooth) Initialized.");

    let mut config_uart2 = UsartConfig::default();
    config_uart2.baudrate = 2400; // 洗衣机波特率
    let uart_wash = Uart::new(
        p.USART2, p.PA3, p.PA2, Irqs, p.DMA1_CH7, p.DMA1_CH6, config_uart2
    ).unwrap();
    defmt::info!("USART2 (Washing Machine) Initialized.");

    // 假设 Uart::split() 在您的环境中返回 (UartTx, UartRx)
    let (_jdy_actual_tx, jdy_actual_rx) = uart_jdy.split();
    let (washer_actual_tx, _washer_actual_rx_unused) = uart_wash.split(); // washer_actual_rx 如果需要从洗衣机读取，则保留

    // 目标逻辑： 从蓝牙的Rx读取数据，然后发送到洗衣机的Tx
    // 所以任务需要： jdy_actual_rx (蓝牙的接收部分)
    //              washer_actual_tx (洗衣机的发送部分)
    spawner.spawn(bluetooth_to_washer_task(jdy_actual_rx, washer_actual_tx)).expect("Failed to spawn forwarder task");


    defmt::info!("Tasks spawned. Entering main loop.");
    let mut led = Output::new(p.PC13, Level::High, Speed::Low);
    loop {
        led.toggle();
        Timer::after_secs(1).await;
    }
}
