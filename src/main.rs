#![no_main]
#![no_std]

mod pins;

use ariel_os::gpio::{Input, Level, Output, Pull};
use ariel_os::{debug::log::*, net, reexports::embassy_net, time::Duration};
use core::str::FromStr;
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Write;

#[ariel_os::task(autostart, peripherals)]
async fn tcp_sender(peripherals: pins::Peripherals) {
    #[allow(unused_variables)]
    let pull = Pull::Up;
    #[cfg(context = "st-nucleo-h755zi-q")]
    let pull = Pull::None;

    let mut btn1 = Input::builder(peripherals.btn1, pull)
        .build_with_interrupt()
        .unwrap();
    let stack = net::network_stack().await.unwrap();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let addr = "10.42.0.1:1234"; // IPv4 🔔
    let other_board = embassy_net::IpEndpoint::from_str(addr).unwrap();

    loop {
        btn1.wait_for_low().await;

        info!("Button pressed, sending message...");

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.connect(other_board).await.unwrap();
        info!("Connected to {:?}", other_board);

        let msg = "T"; // Toggle LED
        let msg_bytes = msg.as_bytes();

        match socket.write_all(msg_bytes).await {
            Ok(()) => {
                info!("Sent toggle command");
            }
            Err(e) => {
                info!("write error: {:?}", e);
                continue;
            }
        }
    }
}

#[ariel_os::task(autostart, peripherals)]
async fn tcp_recieve(peripherals: pins::Peripherals) {
    let mut led = Output::new(peripherals.led1, Level::Low);
    let stack = net::network_stack().await.unwrap();
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            info!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    info!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    info!("read error: {:?}", e);
                    break;
                }
            };
            let buf_str = core::str::from_utf8(&buf[..n]).unwrap_or("invalid utf-8");

            info!("Received: {:?}", buf_str);

            if buf_str.starts_with("T") {
                info!("Toggling LED");
                led.toggle();
            }
        }
    }
}
