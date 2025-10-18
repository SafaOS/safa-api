use core::ptr::NonNull;

use safa_abi::{
    errors::ErrorStatus,
    ffi::slice::Slice,
    sockets::{SockCreateKind, SockDomain, SockMsgFlags, SocketAddr},
};

use crate::syscalls::types::{
    IntoSyscallArg, OptionalPtr, OptionalPtrMut, RequiredPtr, RequiredPtrMut, Ri,
};

use super::SyscallNum;

impl IntoSyscallArg for SockCreateKind {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        let u16: u16 = unsafe { core::mem::transmute(self) };
        (u16 as usize,)
    }
}

impl IntoSyscallArg for SockMsgFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        let u32: u32 = unsafe { core::mem::transmute(self) };
        (u32 as usize,)
    }
}

define_syscall! {
    SyscallNum::SysSockCreate => {
        /// Creates a new generic Unix Socket Descriptor with the given flags, domain, and protocol,
        /// The generic Socket Descriptor can then be upgraded to a Server Socket using [`syssock_bind`]
        /// # Arguments
        /// - `domain` can only be 0 for now indicating Unix Local Sockets
        /// - `kind` information about the Socket Type for example if it is a SEQPACKET or a STREAM socket, and whether or not it blocks
        /// - `protocol` ignored for now
        /// # Returns
        /// Returns The resource ID of the Socket
        syssock_create(domain: SockDomain, kind: SockCreateKind, protocol: u32) Ri
    },
    SyscallNum::SysSockBind => {
        /// Binds a Server Socket to address pointed to by `addr` or upgrades a Generic Socket Descriptor to a Server Socket and then binds it to `addr`
        /// you then have to [`syssock_listen`] to listen for connections
        /// # Arguments
        /// - `sock_resource` either a Server Socket or a Socket Descriptor Resource
        /// - `addr` the address to bind to, the structure varies depending on the socket, an example is [`safa_abi::sockets::SockBindAbstractAddr`] for local sockets
        /// - `addr_struct_size` the total size of `addr` in bytes minus the unused bytes
        syssock_bind(sock_resource: Ri, addr: RequiredPtr<SocketAddr>, addr_struct_size: usize)
    },
    SyscallNum::SysSockListen => {
        /// Configures a Server Socket listening queue to be able to hold `backlog` pending connections,
        /// by default it can only hold 0, theoriticaly the limit is [`isize::MAX`], but you probably wouldn't have enough memory to hold all of that anyways,
        ///
        /// You can then accept a connection using [`syssock_accept`] :3
        syssock_listen(sock_resource: Ri, backlog: usize)
    },
    SyscallNum::SysSockAccept => {
        /// Accepts a pending connection from the listening queue that was configured using [`syssock_listen`]
        ///
        /// Doesn't block if the socket is non-blocking otherwise blocks until a connection is requested.
        /// # Arguments
        /// - `sock_resource`: a Server Socket
        /// - `accepted_addr`: Gets filled with the address we accepted from on success TODO: docs.
        /// # Returns
        ///   The resource ID of the established Connection,
        ///
        /// You can then do reads and writes using [`super::io::sysread`] and [`super::io::syswrite`], on that connection (offsets are ignored),
        /// They might block if the socket was set as blocking.
        syssock_accept(sock_resource: Ri, accepted_addr: OptionalPtrMut<(NonNull<SocketAddr>, usize)>) Ri
    },
    SyscallNum::SysSockConnect => {
        /// Given a Generic Socket Descriptor, Requests a pending connection in a Server Sockets'
        /// (that was binded at `addr` using [`syssock_bind`]) listen queue (that was configured using [`syssock_listen`]),
        ///
        /// The given Socket Descriptor must match that of the Server's,
        /// This will always block until the pending connection gets [`syssock_accept`]ed for now, even if the socket descriptor was non-blocking.
        ///
        /// # Arguments
        /// - `sock_resource`: A Generic Socket Descriptor created using [`syssock_create`].
        /// - `addr`, `addr_struct_size`: see [`syssock_bind`]
        /// - `out_connection_resource`: (return value) the Client's end of the established connection if successful
        ///
        /// see [`syssock_accept`] for more information, the Client's connection works exactly like the Server's
        syssock_connect(sock_resource: Ri, addr: RequiredPtr<SocketAddr>, addr_struct_size: usize)
    },
    SyscallNum::SysSockSendTo => {
        /// Given a socket descriptor, use it to send the data `data` to address `addr`.
        /// TODO: docs
        syssock_sendto(sock_resource: Ri, data: Slice<u8>, flags: SockMsgFlags, addr: OptionalPtr<SocketAddr>, addr_struct_size: usize) usize
    },
    SyscallNum::SysSockRecvFrom => {
        /// Given a socket descriptor, use it to receive data only if its connected, puts the address of the sender in `received_addr`.
        /// TODO: docs
        syssock_recvfrom(sock_resource: Ri, data: Slice<u8>, flags: SockMsgFlags, received_addr: OptionalPtrMut<(NonNull<SocketAddr>, usize)>) usize
    }
}

/// Creates a new generic Unix Socket Descriptor with the given flags, domain, and protocol,
/// The generic Socket Descriptor can then be upgraded to a Server Socket using [`bind`]
/// # Arguments
/// - `domain` can only be 0 for now indicating Unix Local Sockets
/// - `flags` information about the Socket Type for example if it is a SEQPACKET or a STREAM socket, and whether or not it blocks
/// - `protocol` ignored for now
/// # Returns
/// The Resource ID of the Socket Descriptor if successful
pub fn create(domain: SockDomain, kind: SockCreateKind, protocol: u32) -> Result<Ri, ErrorStatus> {
    syssock_create(domain, kind, protocol).get()
}

/// Binds a Server Socket to address pointed to by `addr` or upgrades a Generic Socket Descriptor to a Server Socket and then binds it to `addr`
/// you then have to [`listen`] for connections
/// # Arguments
/// - `sock_resource` either a Server Socket or a Socket Descriptor Resource
/// - `addr` the address to bind to, the structure varies depending on the socket, an example is [`safa_abi::sockets::SockBindAbstractAddr`] for local sockets
/// - `addr_struct_size` the total size of `addr` in bytes minus the unused bytes
pub fn bind(
    sock_resource: Ri,
    addr: &SocketAddr,
    addr_struct_size: usize,
) -> Result<(), ErrorStatus> {
    syssock_bind(
        sock_resource,
        unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) },
        addr_struct_size,
    )
    .get()
}

/// Configures a Server Socket listening queue to be able to hold `backlog` pending connections,
/// by default it can only hold 0, theoriticaly the limit is [`isize::MAX`], but you probably wouldn't have enough memory to hold all of that anyways,
///
/// You can then accept a connection using [`accept`] :3
pub fn listen(sock_resource: Ri, backlog: usize) -> Result<(), ErrorStatus> {
    syssock_listen(sock_resource, backlog).get()
}

/// Accepts a pending connection from the listening queue that was configured using [`listen`]
///
/// Doesn't block if the socket is non-blocking otherwise blocks until a connection is requested.
/// # Arguments
/// - `sock_resource`: a Socket thats listening
/// # Returns
/// The resource ID of the established Connection,
/// You can then do reads and writes using [`super::io::read`] and [`super::io::write`], on that connection (offsets are ignored),
/// They might block if the socket was set as blocking.
pub fn accept(
    sock_resource: Ri,
    accepted_addr: Option<&mut (NonNull<SocketAddr>, usize)>,
) -> Result<Ri, ErrorStatus> {
    let accepted_addr = OptionalPtrMut::from_option(
        accepted_addr.map(|ptr| unsafe { RequiredPtrMut::new_unchecked(ptr) }),
    );
    syssock_accept(sock_resource, accepted_addr).get()
}

/// Given a Generic Socket Descriptor, Requests a pending connection in a Server Sockets'
/// (that was binded at `addr` using [`bind`]) listen queue (that was configured using [`listen`]),
///
/// The given Socket Descriptor must match that of the Server's,
/// This will always block until the pending connection gets [`accept`]ed for now, even if the socket descriptor was non-blocking.
///
/// # Arguments
/// - `sock_resource`: A Generic Socket Descriptor created using [`create`].
/// - `addr`, `addr_struct_size`: see [`bind`]
/// # Returns
/// The Client's end of the established connection if successful.
///
/// see [`accept`] for more information, the Client's connection behaves exactly like the Server's
pub fn connect(
    sock_resource: Ri,
    addr: &SocketAddr,
    addr_struct_size: usize,
) -> Result<(), ErrorStatus> {
    syssock_connect(
        sock_resource,
        unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) },
        addr_struct_size,
    )
    .get()
}

pub fn send_to(
    sock_resource: Ri,
    data: &[u8],
    flags: SockMsgFlags,
    target_addr: Option<(&SocketAddr, usize)>,
) -> Result<usize, ErrorStatus> {
    let (target_addr, target_addr_size) =
        target_addr.map_or((None, 0), |(addr, size)| (Some(addr), size));
    let target_addr =
        target_addr.map(|addr| unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) });

    syssock_sendto(
        sock_resource,
        Slice::from_slice(data),
        flags,
        OptionalPtr::from_option(target_addr),
        target_addr_size,
    )
    .get()
}

pub fn recv_from(
    sock_resource: Ri,
    buffer: &mut [u8],
    flags: SockMsgFlags,
    source_addr: Option<&mut (NonNull<SocketAddr>, usize)>,
) -> Result<usize, ErrorStatus> {
    let source_addr = source_addr.map(|ptr| unsafe { RequiredPtr::new_unchecked(ptr) });
    let source_addr = OptionalPtr::from_option(source_addr);

    syssock_recvfrom(
        sock_resource,
        Slice::from_slice_mut(buffer),
        flags,
        source_addr,
    )
    .get()
}
