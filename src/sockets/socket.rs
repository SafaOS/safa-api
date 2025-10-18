use core::ptr::NonNull;

use safa_abi::{
    errors::ErrorStatus,
    sockets::{SockMsgFlags, SocketAddr},
};

use crate::syscalls::{self, types::Ri};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum SocketOpt {
    /// Wetheher or not the socket can block.
    Blocking = 0,
    /// The number of maximum milliseconds a Read operation can wait for.
    ReadTimeout = 1,
    /// The number of maximum milliseconds a Write operation can wait for.
    WriteTimeout = 2,
    /// The time to live field in an Ip packet.
    IpTTL = 3,
    /// Broad cast permissions.
    IpBroadcast = 4,
    SocketError = 5,
}

/// Describes the kind of a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketKind {
    SeqPacket,
    Stream,
    Datagram,
}

/// Describes the domain of a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDomain {
    /// Local domain socket
    Local,
    /// Internet domain socket
    Ipv4,
}

/// Represents a socket.
#[derive(Debug)]
pub struct Socket(Ri);

impl Drop for Socket {
    fn drop(&mut self) {
        // TODO: Resources high-level wrapper
        syscalls::resources::destroy_resource(self.0).expect("Failed to drop socket")
    }
}

/// Represents a builder for creating sockets.
#[derive(Debug, Clone, Copy)]
pub struct SocketBuilder {
    domain: SocketDomain,
    kind: SocketKind,
    protocol: u32,
    can_block: bool,
}

impl SocketBuilder {
    pub const fn new(domain: SocketDomain, kind: SocketKind, protocol: u32) -> Self {
        Self {
            domain,
            kind,
            protocol,
            can_block: true,
        }
    }

    pub const fn set_non_blocking(&mut self, non_blocking: bool) -> &mut Self {
        self.can_block = !non_blocking;
        self
    }

    pub const fn set_kind(&mut self, kind: SocketKind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub const fn set_protocol(&mut self, protocol: u32) -> &mut Self {
        self.protocol = protocol;
        self
    }

    pub fn build(self) -> Result<Socket, ErrorStatus> {
        use safa_abi::sockets::SockCreateKind as AbiSocketCreateKind;
        use safa_abi::sockets::SockDomain as AbiSocketDomain;

        let domain = match self.domain {
            SocketDomain::Ipv4 => AbiSocketDomain::INETV4,
            SocketDomain::Local => AbiSocketDomain::LOCAL,
        };

        let protocol = self.protocol;
        let mut kind = match self.kind {
            SocketKind::Datagram => AbiSocketCreateKind::SOCK_DGRAM,
            SocketKind::Stream => AbiSocketCreateKind::SOCK_STREAM,
            SocketKind::SeqPacket => AbiSocketCreateKind::SOCK_SEQPACKET,
        };

        if !self.can_block {
            kind = kind | AbiSocketCreateKind::SOCK_NON_BLOCKING;
        }

        syscalls::sockets::create(domain, kind, protocol).map(|ri| Socket(ri))
    }
}

impl Socket {
    /// Returns a new socket builder.
    pub const fn builder(domain: SocketDomain, kind: SocketKind, protocol: u32) -> SocketBuilder {
        SocketBuilder::new(domain, kind, protocol)
    }

    /// Wrapper around [`syscalls::sockets::listen`], configures the socket to listen for incoming connections.
    #[inline]
    pub fn listen(&self, backlog: usize) -> Result<(), ErrorStatus> {
        syscalls::sockets::listen(self.0, backlog)
    }

    /// Wrapper around [`syscalls::sockets::bind`], binds the socket to a specific address.
    #[inline]
    pub fn bind(&self, addr: &SocketAddr, size: usize) -> Result<(), ErrorStatus> {
        syscalls::sockets::bind(self.0, addr, size)
    }

    /// Wrapper around [`syscalls::sockets::connect`], connects the socket to an address.
    #[inline]
    pub fn connect(&self, addr: &SocketAddr, size: usize) -> Result<(), ErrorStatus> {
        syscalls::sockets::connect(self.0, &addr, size)
    }

    /// Wrapper around [`syscalls::sockets::send_to`], sends data with flags to a specific address or to the connected address.
    #[inline]
    pub fn send_to(
        &self,
        buf: &[u8],
        flags: SockMsgFlags,
        addr: Option<(&SocketAddr, usize)>,
    ) -> Result<usize, ErrorStatus> {
        syscalls::sockets::send_to(self.0, buf, flags, addr)
    }

    /// Same as [`send_to`] but sends data to the connected socket only.
    #[inline]
    pub fn send(&self, buf: &[u8], flags: SockMsgFlags) -> Result<usize, ErrorStatus> {
        self.send_to(buf, flags, None)
    }

    /// Wrapper around [`syscalls::sockets::recv_from`], receives data with flags
    /// and returns the senders address if `retrieve_addr` is true and it is available.
    #[inline]
    fn recv_from_inner(
        &self,
        buf: &mut [u8],
        flags: SockMsgFlags,
        store_addr: Option<&mut (NonNull<SocketAddr>, usize)>,
    ) -> Result<usize, ErrorStatus> {
        let results = syscalls::sockets::recv_from(self.0, buf, flags, store_addr)?;
        Ok(results)
    }

    /// Receives a message from the socket, storing the senders address if possible in `store_addr` and returns the amount of bytes received.
    ///
    /// Wrapper around [`syscalls::sockets::recv_from`].
    #[inline]
    pub fn recv_from(
        &self,
        buf: &mut [u8],
        flags: SockMsgFlags,
        store_addr: &mut (NonNull<SocketAddr>, usize),
    ) -> Result<usize, ErrorStatus> {
        self.recv_from_inner(buf, flags, Some(store_addr))
    }

    /// Same as [`Self::recv_from`] but doesn't return the sender's address.
    #[inline]
    pub fn recv(&self, buf: &mut [u8], flags: SockMsgFlags) -> Result<usize, ErrorStatus> {
        self.recv_from_inner(buf, flags, None)
    }

    /// Wrapper around [`syscalls::sockets::accept`]
    fn accept_inner(
        &self,
        store_addr: Option<&mut (NonNull<SocketAddr>, usize)>,
    ) -> Result<Socket, ErrorStatus> {
        let results = syscalls::sockets::accept(self.0, store_addr)?;
        let results = Socket(results);

        Ok(results)
    }

    /// Accepts a new connection from this socket.
    ///
    /// Wrapper around [`syscalls::sockets::accept`].
    pub fn accept(&self) -> Result<Socket, ErrorStatus> {
        self.accept_inner(None)
    }

    /// Accepts a new connection from this socket returning the accepted socket and the address of the remote peer if it is available.
    ///
    /// Wrapper around [`syscalls::sockets::accept`].
    pub fn accept_from(
        &self,
        store_addr: &mut (NonNull<SocketAddr>, usize),
    ) -> Result<Socket, ErrorStatus> {
        self.accept_inner(Some(store_addr))
    }

    /// Wrapper around [`syscalls::io::read`].
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::read(self.0, 0, buf)
    }

    /// Wrapper around [`syscalls::io::write`].
    pub fn write(&self, buf: &[u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::write(self.0, 0, buf)
    }

    pub fn io_cmd(&self, cmd: u16, arg: u64) -> Result<(), ErrorStatus> {
        syscalls::io::io_command(self.0, cmd, arg)
    }

    pub fn set_sock_opt<T: Into<u64>>(&self, opt: SocketOpt, arg: T) -> Result<(), ErrorStatus> {
        self.io_cmd(opt as u16, arg.into())
    }

    /// Safety: the pointer is verified by the kernel to be aligned, however if you pass the wrong type, it will cause undefined behavior.
    pub unsafe fn get_sock_opt<T>(&self, opt: SocketOpt, arg: &mut T) -> Result<(), ErrorStatus> {
        self.io_cmd(opt as u16 & (1 << 15), arg as *mut T as u64)
    }

    /// Configures the socket to block when necessary.
    pub fn set_blocking(&self, blocking: bool) -> Result<(), ErrorStatus> {
        self.set_sock_opt(SocketOpt::Blocking, blocking)
    }

    /// Returns the raw socket resource identifier.
    pub const fn ri(&self) -> Ri {
        self.0
    }
}
