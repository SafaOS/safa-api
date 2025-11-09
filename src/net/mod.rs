#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
extern crate alloc;

use core::net::IpAddr;
use core::net::Ipv4Addr;
use core::net::SocketAddrV4;
#[cfg(feature = "std")]
use std as alloc;

use alloc::boxed::Box;
use alloc::string::String;

use safa_abi::errors::ErrorStatus;
use safa_abi::sockets::InetV4SocketAddr;
use safa_abi::sockets::SockCreateKind as AbiSocketKind;
use safa_abi::sockets::SockDomain as AbiSocketDomain;
use safa_abi::sockets::SocketAddr;

mod dns;
use crate::net::dns::DnsResolutionError;
use crate::sockets::{SocketDomain, SocketKind};

const fn fam_to_raw(fam: Option<SocketDomain>) -> AbiSocketDomain {
    match fam {
        Some(f) => f.into_raw(),
        None => unsafe { core::mem::transmute(u8::MAX) },
    }
}

const fn kind_to_raw(kind: Option<SocketKind>) -> AbiSocketKind {
    match kind {
        Some(k) => k.into_raw(),
        None => AbiSocketKind::from_bits_retaining(u16::MAX),
    }
}

/// Address hints given to [`lookup_addr_info`]
///
/// TODO: Docs
#[repr(C)]
pub struct AddrHints {
    family: AbiSocketDomain,
    __0: u8,
    kind: AbiSocketKind,
    protocol: u32,
    __1: u64,
}

impl AddrHints {
    pub const fn new(
        kind: Option<SocketKind>,
        family: Option<SocketDomain>,
        protocol: u32,
    ) -> Self {
        let kind = kind_to_raw(kind);
        let family = fam_to_raw(family);

        Self {
            family,
            __0: 0,
            kind,
            protocol,
            __1: 0,
        }
    }

    /// Returns the kind of the socket that this hint accepts
    pub const fn kind(&self) -> Option<SocketKind> {
        match SocketKind::from_raw(self.kind) {
            None => None,
            Some((x, _)) => Some(x),
        }
    }

    /// Returns the domain(family) of the socket that this hint accepts.
    pub const fn domain(&self) -> Option<SocketDomain> {
        SocketDomain::from_raw(self.family)
    }
    #[inline]
    /// Returns the protocol this hint accepts.
    pub const fn protocol(&self) -> u32 {
        self.protocol
    }
}

/// AddrInfo returned by [`lookup_addr_info`]
///
/// TODO: Docs
#[repr(C)]
pub struct AddrInfo {
    family: AbiSocketDomain,
    __0: u8,
    kind: AbiSocketKind,
    protocol: u32,
    __1: u64,
    next: Option<Box<Self>>,
    socket_addr_raw: Box<[u8]>,
    canon_name: Option<Box<str>>,
}

impl AddrInfo {
    fn new(
        fam: Option<SocketDomain>,
        kind: Option<SocketKind>,
        protocol: u32,
        addr: core::net::SocketAddrV4,
        canon_name: Option<String>,
    ) -> Self {
        let family = fam_to_raw(fam);
        let kind = kind_to_raw(kind);
        let addr_raw = InetV4SocketAddr::new(addr.port(), *addr.ip());
        let addr_bytes = addr_raw.as_bytes();
        Self {
            family,
            __0: 0,
            kind,
            protocol,
            __1: 0,
            next: None,
            socket_addr_raw: addr_bytes.to_vec().into_boxed_slice(),
            canon_name: canon_name.map(|s| s.into_boxed_str()),
        }
    }

    fn set_next(&mut self, n: Option<Box<Self>>) {
        self.next = n;
    }

    fn set_canon(&mut self, canon_name: Option<String>) {
        self.canon_name = canon_name.map(|s| s.into_boxed_str());
    }

    /// Returns the next [`AddrInfo`] in this linked list
    pub fn next(&self) -> Option<&AddrInfo> {
        self.next.as_ref().map(|n| n.as_ref())
    }

    /// Same as [`Self::next`] but muttable.
    pub fn next_mut(&mut self) -> Option<&mut AddrInfo> {
        self.next.as_mut().map(|n| n.as_mut())
    }

    /// Similar to [`Self::next_mut`] but returns a reference to the container
    pub const fn next_mut_ref(&mut self) -> &mut Option<Box<AddrInfo>> {
        &mut self.next
    }

    /// Takes the next [`AddrInfo`] structure in that linked list, leaving None instead.
    pub fn take_next(&mut self) -> Option<AddrInfo> {
        self.next.take().map(|n| *n)
    }

    /// Returns the domain(family) of the socket that uses this address.
    pub const fn domain(&self) -> Option<SocketDomain> {
        SocketDomain::from_raw(self.family)
    }

    /// Returns the kind of the socket that uses this address
    pub const fn kind(&self) -> Option<SocketKind> {
        match SocketKind::from_raw(self.kind) {
            None => None,
            Some((x, _)) => Some(x),
        }
    }

    /// Returns the protocol of the socket that uses this address
    pub const fn protocol(&self) -> u32 {
        self.protocol
    }

    /// Returns true if the socket created to point to this address is blocking.
    #[inline]
    pub const fn socket_blocks(&self) -> bool {
        !self.kind.contains(AbiSocketKind::SOCK_NON_BLOCKING)
    }

    #[inline]
    pub const fn socket_addr(&self) -> &SocketAddr {
        unsafe { &*self.socket_addr_raw.as_ptr().cast::<SocketAddr>() }
    }
    #[inline]
    pub const fn socket_addr_size(&self) -> usize {
        self.socket_addr_raw.len()
    }

    #[inline]
    /// Returns the socket addr inside of self as an [`core::net::SocketAddr`] which is possible because we only accept IpV4 and IpV6 family.
    pub fn ip_socket_addr(&self) -> core::net::SocketAddr {
        let addr = self.socket_addr();

        addr.as_known::<InetV4SocketAddr>()
            .map(|k| core::net::SocketAddr::new(IpAddr::V4(k.ip()), k.port()))
            .expect("AddrInfo family isn't IpV4 or IpV6")
    }
}

/// An error during node and service lookup operation
///
/// see [`lookup_addr_info`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    /// Couldn't resolve service to a port.
    NoSuchService,
    /// Couldn't resolve node, either because it is in an invalid format or for example DNS Resolution found no domain under that name.
    ///
    /// Is also returned when both node and services are None.
    NoSuchNode,
    /// Requested Family isn't supported.
    InvalidFamily,
    /// Failure was temporary, Trying Again may return results.
    ///
    /// Eg. Nameserver didn't respond with anything.
    TemporaryFailure,
    /// Nameserver Refused to respond, might be a nameserver error.
    ServerRefused,
    /// The given node resolution was successful but no addresses were found.
    ///
    /// Eg. A DNS Query resolving the node as a domain name didn't return any address nodes.
    NoData,
    /// A System Error has occurred.
    System(ErrorStatus),
}

impl From<DnsResolutionError> for LookupError {
    fn from(value: DnsResolutionError) -> Self {
        match value {
            DnsResolutionError::InvalidDomainName => Self::NoSuchNode,
            DnsResolutionError::NoResponse => Self::TemporaryFailure,
            DnsResolutionError::NoSuchName => Self::NoSuchNode,
            DnsResolutionError::Refused => Self::ServerRefused,
            DnsResolutionError::System(sys) => Self::System(sys),
        }
    }
}

/// Given a `node` and a `service`, resolve the service to a port number and information about the service, and then lookup the node's addr info.
///
/// `node` can be a string indicating a domain name in this case a DNS Resolution would be performed or None for only service lookup or an Ip Address respecting the family.
/// `service` can be a port number or a string specifying the service (it will be converted to a port number) not really implemented currently.
///
/// `hint` is information and hints about what addresses we should accept see [`AddrHints`], it is currently necessary to figure out the returned protocol and kind.
///
/// Returns a linked list of [`AddrInfo`] or a [`LookupError`].
pub fn lookup_addr_info(
    node: Option<&str>,
    service: Option<&str>,
    hint: Option<&AddrHints>,
) -> Result<AddrInfo, LookupError> {
    if node.is_none() && service.is_none() {
        return Err(LookupError::NoSuchNode);
    }

    // TODO: Implement services lookup
    let service = service
        .map(|s| s.parse::<u16>())
        .unwrap_or(Ok(0))
        .map_err(|_| LookupError::NoSuchService)?;

    let protocol = hint.map(|h| h.protocol()).unwrap_or(0);
    let family = hint
        .map(|h| h.domain())
        .flatten()
        .unwrap_or(SocketDomain::Ipv4);

    let kind = hint.map(|h| h.kind()).flatten();

    match family {
        SocketDomain::Ipv4 => {}
        // TODO: Ipv6
        _ => return Err(LookupError::InvalidFamily),
    }

    match node {
        None => {
            // STUB
            // TODO: service lookup

            Ok(AddrInfo::new(
                Some(family),
                kind,
                protocol,
                core::net::SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, service),
                None,
            ))
        }

        Some(domain) => {
            if let Ok(ip) = domain.parse::<Ipv4Addr>() {
                // STUB
                // TODO: service lookup
                return Ok(AddrInfo::new(
                    Some(family),
                    kind,
                    protocol,
                    core::net::SocketAddrV4::new(ip, service),
                    None,
                ));
            }

            let mut root = None;
            let mut tail = None;
            let canon = dns::lookup_dns(domain, |ip| {
                let mut inner = AddrInfo::new(
                    Some(family),
                    kind,
                    protocol,
                    SocketAddrV4::new(ip, service),
                    None,
                );

                if root.is_none() {
                    root = Some(inner);
                } else {
                    match core::mem::take(&mut tail) {
                        None => {}
                        Some(o) => {
                            inner.set_next(Some(Box::new(o)));
                        }
                    }

                    tail = Some(inner);
                }
            })?;

            match root {
                Some(mut r) => {
                    r.set_next(tail.map(|t| Box::new(t)));
                    if let Some(canon) = canon {
                        if r.next.is_none() {
                            r.set_canon(Some(canon));
                        } else {
                            r.set_canon(Some(canon.clone()));

                            while let Some(n) = r.next.as_mut() {
                                n.set_canon(Some(canon.clone()));
                            }
                        }
                    }

                    Ok(r)
                }
                None => Err(LookupError::NoData),
            }
        }
    }
}
