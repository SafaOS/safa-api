use core::{net::Ipv4Addr, num::NonZero, time::Duration, usize};

use safa_abi::{
    consts::MAX_NAME_LENGTH,
    errors::ErrorStatus,
    poll::{PollEntry, PollEvents},
    sockets::{
        SockBindAbstractAddr, SockBindAddr, SockBindInetV4Addr, SockCreateKind, SockDomain,
        SockMsgFlags,
    },
};

use crate::syscalls::{self, types::Ri};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockKind {
    SeqPacket,
    Stream,
}

enum SockAddr {
    Abstract(([u8; MAX_NAME_LENGTH], usize)),
}

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
        target_ipv4_addr: Ipv4Addr,
        target_port: u16,
    ) -> Result<usize, ErrorStatus> {
        let addr = SockBindInetV4Addr::new(target_port, target_ipv4_addr);
        let addr_ref = unsafe { &*(&addr as *const SockBindInetV4Addr as *const SockBindAddr) };

        crate::syscalls::sockets::send_to(
            self.sock_resource,
            payload,
            SockMsgFlags::NONE,
            Some((addr_ref, size_of::<SockBindInetV4Addr>())),
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
    pub fn recv_from(&mut self, buffer: &mut [u8]) -> Result<usize, ErrorStatus> {
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

        crate::syscalls::io::read(self.sock_resource, 0, buffer)
    }

    /// Returns the socket resource identifier.
    pub const fn ri(&self) -> Ri {
        self.sock_resource
    }
}

/// Describes a Unix Socket Connection Builder
pub struct UnixSockConnectionBuilder {
    addr: SockAddr,
    non_blocking: bool,
    kind: SockKind,
}

impl UnixSockConnectionBuilder {
    /// Construct a local Unix Socket Connection that uses an abstract path
    pub fn from_abstract_path(path: &str) -> Result<Self, ()> {
        if path.len() > MAX_NAME_LENGTH {
            return Err(());
        }

        let mut raw_buf = [0u8; MAX_NAME_LENGTH];
        raw_buf[..path.len()].copy_from_slice(&path.as_bytes());
        let addr = SockAddr::Abstract((raw_buf, path.len()));
        Ok(Self {
            kind: SockKind::Stream,
            addr,
            non_blocking: false,
        })
    }

    /// Marks the connection as non-blocking if `non-blocking` was true
    pub const fn set_non_blocking(&mut self, non_blocking: bool) -> &mut Self {
        self.non_blocking = non_blocking;
        self
    }

    /// Sets the type of the connection to `kind`, by default we are using [`SockKind::Stream`]
    pub const fn set_type(&mut self, kind: SockKind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// Builds the final connection
    pub fn connect(self) -> Result<UnixSockConnection, ErrorStatus> {
        let mut flags = SockCreateKind::from_bits_retaining(0);

        if self.kind == SockKind::SeqPacket {
            flags = flags | SockCreateKind::SOCK_SEQPACKET;
        }

        if self.non_blocking {
            flags = flags | SockCreateKind::SOCK_NON_BLOCKING;
        }

        let (addr, addr_struct_size) = match self.addr {
            SockAddr::Abstract((name, len)) => (
                SockBindAbstractAddr::new(name),
                size_of::<SockBindAddr>() + len,
            ),
        };

        let addr: &SockBindAddr = unsafe { core::mem::transmute(&addr) };

        let socket_res = syscalls::sockets::create(SockDomain::LOCAL, flags, 0)?;
        let connection = syscalls::sockets::connect(socket_res, addr, addr_struct_size);

        let connection_ri = match connection {
            Ok(()) => socket_res,
            Err(e) => {
                syscalls::resources::destroy_resource(socket_res)
                    .expect("failed to destroy a socket descriptor");
                return Err(e);
            }
        };
        Ok(UnixSockConnection {
            inner_ri: connection_ri,
        })
    }
}

pub struct UnixSockConnection {
    inner_ri: Ri,
}

impl UnixSockConnection {
    /// Performs a read operation on this socket
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::read(self.inner_ri, 0, buf)
    }

    /// Performs a write operation on this socket
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::write(self.inner_ri, 0, buf)
    }

    /// Set the ability for the socket to block to `can_block`
    pub fn set_can_block(&mut self, can_block: bool) -> Result<(), ErrorStatus> {
        const SET_BLOCKING: u16 = 0;
        syscalls::io::io_command(self.inner_ri, SET_BLOCKING, can_block as u64)
    }

    /// The raw Resource ID of self
    pub const fn ri(&self) -> Ri {
        self.inner_ri
    }
}

impl Drop for UnixSockConnection {
    fn drop(&mut self) {
        syscalls::resources::destroy_resource(self.inner_ri)
            .expect("failed to destroy a UnixSockConnection");
    }
}

pub struct UnixListenerBuilder {
    addr: SockAddr,
    non_blocking: bool,
    kind: SockKind,
    backlog: usize,
}
impl UnixListenerBuilder {
    /// Construct a local Unix Socket Listener (Server Unix Socket) that uses an abstract path
    pub fn from_abstract_path(path: &str) -> Result<Self, ()> {
        if path.len() > MAX_NAME_LENGTH {
            return Err(());
        }

        let mut raw_buf = [0u8; MAX_NAME_LENGTH];
        raw_buf[..path.len()].copy_from_slice(&path.as_bytes());
        let addr = SockAddr::Abstract((raw_buf, path.len()));
        Ok(Self {
            kind: SockKind::Stream,
            addr,
            non_blocking: false,
            backlog: usize::MAX,
        })
    }

    /// Marks the connection as non-blocking if `non-blocking` was true
    pub const fn set_non_blocking(&mut self, non_blocking: bool) -> &mut Self {
        self.non_blocking = non_blocking;
        self
    }

    /// Sets the type of the connection to `kind`, by default we are using [`SockKind::Stream`]
    pub const fn set_type(&mut self, kind: SockKind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// Sets the max amount of connection this listener can accept to `backlog`
    pub const fn set_backlog(&mut self, backlog: usize) -> &mut Self {
        self.backlog = backlog;
        self
    }

    /// Builds and binds the final listener
    pub fn bind(self) -> Result<UnixListener, ErrorStatus> {
        let mut flags = SockCreateKind::from_bits_retaining(0);

        if self.kind == SockKind::SeqPacket {
            flags = flags | SockCreateKind::SOCK_SEQPACKET;
        }

        if self.non_blocking {
            flags = flags | SockCreateKind::SOCK_NON_BLOCKING;
        }

        let socket_res = syscalls::sockets::create(SockDomain::LOCAL, flags, 0)?;

        let (addr, addr_struct_size) = match self.addr {
            SockAddr::Abstract((name, len)) => (
                SockBindAbstractAddr::new(name),
                size_of::<SockBindAddr>() + len,
            ),
        };
        let addr: &SockBindAddr = unsafe { core::mem::transmute(&addr) };

        match syscalls::sockets::bind(socket_res, addr, addr_struct_size) {
            Ok(_) => {}
            Err(e) => {
                syscalls::resources::destroy_resource(socket_res)
                    .expect("failed to destroy a socket descriptor");
                return Err(e);
            }
        };

        match syscalls::sockets::listen(socket_res, self.backlog) {
            Ok(_) => {}
            Err(e) => {
                syscalls::resources::destroy_resource(socket_res)
                    .expect("failed to destroy a socket descriptor");
                return Err(e);
            }
        }

        Ok(UnixListener {
            inner_ri: socket_res,
        })
    }
}

/// A Server Unix Socket that can accept incoming connections
pub struct UnixListener {
    inner_ri: Ri,
}

impl UnixListener {
    /// Accepts 1 pending connection request, returns the Server's Side of the connection
    pub fn accept(&self) -> Result<UnixSockConnection, ErrorStatus> {
        let connection_ri = syscalls::sockets::accept(self.inner_ri, None)?;
        Ok({
            UnixSockConnection {
                inner_ri: connection_ri,
            }
        })
    }

    /// The raw resource ID of self
    pub const fn ri(&self) -> Ri {
        self.inner_ri
    }
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        syscalls::resources::destroy_resource(self.inner_ri)
            .expect("failed to destroy a UnixListener");
    }
}

#[cfg(feature = "std")]
mod _std {
    use std::io;
    use std::io::Read;
    use std::io::Write;

    impl Read for super::UnixSockConnection {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            super::UnixSockConnection::read(self, buf).map_err(|e| crate::errors::into_io_error(e))
        }
    }

    impl Write for super::UnixSockConnection {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            super::UnixSockConnection::write(self, buf).map_err(|e| crate::errors::into_io_error(e))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
