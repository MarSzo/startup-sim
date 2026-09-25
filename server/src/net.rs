//! UDP socket wrapper with an optional network-condition simulator
//! (one-way delay, jitter and packet loss applied in both directions).

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default)]
pub struct LinkConditions {
    /// One-way delay added to every packet, each direction (RTT grows by 2x).
    pub lag: Duration,
    /// Extra random delay in `0..=jitter` (may reorder packets, like real UDP).
    pub jitter: Duration,
    /// Drop probability per packet, each direction (0.0..1.0).
    pub loss: f64,
}

impl LinkConditions {
    pub fn is_ideal(&self) -> bool {
        self.lag.is_zero() && self.jitter.is_zero() && self.loss <= 0.0
    }
}

type Queued = Reverse<(Instant, u64, SocketAddr, Vec<u8>)>;

pub struct Net {
    socket: UdpSocket,
    cond: LinkConditions,
    rng: fastrand::Rng,
    seq: u64,
    inbound: BinaryHeap<Queued>,
    outbound: BinaryHeap<Queued>,
    buf: Vec<u8>,
    pub bytes_out: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub packets_in: u64,
    pub dropped: u64,
}

impl Net {
    pub fn bind(addr: SocketAddr, cond: LinkConditions) -> io::Result<Net> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Net {
            socket,
            cond,
            rng: fastrand::Rng::new(),
            seq: 0,
            inbound: BinaryHeap::new(),
            outbound: BinaryHeap::new(),
            buf: vec![0; 2048],
            bytes_out: 0,
            packets_out: 0,
            bytes_in: 0,
            packets_in: 0,
            dropped: 0,
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    fn delay(&mut self) -> Option<Duration> {
        if self.cond.loss > 0.0 && self.rng.f64() < self.cond.loss {
            self.dropped += 1;
            return None;
        }
        let jitter_us = self.cond.jitter.as_micros() as u64;
        let j = if jitter_us > 0 { self.rng.u64(0..=jitter_us) } else { 0 };
        Some(self.cond.lag + Duration::from_micros(j))
    }

    pub fn send(&mut self, addr: SocketAddr, data: Vec<u8>) {
        self.bytes_out += data.len() as u64;
        self.packets_out += 1;
        if self.cond.is_ideal() {
            let _ = self.socket.send_to(&data, addr);
            return;
        }
        if let Some(d) = self.delay() {
            self.seq += 1;
            self.outbound.push(Reverse((Instant::now() + d, self.seq, addr, data)));
        }
    }

    /// Send outbound packets whose simulated delay has elapsed.
    pub fn flush(&mut self, now: Instant) {
        while let Some(Reverse((t, ..))) = self.outbound.peek() {
            if *t > now {
                break;
            }
            let Reverse((_, _, addr, data)) = self.outbound.pop().unwrap();
            let _ = self.socket.send_to(&data, addr);
        }
    }

    /// Earliest time a queued packet (either direction) becomes due.
    pub fn next_release(&self) -> Option<Instant> {
        let a = self.inbound.peek().map(|Reverse((t, ..))| *t);
        let b = self.outbound.peek().map(|Reverse((t, ..))| *t);
        match (a, b) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Wait up to `timeout` for one datagram and queue it (with simulated delay).
    pub fn recv(&mut self, timeout: Duration) {
        let timeout = timeout.max(Duration::from_micros(100));
        if self.socket.set_read_timeout(Some(timeout)).is_err() {
            return;
        }
        match self.socket.recv_from(&mut self.buf) {
            Ok((n, addr)) => {
                self.bytes_in += n as u64;
                self.packets_in += 1;
                let data = self.buf[..n].to_vec();
                let d = if self.cond.is_ideal() { Some(Duration::ZERO) } else { self.delay() };
                if let Some(d) = d {
                    self.seq += 1;
                    self.inbound.push(Reverse((Instant::now() + d, self.seq, addr, data)));
                }
            }
            Err(_) => {} // timeout, or ICMP port-unreachable reported as error on some OSes
        }
    }

    /// Pop the next inbound packet whose simulated delay has elapsed.
    pub fn pop_inbound(&mut self, now: Instant) -> Option<(SocketAddr, Vec<u8>)> {
        match self.inbound.peek() {
            Some(Reverse((t, ..))) if *t <= now => {
                let Reverse((_, _, addr, data)) = self.inbound.pop().unwrap();
                Some((addr, data))
            }
            _ => None,
        }
    }
}
