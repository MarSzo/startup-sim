//! UDP socket wrapper with an optional network-condition simulator
//! (one-way delay, jitter and packet loss applied in both directions).

use std::cmp::Reverse;
use std::collections::binary_heap::{BinaryHeap, PeekMut};
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

/// Bind a UDP socket. An IPv6 wildcard (`[::]`) is made dual-stack so IPv4
/// clients are accepted too (they appear as `::ffff:a.b.c.d`); Windows and
/// some BSDs default to IPv6-only, so the flag is set explicitly.
pub fn bind_udp(addr: SocketAddr) -> io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
    if addr.is_ipv6() {
        socket.set_only_v6(false)?;
    }
    socket.bind(&addr.into())?;
    Ok(socket.into())
}

/// Normalize `::ffff:a.b.c.d` to plain IPv4 (for logs and address comparisons).
pub fn canonical(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => match v6.ip().to_ipv4_mapped() {
            Some(v4) => SocketAddr::new(v4.into(), v6.port()),
            None => addr,
        },
        v4 => v4,
    }
}

type Queued = Reverse<(Instant, u64, SocketAddr, Vec<u8>)>;

/// Shortest socket wait (a zero timeout would mean "block forever").
const MIN_WAIT: Duration = Duration::from_micros(100);

/// Packets held back by the link simulator, per direction, at most.
const MAX_QUEUED: usize = 1 << 16;

pub struct Net {
    socket: UdpSocket,
    cond: LinkConditions,
    rng: fastrand::Rng,
    seq: u64,
    inbound: BinaryHeap<Queued>,
    outbound: BinaryHeap<Queued>,
    buf: Vec<u8>,
    /// Read timeout currently set on the socket (a syscall to change).
    read_timeout: Option<Duration>,
    pub bytes_out: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub packets_in: u64,
    pub dropped: u64,
}

impl Net {
    pub fn bind(addr: SocketAddr, cond: LinkConditions) -> io::Result<Net> {
        let socket = bind_udp(addr)?;
        Ok(Net {
            socket,
            cond,
            rng: fastrand::Rng::new(),
            seq: 0,
            inbound: BinaryHeap::new(),
            outbound: BinaryHeap::new(),
            buf: vec![0; 2048],
            read_timeout: None,
            bytes_out: 0,
            packets_out: 0,
            bytes_in: 0,
            packets_in: 0,
            dropped: 0,
        })
    }

    /// Zero the traffic counters (after a stats report).
    pub fn reset_counters(&mut self) {
        self.bytes_out = 0;
        self.packets_out = 0;
        self.bytes_in = 0;
        self.packets_in = 0;
        self.dropped = 0;
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
            if self.outbound.len() >= MAX_QUEUED {
                self.dropped += 1; // simulated link flooded
                return;
            }
            self.seq += 1;
            self.outbound.push(Reverse((Instant::now() + d, self.seq, addr, data)));
        }
    }

    /// Send outbound packets whose simulated delay has elapsed.
    pub fn flush(&mut self, now: Instant) {
        while let Some(top) = self.outbound.peek_mut() {
            if top.0 .0 > now {
                break;
            }
            let Reverse((_, _, addr, data)) = PeekMut::pop(top);
            // Best effort, like UDP itself (e.g. ICMP unreachable on some OSes).
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
        // Whole milliseconds, rounded down (waking a little early is fine,
        // late is not), so bursts of packets reuse the same setting instead
        // of a setsockopt per datagram.
        let timeout = match timeout.as_millis() {
            0 => MIN_WAIT,
            ms => Duration::from_millis(u64::try_from(ms).unwrap_or(u64::MAX)),
        };
        if self.read_timeout != Some(timeout) {
            if self.socket.set_read_timeout(Some(timeout)).is_err() {
                return;
            }
            self.read_timeout = Some(timeout);
        }
        // Err: timeout, or ICMP port-unreachable reported as error on some OSes.
        let Ok((n, addr)) = self.socket.recv_from(&mut self.buf) else { return };
        self.bytes_in += n as u64;
        self.packets_in += 1;
        let d = if self.cond.is_ideal() { Some(Duration::ZERO) } else { self.delay() };
        if let Some(d) = d {
            if self.inbound.len() >= MAX_QUEUED {
                self.dropped += 1; // simulated link flooded
                return;
            }
            self.seq += 1;
            self.inbound.push(Reverse((Instant::now() + d, self.seq, addr, self.buf[..n].to_vec())));
        }
    }

    /// Pop the next inbound packet whose simulated delay has elapsed.
    pub fn pop_inbound(&mut self, now: Instant) -> Option<(SocketAddr, Vec<u8>)> {
        let top = self.inbound.peek_mut().filter(|top| top.0 .0 <= now)?;
        let Reverse((_, _, addr, data)) = PeekMut::pop(top);
        Some((addr, data))
    }
}
