use core::{net::Ipv4Addr, num::NonZero, ptr::NonNull, time::Duration};

use safa_abi::{
    errors::ErrorStatus,
    poll::{PollEntry, PollEvents},
    sockets::{SockBindAddr, SockBindInetV4Addr, SockCreateKind, SockDomain, SockMsgFlags},
};

use crate::syscalls::{self, types::Ri};

/// Represents a UDP Datagram connectionless socket
pub struct UDPSocket {
    sock_resource: Ri,
    write_timeout: Option<NonZero<u64>>,
    read_timeout: Option<NonZero<u64>>,
    can_block: bool,
}

impl UDPSocket {
    /// Create a new UDP socket
    pub fn create() -> Result<Self, ErrorStatus> {
        let sock_resource =
            syscalls::sockets::create(SockDomain::INETV4, SockCreateKind::SOCK_DGRAM, 0)?;
        Ok(Self {
            sock_resource,
            write_timeout: None,
            read_timeout: None,
            can_block: true,
        })
    }

    /// Set the write timeout for the socket, only milliseconds are taken.
    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) {
        self.write_timeout = timeout.and_then(|t| NonZero::new(t.as_millis() as u64));
    }

    /// Set the read timeout for the socket, only milliseconds are taken.
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout.and_then(|t| NonZero::new(t.as_millis() as u64));
    }

    pub fn write_timeout(&self) -> Option<Duration> {
        self.write_timeout.map(|t| Duration::from_millis(t.get()))
    }

    pub fn read_timeout(&self) -> Option<Duration> {
        self.read_timeout.map(|t| Duration::from_millis(t.get()))
    }

    /// Binds the socket to the specified IP address and port, use [`Ipv4Addr::UNSPECIFIED`] to bind to all available interfaces.
    pub fn bind_at(&self, ipv4_addr: Ipv4Addr, port: u16) -> Result<(), ErrorStatus> {
        let addr = SockBindInetV4Addr::new(port, ipv4_addr);
        let addr_ref = unsafe { &*(&addr as *const SockBindInetV4Addr as *const SockBindAddr) };

        syscalls::sockets::bind(
            self.sock_resource,
            addr_ref,
            size_of::<SockBindInetV4Addr>(),
        )
    }

    /// Sends payload [`payload`] to the IP [`target_ipv4_addr`] and the port [`target_port`].
    pub fn send_to(
        &self,
        payload: &[u8],
        flags: SockMsgFlags,
        target: Option<(Ipv4Addr, u16)>,
    ) -> Result<usize, ErrorStatus> {
        let addr = target.map(|(target_ipv4_addr, target_port)| {
            SockBindInetV4Addr::new(target_port, target_ipv4_addr)
        });
        let addr_ref = addr
            .as_ref()
            .map(|addr| unsafe { &*(addr as *const SockBindInetV4Addr as *const SockBindAddr) });

        crate::syscalls::sockets::send_to(
            self.sock_resource,
            payload,
            flags,
            addr_ref.map(|addr| (addr, size_of::<SockBindInetV4Addr>())),
        )
    }

    /// Set the ability for the socket to block to `can_block`
    pub fn set_can_block(&mut self, can_block: bool) -> Result<(), ErrorStatus> {
        const SET_BLOCKING: u16 = 0;
        syscalls::io::io_command(self.sock_resource, SET_BLOCKING, can_block as u64)?;
        self.can_block = can_block;
        Ok(())
    }

    /// Receives payload from the socket and stores it in [`buffer`], returns the amount of bytes received.
    pub fn recv_from(
        &mut self,
        flags: SockMsgFlags,
        buffer: &mut [u8],
        source: Option<&mut (Ipv4Addr, u16)>,
    ) -> Result<usize, ErrorStatus> {
        if let Some(timeout) = self.read_timeout() {
            let mut entries = [PollEntry::new(
                self.sock_resource,
                PollEvents::DATA_AVAILABLE,
            )];

            syscalls::io::poll_resources(&mut entries, Some(timeout))?;
            let entry = entries[0];
            let r_events = entry.returned_events();

            if r_events.contains(PollEvents::DISCONNECTED) {
                return Err(ErrorStatus::ConnectionClosed);
            }

            if !r_events.contains(PollEvents::DATA_AVAILABLE) {
                return Err(ErrorStatus::Timeout);
            }
        }

        let mut source_addr = source.as_ref().map(|(source_ipv4_addr, source_port)| {
            SockBindInetV4Addr::new(*source_port, *source_ipv4_addr)
        });

        let addr_ref = source_addr
            .as_mut()
            .map(|addr| unsafe { &mut *(addr as *mut SockBindInetV4Addr as *mut SockBindAddr) });
        let mut source_addr_results_ref = addr_ref.map(|addr| {
            (
                unsafe { NonNull::new_unchecked(addr) },
                size_of::<SockBindInetV4Addr>(),
            )
        });
        let results = crate::syscalls::sockets::recv_from(
            self.sock_resource,
            buffer,
            flags,
            source_addr_results_ref.as_mut(),
        )?;

        if let Some((_, size)) = source_addr_results_ref {
            let Some(s_addr_info) = source else {
                unreachable!()
            };

            let Some(addr) = source_addr else {
                unreachable!()
            };
            if size == size_of::<SockBindInetV4Addr>() {
                *s_addr_info = (addr.ip, addr.port);
            }
        }

        Ok(results)
    }

    /// Returns the socket resource identifier.
    pub const fn ri(&self) -> Ri {
        self.sock_resource
    }
}
