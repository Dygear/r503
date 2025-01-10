#![cfg_attr(not(feature = "use-std"), no_std)]

use postcard_rpc::{endpoints, topics, TopicDirection};
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SleepMillis {
    pub millis: u16,
}

#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SleptMillis {
    pub millis: u16,
}

#[cfg(not(feature = "use-std"))]
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct UartFrame<'a> {
    pub data: &'a [u8],
}

#[cfg(feature = "use-std")]
#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct UartFrame {
    pub data: Vec<u8>,
}

// ---

// Endpoints spoken by our device
//
// GetUniqueIdEndpoint is mandatory, the others are examples
endpoints! {
    list = ENDPOINT_LIST;
    | EndpointTy                | RequestTy     | ResponseTy            | Path                          |
    | ----------                | ---------     | ----------            | ----                          |
    | GetUniqueIdEndpoint       | ()            | u64                   | "poststation/unique_id/get"   |
    | RebootToPicoBoot          | ()            | ()                    | "template/picoboot/reset"     |
    | SleepEndpoint             | SleepMillis   | SleptMillis           | "template/sleep"              |
    | SetBaudrate               | u32           | ()                    | "uartbridge/baudrate/set"     |
    | GetBaudrate               | ()            | u32                   | "uartbridge/baudrate/get"     |
}

// incoming topics handled by our device
topics! {
    list = TOPICS_IN_LIST;
    direction = TopicDirection::ToServer;
    | TopicTy                   | MessageTy     | Path                      | Cfg                           |
    | -------                   | ---------     | ----                      | ---                           |
    | UartSendTopic             | UartFrame<'a> | "uartbridge/uart/send"    | cfg(not(feature = "use-std")) |
    | UartSendTopic             | UartFrame     | "uartbridge/uart/send"    | cfg(feature = "use-std")      |
}

// outgoing topics handled by our device
topics! {
    list = TOPICS_OUT_LIST;
    direction = TopicDirection::ToClient;
    | TopicTy                   | MessageTy     | Path                      | Cfg                           |
    | -------                   | ---------     | ----                      | ---                           |
    | UartRecvTopic             | UartFrame<'a> | "uartbridge/uart/recv"    | cfg(not(feature = "use-std")) |
    | UartRecvTopic             | UartFrame     | "uartbridge/uart/recv"    | cfg(feature = "use-std")      |
}
