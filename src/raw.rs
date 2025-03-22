use core::ptr::NonNull;

#[repr(C)]
#[derive(Debug)]
pub struct RawSlice<T> {
    ptr: *const T,
    len: usize,
}

impl<T> RawSlice<T> {
    #[inline(always)]
    pub unsafe fn from_raw_parts(ptr: *const T, len: usize) -> Self {
        Self { ptr, len }
    }
    #[inline(always)]
    pub unsafe fn from_slice(slice: &[T]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }
}

impl<T> RawSliceMut<T> {
    #[inline(always)]
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Self {
        Self { ptr, len }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T> RawSliceMut<RawSlice<T>> {
    /// Converts a slice of slices of [`T`] into [`RawSliceMut<RawSlice<T>>`]
    /// # Safety
    /// `slices` becomes invaild after use
    /// as it is going to be reused as a memory location for creating `Self`
    /// making this unexpensive but dangerous
    /// O(N) expect if the Layout of RawSlice is equal to the Layout of rust slices, and it has been optimized it is O(1)
    #[inline(always)]
    pub unsafe fn from_slices(slices: *mut [&[T]]) -> Self {
        let old_slices = unsafe { &mut *slices };
        let raw_slices = unsafe { &mut *(slices as *mut [RawSlice<T>]) };

        for (i, slice) in old_slices.iter().enumerate() {
            raw_slices[i] = unsafe { RawSlice::from_slice(slice) };
        }
        unsafe { RawSliceMut::from_raw_parts(raw_slices.as_mut_ptr(), raw_slices.len()) }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct RawSliceMut<T> {
    ptr: *mut T,
    len: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct NonNullSlice<T> {
    ptr: NonNull<T>,
    len: usize,
}

impl<T> NonNullSlice<T> {
    pub const unsafe fn from_raw_parts(ptr: NonNull<T>, len: usize) -> Self {
        Self { ptr, len }
    }
}

/// A C complitable Option-like type
#[derive(Debug)]
#[repr(C)]
pub enum Optional<T> {
    None,
    Some(T),
}

impl<T> From<Option<T>> for Optional<T> {
    #[inline(always)]
    fn from(value: Option<T>) -> Self {
        match value {
            None => Self::None,
            Some(x) => Self::Some(x),
        }
    }
}

// TODO: consider moving this to the abi?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InodeType {
    File,
    Directory,
    Device,
}

// TODO: consider moving this to the abi?
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct DirEntry {
    pub kind: InodeType,
    pub size: usize,
    pub name_length: usize,
    pub name: [u8; Self::MAX_NAME_LEN],
}

impl DirEntry {
    const MAX_NAME_LEN: usize = 128;
}
