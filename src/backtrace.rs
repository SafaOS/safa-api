#[cfg(target_arch = "aarch64")]
mod inner {
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    pub(super) struct FramePointer(*mut StackFrame);

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct StackFrame {
        prev: FramePointer,
        return_addr: *mut u8,
    }

    impl StackFrame {
        #[inline(always)]
        /// Gets the current Frame Pointer from the fp register
        pub unsafe fn get_current<'a>() -> &'a Self {
            unsafe {
                let fp: *mut Self;
                core::arch::asm!("mov {}, fp", out(reg) fp);
                &*fp
            }
        }

        /// Gets the return address from the Frame
        pub fn return_ptr(&self) -> *mut u8 {
            self.return_addr
        }

        /// Gets the previous Frame Pointer from this one
        pub unsafe fn prev(&self) -> Option<&Self> {
            let prev = self.prev.0;

            if self.return_addr.is_null() || !prev.is_aligned() || (prev as usize) < 0x1000 {
                return None;
            }
            unsafe { Some(&*prev) }
        }
    }
}

#[cfg(target_arch = "x86_64")]
mod inner {
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct StackFrame {
        prev: *mut StackFrame,
        return_addr: *mut u8,
    }

    impl StackFrame {
        #[inline(always)]
        /// Gets the current Frame Pointer from the fp register
        pub unsafe fn get_current<'a>() -> &'a Self {
            unsafe {
                let fp: *mut Self;
                core::arch::asm!("mov {}, rbp", out(reg) fp);
                &*fp
            }
        }

        /// Gets the return address from the Frame
        pub fn return_ptr(&self) -> *mut u8 {
            self.return_addr
        }

        /// Gets the previous Frame Pointer from this one
        pub unsafe fn prev(&self) -> Option<&Self> {
            let prev = self.prev;

            if self.return_addr.is_null() || !prev.is_aligned() || (prev as usize) < 0x1000 {
                return None;
            }
            unsafe { Some(&*prev) }
        }
    }
}

use core::fmt::Display;
pub use inner::StackFrame;

#[derive(Clone, Copy)]
pub struct StackTrace<'a>(&'a StackFrame);

impl<'a> StackTrace<'a> {
    /// Gets the current Stack Trace, unsafe because the StackTrace may be corrupted
    #[inline(always)]
    pub unsafe fn current() -> Self {
        Self(unsafe { StackFrame::get_current() })
    }
}

impl<'a> Display for StackTrace<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const MAX_FRAMES: usize = 16;
        unsafe {
            let mut fp = self.0;
            writeln!(f, "\x1B[34mStack trace:")?;

            let mut omitted = false;
            for i in 0..MAX_FRAMES {
                let return_address = fp.return_ptr();

                writeln!(f, "  {:?} ", return_address)?;

                let Some(frame) = fp.prev() else {
                    break;
                };

                fp = frame;
                omitted = i == MAX_FRAMES - 1;
            }

            if omitted {
                writeln!(f, "  ...<frames omitted>")?;
            }
            write!(f, "\x1B[0m")?;
        }
        Ok(())
    }
}
