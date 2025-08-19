use core::usize;

use safa_abi::{
    consts::MAX_NAME_LENGTH,
    errors::ErrorStatus,
    sockets::{SockBindAbstractAddr, SockBindAddr, SockCreateFlags},
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
        let mut flags = SockCreateFlags::from_bits_retaining(0);

        if self.kind == SockKind::SeqPacket {
            flags = flags | SockCreateFlags::SOCK_SEQPACKET;
        }

        if self.non_blocking {
            flags = flags | SockCreateFlags::SOCK_NON_BLOCKING;
        }

        let (addr, addr_struct_size) = match self.addr {
            SockAddr::Abstract((name, len)) => (
                SockBindAbstractAddr::new(name),
                size_of::<SockBindAddr>() + len,
            ),
        };

        let addr: &SockBindAddr = unsafe { core::mem::transmute(&addr) };

        let socket_res = syscalls::sockets::create(0, flags, 0)?;
        let connection = syscalls::sockets::connect(socket_res, addr, addr_struct_size);

        let connection_ri = match connection {
            Ok(ri) => ri,
            Err(e) => {
                syscalls::resources::destroy_resource(socket_res)
                    .expect("failed to destroy a socket descriptor");
                return Err(e);
            }
        };
        syscalls::resources::destroy_resource(socket_res)
            .expect("failed to destroy a socket descriptor");
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
        let mut flags = SockCreateFlags::from_bits_retaining(0);

        if self.kind == SockKind::SeqPacket {
            flags = flags | SockCreateFlags::SOCK_SEQPACKET;
        }

        if self.non_blocking {
            flags = flags | SockCreateFlags::SOCK_NON_BLOCKING;
        }

        let socket_res = syscalls::sockets::create(0, flags, 0)?;

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
        let connection_ri = syscalls::sockets::accept(self.inner_ri, None, None)?;
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
