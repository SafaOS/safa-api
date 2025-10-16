use core::{net::Ipv4Addr, ptr::NonNull};

use safa_abi::{
    consts::MAX_NAME_LENGTH,
    errors::ErrorStatus,
    sockets::{SockBindAbstractAddr, SockBindAddr, SockBindInetV4Addr, SockMsgFlags},
};

use crate::syscalls::{self, types::Ri};

enum AbiSocketAddr {
    Abstract((SockBindAbstractAddr, usize)),
    Ipv4(SockBindInetV4Addr),
}

impl AbiSocketAddr {
    pub fn as_ref(&self) -> (&SockBindAddr, usize) {
        match self {
            AbiSocketAddr::Abstract((addr, name_len)) => (
                unsafe { &*(addr as *const SockBindAbstractAddr as *const SockBindAddr) },
                name_len + size_of::<SockBindAddr>(),
            ),
            AbiSocketAddr::Ipv4(addr) => (
                unsafe { &*(addr as *const SockBindInetV4Addr as *const SockBindAddr) },
                size_of::<SockBindInetV4Addr>(),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketAddr {
    Local([u8; MAX_NAME_LENGTH], usize),
    Ipv4 { addr: Ipv4Addr, port: u16 },
}

impl SocketAddr {
    pub fn new_local(name: &str) -> Self {
        assert!(name.len() <= MAX_NAME_LENGTH);

        let mut bytes = [0; MAX_NAME_LENGTH];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self::Local(bytes, name.len())
    }

    pub const fn new_ipv4(addr: Ipv4Addr, port: u16) -> Self {
        Self::Ipv4 { addr, port }
    }

    fn into_abi(self) -> AbiSocketAddr {
        match self {
            SocketAddr::Local(name, len) => {
                AbiSocketAddr::Abstract((SockBindAbstractAddr::new(name), len))
            }
            SocketAddr::Ipv4 { addr, port } => {
                AbiSocketAddr::Ipv4(SockBindInetV4Addr::new(port, addr))
            }
        }
    }

    pub fn from_bytes(u32: &[u32]) -> Self {
        let size = u32.len() * size_of::<u32>();
        assert!(size >= size_of::<SockBindAddr>());
        let sock_bind_addr_ptr = u32.as_ptr().cast::<SockBindAddr>();

        assert!(sock_bind_addr_ptr.is_aligned());
        let as_sock_bind_addr = unsafe { &*sock_bind_addr_ptr };

        match as_sock_bind_addr.kind {
            SockBindAbstractAddr::KIND => {
                let abstract_name = unsafe { &*sock_bind_addr_ptr.cast::<SockBindAbstractAddr>() };
                Self::Local(abstract_name.name, size - size_of::<SockBindAddr>())
            }
            SockBindInetV4Addr::KIND => {
                let inet_v4_addr = unsafe { &*sock_bind_addr_ptr.cast::<SockBindInetV4Addr>() };
                Self::Ipv4 {
                    addr: inet_v4_addr.ip,
                    port: inet_v4_addr.port,
                }
            }
            k => unreachable!("Unimplemented socket address kind: {k}"),
        }
    }
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
            SocketKind::Stream => AbiSocketCreateKind::from_bits_retaining(0), /* FIXME: AbiSocketCreateKind::SOCK_STREAM */
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
    pub fn bind(&self, addr: SocketAddr) -> Result<(), ErrorStatus> {
        let abi_addr = addr.into_abi();
        let (abi_ref, abi_size) = abi_addr.as_ref();
        syscalls::sockets::bind(self.0, abi_ref, abi_size)
    }

    /// Wrapper around [`syscalls::sockets::connect`], connects the socket to an address.
    #[inline]
    pub fn connect(&self, addr: SocketAddr) -> Result<(), ErrorStatus> {
        let abi_addr = addr.into_abi();
        let (abi_ref, abi_size) = abi_addr.as_ref();
        syscalls::sockets::connect(self.0, abi_ref, abi_size)
    }

    /// Wrapper around [`syscalls::sockets::send_to`], sends data with flags to a specific address or to the connected address.
    #[inline]
    pub fn send_to(
        &self,
        buf: &[u8],
        flags: SockMsgFlags,
        addr: Option<SocketAddr>,
    ) -> Result<usize, ErrorStatus> {
        let abi_addr = addr.map(|a| a.into_abi());
        let abi_ref = abi_addr.as_ref().map(|a| a.as_ref());
        syscalls::sockets::send_to(self.0, buf, flags, abi_ref)
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
        retrieve_addr: bool,
    ) -> Result<(usize, Option<SocketAddr>), ErrorStatus> {
        let mut abi_addr = retrieve_addr.then(|| [0u32; size_of::<SockBindAbstractAddr>() / 4]);
        let mut abi_ref = abi_addr.as_mut().map(|a| {
            (
                NonNull::new(a).unwrap().cast::<SockBindAddr>(),
                size_of::<SockBindAbstractAddr>(),
            )
        });
        let results = syscalls::sockets::recv_from(self.0, buf, flags, abi_ref.as_mut())?;

        let received_from = match abi_ref {
            None | Some((_, 0)) => None,
            Some((_, size)) => Some(SocketAddr::from_bytes(&abi_addr.unwrap()[..size / 4])),
        };
        Ok((results, received_from))
    }

    /// Receives a message from the socket, returning the senders address if possible and the amount of bytes received.
    ///
    /// Wrapper around [`syscalls::sockets::recv_from`].
    #[inline]
    pub fn recv_from(
        &self,
        buf: &mut [u8],
        flags: SockMsgFlags,
    ) -> Result<(usize, Option<SocketAddr>), ErrorStatus> {
        self.recv_from_inner(buf, flags, true)
    }

    /// Same as [`Self::recv_from`] but doesn't return the sender's address.
    #[inline]
    pub fn recv(&self, buf: &mut [u8], flags: SockMsgFlags) -> Result<usize, ErrorStatus> {
        self.recv_from_inner(buf, flags, false)
            .map(|(received, a)| {
                assert!(a.is_none());
                received
            })
    }

    /// Wrapper around [`syscalls::sockets::accept`]
    fn accept_inner(
        &self,
        retrieve_addr: bool,
    ) -> Result<(Socket, Option<SocketAddr>), ErrorStatus> {
        let mut abi_addr = retrieve_addr.then(|| [0u32; size_of::<SockBindAbstractAddr>() / 4]);
        let mut abi_ref = abi_addr.as_mut().map(|a| {
            (
                NonNull::new(a).unwrap().cast::<SockBindAddr>(),
                size_of::<SockBindAbstractAddr>(),
            )
        });
        let results = syscalls::sockets::accept(self.0, abi_ref.as_mut())?;
        let results = Socket(results);
        let accepted_from = match abi_ref {
            None | Some((_, 0)) => None,
            Some((_, size)) => Some(SocketAddr::from_bytes(&abi_addr.unwrap()[..size / 4])),
        };

        Ok((results, accepted_from))
    }

    /// Accepts a new connection from this socket.
    ///
    /// Wrapper around [`syscalls::sockets::accept`].
    pub fn accept(&self) -> Result<Socket, ErrorStatus> {
        self.accept_inner(false).map(|(socket, a)| {
            assert!(a.is_none());
            socket
        })
    }

    /// Accepts a new connection from this socket returning the accepted socket and the address of the remote peer if it is available.
    ///
    /// Wrapper around [`syscalls::sockets::accept`].
    pub fn accept_from(&self) -> Result<(Socket, Option<SocketAddr>), ErrorStatus> {
        self.accept_inner(true)
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

    /// Configures the socket to block when necessary.
    pub fn set_blocking(&self, blocking: bool) -> Result<(), ErrorStatus> {
        const SET_BLOCKING: u16 = 0;
        self.io_cmd(SET_BLOCKING, blocking as u64)
    }

    /// Returns the raw socket resource identifier.
    pub const fn ri(&self) -> Ri {
        self.0
    }
}
