use safa_abi::{
    errors::ErrorStatus,
    ffi::{option::OptZero, ptr::FFINonNull},
    sockets::{SockBindAddr, SockCreateFlags},
};

use crate::syscalls::types::{IntoSyscallArg, OptionalPtrMut, RequiredPtr, Ri};

use super::SyscallNum;

impl IntoSyscallArg for SockCreateFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        let u16: u16 = unsafe { core::mem::transmute(self) };
        (u16 as usize,)
    }
}
define_syscall! {
    SyscallNum::SysSockCreate => {
        /// Creates a new generic Unix Socket Descriptor with the given flags, domain, and protocol,
        /// The generic Socket Descriptor can then be upgraded to a Server Socket using [`syssock_bind`]
        /// # Arguments
        /// - `domain` can only be 0 for now indicating Unix Local Sockets
        /// - `flags` information about the Socket Type for example if it is a SEQPACKET or a STREAM socket, and whether or not it blocks
        /// - `protocol` ignored for now
        /// - `out_resource` The returned Resource ID of the Socket Descriptor
        syssock_create(domain: u8, flags: SockCreateFlags, protocol: u32, out_resource: OptionalPtrMut<Ri>)
    },
    SyscallNum::SysSockBind => {
        /// Binds a Server Socket to address pointed to by `addr` or upgrades a Generic Socket Descriptor to a Server Socket and then binds it to `addr`
        /// you then have to [`syssock_listen`] to listen for connections
        /// # Arguments
        /// - `sock_resource` either a Server Socket or a Socket Descriptor Resource
        /// - `addr` the address to bind to, the structure varies depending on the socket, an example is [`safa_abi::sockets::SockBindAbstractAddr`] for local sockets
        /// - `addr_struct_size` the total size of `addr` in bytes minus the unused bytes
        syssock_bind(sock_resource: Ri, addr: RequiredPtr<SockBindAddr>, addr_struct_size: usize)
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
        /// - `addr` and `addr_struct_size`:
        ///   The address to accept from or null to accept from anything,
        ///   Then the address would get overwritten by smth I don't know,
        ///   This is currently not implemented so set it as null always.
        /// - `out_connection_resource`: (return value) The resource ID of the established Connection,
        ///   You can then do reads and writes using [`super::io::sysread`] and [`super::io::syswrite`], on that connection (offsets are ignored),
        ///   They might block if the socket was set as blocking.
        syssock_accept(sock_resource: Ri, addr: OptionalPtrMut<SockBindAddr>, addr_struct_size: OptionalPtrMut<usize>, out_connection_resource: OptionalPtrMut<Ri>)
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
        syssock_connect(sock_resource: Ri, addr: RequiredPtr<SockBindAddr>, addr_struct_size: usize, out_connection_resource: OptionalPtrMut<Ri>)
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
pub fn create(domain: u8, flags: SockCreateFlags, protocol: u32) -> Result<Ri, ErrorStatus> {
    let mut ri = 0xAAAAAAAAAAAAAAAA;
    err_from_u16!(
        syssock_create(domain, flags, protocol, RequiredPtr::new(&mut ri).into()),
        ri
    )
}

/// Binds a Server Socket to address pointed to by `addr` or upgrades a Generic Socket Descriptor to a Server Socket and then binds it to `addr`
/// you then have to [`listen`] for connections
/// # Arguments
/// - `sock_resource` either a Server Socket or a Socket Descriptor Resource
/// - `addr` the address to bind to, the structure varies depending on the socket, an example is [`safa_abi::sockets::SockBindAbstractAddr`] for local sockets
/// - `addr_struct_size` the total size of `addr` in bytes minus the unused bytes
pub fn bind(
    sock_resource: Ri,
    addr: &SockBindAddr,
    addr_struct_size: usize,
) -> Result<(), ErrorStatus> {
    err_from_u16!(syssock_bind(
        sock_resource,
        unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) },
        addr_struct_size
    ))
}

/// Configures a Server Socket listening queue to be able to hold `backlog` pending connections,
/// by default it can only hold 0, theoriticaly the limit is [`isize::MAX`], but you probably wouldn't have enough memory to hold all of that anyways,
///
/// You can then accept a connection using [`accept`] :3
pub fn listen(sock_resource: Ri, backlog: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(syssock_listen(sock_resource, backlog))
}

/// Accepts a pending connection from the listening queue that was configured using [`listen`]
///
/// Doesn't block if the socket is non-blocking otherwise blocks until a connection is requested.
/// # Arguments
/// - `sock_resource`: a Server Socket
/// - `addr` and `addr_struct_size`:
///   The address to accept from or null to accept from anything,
///   Then the address would get overwritten by smth I don't know,
///   This is currently not implemented so set it as null always.
/// # Returns
/// The resource ID of the established Connection,
/// You can then do reads and writes using [`super::io::read`] and [`super::io::write`], on that connection (offsets are ignored),
/// They might block if the socket was set as blocking.
pub fn accept(
    sock_resource: Ri,
    addr: Option<&mut SockBindAddr>,
    addr_struct_size: Option<&mut usize>,
) -> Result<Ri, ErrorStatus> {
    let mut ri = 0xAAAAAAAAAAAAAAAA;
    err_from_u16!(
        syssock_accept(
            sock_resource,
            match addr {
                Some(addr) => unsafe { OptZero::some(FFINonNull::new_unchecked(addr)) },
                None => OptZero::none(),
            },
            match addr_struct_size {
                Some(addr) => unsafe { OptZero::some(FFINonNull::new_unchecked(addr)) },
                None => OptZero::none(),
            },
            RequiredPtr::new(&mut ri).into()
        ),
        ri
    )
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
    addr: &SockBindAddr,
    addr_struct_size: usize,
) -> Result<Ri, ErrorStatus> {
    let mut ri = 0xAAAAAAAAAAAAAAAA;
    err_from_u16!(
        syssock_connect(
            sock_resource,
            unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) },
            addr_struct_size,
            RequiredPtr::new(&mut ri).into()
        ),
        ri
    )
}
