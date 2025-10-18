use safa_abi::{
    consts::MAX_NAME_LENGTH,
    errors::ErrorStatus,
    sockets::{LocalSocketAddr, SockMsgFlags, ToSocketAddr},
};

use crate::{sockets::Socket, syscalls::types::Ri};

/// Describes the kind of a local domain socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnixSockKind {
    SeqPacket,
    Stream,
}

enum SockAddr<'a> {
    Abstract(&'a str),
}

// Describes a Unix Socket Connection Builder
pub struct UnixSockConnectionBuilder<'a> {
    addr: SockAddr<'a>,
    non_blocking: bool,
    kind: UnixSockKind,
}

impl<'a> UnixSockConnectionBuilder<'a> {
    /// Construct a local Unix Socket Connection that uses an abstract path
    pub fn from_abstract_path(path: &'a str) -> Result<Self, ()> {
        if path.len() > MAX_NAME_LENGTH {
            return Err(());
        }

        let addr = SockAddr::Abstract(path);
        Ok(Self {
            kind: UnixSockKind::Stream,
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
    pub const fn set_type(&mut self, kind: UnixSockKind) -> &mut Self {
        self.kind = kind;
        self
    }

    /// Builds the final connection
    pub fn connect(self) -> Result<UnixSockConnection, ErrorStatus> {
        let domain = super::SocketDomain::Local;
        let kind = match self.kind {
            UnixSockKind::SeqPacket => super::SocketKind::SeqPacket,
            UnixSockKind::Stream => super::SocketKind::Stream,
        };

        let socket = Socket::builder(domain, kind, 0)
            .set_non_blocking(self.non_blocking)
            .build()?;

        let (addr, size) = match self.addr {
            SockAddr::Abstract(path) => LocalSocketAddr::new_abstract_from(path),
        };
        socket.connect(addr.as_generic(), size)?;

        Ok(UnixSockConnection(socket))
    }
}

pub struct UnixSockConnection(Socket);

impl UnixSockConnection {
    /// Performs a read operation on this socket
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        self.0.read(buf)
    }

    /// Performs a peek operation on this socket, doesn't consume the data...
    pub fn peek(&mut self, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        self.0.recv(buf, SockMsgFlags::PEEK)
    }

    /// Performs a write operation on this socket
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorStatus> {
        self.0.write(buf)
    }

    /// Set the ability for the socket to block to `can_block`
    pub fn set_can_block(&mut self, can_block: bool) -> Result<(), ErrorStatus> {
        self.0.set_blocking(can_block)
    }

    /// The raw Resource ID of self
    pub const fn ri(&self) -> Ri {
        self.0.ri()
    }

    pub const fn raw_socket(&self) -> &Socket {
        &self.0
    }
}

pub struct UnixListenerBuilder<'a> {
    addr: SockAddr<'a>,
    non_blocking: bool,
    kind: UnixSockKind,
    backlog: usize,
}
impl<'a> UnixListenerBuilder<'a> {
    /// Construct a local Unix Socket Listener (Server Unix Socket) that uses an abstract path
    pub fn from_abstract_path(path: &'a str) -> Result<Self, ()> {
        if path.len() > MAX_NAME_LENGTH {
            return Err(());
        }

        let addr = SockAddr::Abstract(path);
        Ok(Self {
            kind: UnixSockKind::Stream,
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
    pub const fn set_type(&mut self, kind: UnixSockKind) -> &mut Self {
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
        let domain = super::SocketDomain::Local;
        let kind = match self.kind {
            UnixSockKind::SeqPacket => super::SocketKind::SeqPacket,
            UnixSockKind::Stream => super::SocketKind::Stream,
        };

        let socket = Socket::builder(domain, kind, 0)
            .set_non_blocking(self.non_blocking)
            .build()?;

        let (addr, size) = match self.addr {
            SockAddr::Abstract(path) => LocalSocketAddr::new_abstract_from(path),
        };

        socket.bind(addr.as_generic(), size)?;
        socket.listen(self.backlog)?;
        Ok(UnixListener(socket))
    }
}

/// A Server Unix Socket that can accept incoming connections
pub struct UnixListener(Socket);

impl UnixListener {
    /// Accepts 1 pending connection request, returns the Server's Side of the connection
    pub fn accept(&self) -> Result<UnixSockConnection, ErrorStatus> {
        let socket = self.0.accept()?;
        Ok(UnixSockConnection(socket))
    }

    /// The raw resource ID of self
    pub const fn ri(&self) -> Ri {
        self.0.ri()
    }

    pub const fn raw_socket(&self) -> &Socket {
        &self.0
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
