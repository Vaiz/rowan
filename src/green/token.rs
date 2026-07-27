use std::{borrow::Borrow, ffi::c_void, fmt, ops, ptr, slice, sync::atomic::AtomicUsize};

use countme::Count;
use triomphe::{HeaderSlice, HeaderWithLength, ThinArc};

use crate::{TextSize, green::SyntaxKind};

#[derive(PartialEq, Eq, Hash)]
struct GreenTokenHead {
    kind: SyntaxKind,
    _c: Count<GreenToken>,
}

type ReprThin = HeaderSlice<HeaderWithLength<GreenTokenHead>, [u8; 0]>;
/// Borrowed view of a [`GreenToken`]'s backing allocation.
///
/// Laid out to mirror triomphe's private `ArcInner<ReprThin>` — a `#[repr(C)]`
/// struct whose leading field is the atomic strong count — so that a
/// `&GreenTokenData` addresses the *whole* allocation and every read is
/// in-provenance. We never touch `strong` ourselves: all refcount changes and
/// deallocation go through triomphe's [`ThinArc`].
#[repr(C)]
pub struct GreenTokenData {
    strong: AtomicUsize,
    data: ReprThin,
}

impl PartialEq for GreenTokenData {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind() && self.text() == other.text()
    }
}

/// Leaf node in the immutable tree.
#[derive(PartialEq, Eq, Hash, Clone)]
#[repr(transparent)]
pub struct GreenToken {
    ptr: ThinArc<GreenTokenHead, u8>,
}

impl ToOwned for GreenTokenData {
    type Owned = GreenToken;

    #[inline]
    fn to_owned(&self) -> GreenToken {
        // A deep, independent copy. `self` is a *shared* borrow, so any owned
        // `Arc` reconstructed from it would carry provenance narrowed to a
        // read-only view of the allocation; bumping the refcount and later
        // deallocating the payload as the last owner is Undefined Behavior under
        // Tree Borrows. Rebuilding a fresh allocation avoids that entirely.
        GreenToken::new(self.kind(), self.text())
    }
}

impl Borrow<GreenTokenData> for GreenToken {
    #[inline]
    fn borrow(&self) -> &GreenTokenData {
        self
    }
}

impl fmt::Debug for GreenTokenData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GreenToken")
            .field("kind", &self.kind())
            .field("text", &self.text())
            .finish()
    }
}

impl fmt::Debug for GreenToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data: &GreenTokenData = self;
        fmt::Debug::fmt(data, f)
    }
}

impl fmt::Display for GreenToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data: &GreenTokenData = self;
        fmt::Display::fmt(data, f)
    }
}

impl fmt::Display for GreenTokenData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text())
    }
}

impl GreenTokenData {
    /// Kind of this Token.
    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.data.header.header.kind
    }

    /// Text of this Token.
    #[inline]
    pub fn text(&self) -> &str {
        let len = self.data.header.length;
        // `addr_of!` keeps the whole-allocation provenance of `&self`; the text
        // bytes live right after the `[u8; 0]` marker in the allocation.
        let ptr = ptr::addr_of!(self.data.slice) as *const u8;
        unsafe { std::str::from_utf8_unchecked(slice::from_raw_parts(ptr, len)) }
    }

    /// Returns the length of the text covered by this token.
    #[inline]
    pub fn text_len(&self) -> TextSize {
        TextSize::of(self.text())
    }
}

impl GreenToken {
    /// Creates new Token.
    #[inline]
    pub fn new(kind: SyntaxKind, text: &str) -> GreenToken {
        let head = GreenTokenHead { kind, _c: Count::new() };
        let ptr = ThinArc::from_header_and_iter(head, text.bytes());
        GreenToken { ptr }
    }

    #[inline]
    pub(crate) fn into_raw(this: GreenToken) -> ptr::NonNull<GreenTokenData> {
        // `ThinArc::into_raw` hands back the whole-allocation base pointer
        // without changing the refcount.
        let raw = ThinArc::into_raw(this.ptr);
        unsafe { ptr::NonNull::new_unchecked(raw as *mut GreenTokenData) }
    }

    #[inline]
    pub(crate) unsafe fn from_raw(ptr: ptr::NonNull<GreenTokenData>) -> GreenToken {
        // `ptr` is a whole-allocation base pointer produced by `into_raw`/`as_ptr`.
        let ptr = unsafe { ThinArc::from_raw(ptr.as_ptr() as *const c_void) };
        GreenToken { ptr }
    }
}

impl ops::Deref for GreenToken {
    type Target = GreenTokenData;

    #[inline]
    fn deref(&self) -> &GreenTokenData {
        // `ThinArc::as_ptr` yields the whole-allocation base pointer; casting it
        // to `&GreenTokenData` (which mirrors the `ArcInner` layout) is sound.
        unsafe { &*(self.ptr.as_ptr() as *const GreenTokenData) }
    }
}
