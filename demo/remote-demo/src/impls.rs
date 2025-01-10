// Implements `embedded_io_async` over poststation remote proxying

use std::{collections::VecDeque, time::Duration};

use poststation_sdk::{SquadClient, StreamListener};
use tokio::time::timeout;
use uartbridge_icd::{SetBaudrate, UartFrame, UartRecvTopic, UartSendTopic};


pub struct FakeSerial {
    client: SquadClient,
    serial: u64,
    in_queue: VecDeque<u8>,
    subs: StreamListener<UartRecvTopic>,
    seq_ctr: u32,
}

impl FakeSerial {
    pub async fn new(client: &SquadClient, serial: u64) -> Result<Self, String> {
        let subs = client.stream_topic::<UartRecvTopic>(serial).await?;
        Ok(Self {
            client: client.clone(),
            serial,
            in_queue: VecDeque::new(),
            subs,
            seq_ctr: 0,
        })
    }

    pub async fn set_baudrate(&mut self, baudrate: u32) -> Result<(), String> {
        self.seq_ctr = self.seq_ctr.wrapping_add(1);
        self.client
            .proxy_endpoint::<SetBaudrate>(self.serial, self.seq_ctr, &baudrate)
            .await
    }
}

#[derive(Debug)]
pub enum FSError {
    Other,
    ConnectionClosed,
}

impl embedded_io_async::Error for FSError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Self::Other => embedded_io_async::ErrorKind::Other,
            Self::ConnectionClosed => embedded_io_async::ErrorKind::ConnectionReset,
        }
    }
}

impl embedded_io_async::ErrorType for FakeSerial {
    type Error = FSError;
}

impl embedded_io_async::Read for FakeSerial {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if !self.in_queue.is_empty() {
            // Dump the current contents of in buf
            for (i, obyte) in buf.iter_mut().enumerate() {
                let Some(ibyte) = self.in_queue.pop_front() else {
                    return Ok(i);
                };
                *obyte = ibyte;
            }
            // We made it to the end, so we got len bytes. If the input buffer was
            // empty, Ok(0) is okay.
            return Ok(buf.len());
        }
        // We need to do a read/wait for data
        let mut used = 0;
        loop {
            let rfut = self.subs.recv();
            if used != 0 {
                // We have received some data already, only wait a little bit.
                // TODO: This should be replaced with a try_recv once we have that.
                let res = timeout(Duration::from_millis(1), rfut).await;
                match res {
                    Ok(Some(frame)) => {
                        // Cool, we got some data!
                        if used >= buf.len() {
                            // No room left, just put it all in the queue
                            for b in frame.data {
                                self.in_queue.push_back(b);
                            }
                        } else {
                            // Take what we can into the output buffer directly
                            let remain = &mut buf[used..];
                            let to_take = remain.len().min(frame.data.len());
                            remain[..to_take].copy_from_slice(&frame.data[..to_take]);
                            used += to_take;

                            // If there are any remainders, put that in the in queue
                            for b in &frame.data[to_take..] {
                                self.in_queue.push_back(*b);
                            }
                        }
                    }
                    Ok(None) | Err(_) => {
                        // EOF OR Timeout, BUT used is nonzero, so return what we have
                        return Ok(used);
                    }
                }
            } else {
                // No data yet, so this wait is unbounded
                if let Some(frame) = rfut.await {
                    // Cool, we got some data!
                    if used >= buf.len() {
                        // No room left, just put it all in the queue
                        for b in frame.data {
                            self.in_queue.push_back(b);
                        }
                    } else {
                        // Take what we can into the output buffer directly
                        let remain = &mut buf[used..];
                        let to_take = remain.len().min(frame.data.len());
                        remain[..to_take].copy_from_slice(&frame.data[..to_take]);
                        used += to_take;

                        // If there are any remainders, put that in the in queue
                        for b in &frame.data[to_take..] {
                            self.in_queue.push_back(*b);
                        }
                    }
                } else {
                    // EOF, and no data anywhere. Sorry buddy.
                    return Err(FSError::ConnectionClosed);
                }
            }
        }
    }
}

impl embedded_io_async::Write for FakeSerial {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // For now, cap a single send to 128 bytes max
        let to_send = buf.len().min(128);
        let frame = UartFrame {
            data: buf.iter().copied().take(to_send).collect(),
        };
        self.seq_ctr = self.seq_ctr.wrapping_add(1);
        let res = self
            .client
            .publish_topic::<UartSendTopic>(self.serial, self.seq_ctr, &frame)
            .await;
        match res {
            Ok(()) => Ok(to_send),
            Err(_) => {
                // I think this can only be "closed"?
                Err(FSError::ConnectionClosed)
            }
        }
    }
}
