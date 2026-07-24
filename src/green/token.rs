use std::{borrow::Borrow, fmt, mem::ManuallyDrop, ops, ptr};

use countme::Count;

use crate::{
    TextSize,
    arc::{ArcInner, HeaderSlice, ThinArc},
    green::SyntaxKind,
};

#[derive(PartialEq, Eq, Hash)]
struct GreenTokenHead {
    kind: SyntaxKind,
    _c: Count<GreenToken>,
}

type ReprThin = HeaderSlice<GreenTokenHead, [u8; 0]>;
/// Thin, unsized-erased view of a [`GreenToken`]'s backing allocation.
///
/// Like [`super::node::GreenNodeData`], `#[repr(transparent)]` over the whole
/// [`ArcInner`], so a `&GreenTokenData` covers the interior-mutable `count`.
#[repr(transparent)]
pub struct GreenTokenData {
    inner: ArcInner<ReprThin>,
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
        let green = unsafe { GreenToken::from_raw(ptr::NonNull::from(self)) };
        let green = ManuallyDrop::new(green);
        GreenToken::clone(&green)
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
        self.inner.data.header.kind
    }

    /// Text of this Token.
    #[inline]
    pub fn text(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.inner.data.slice()) }
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
        this.ptr.into_raw_inner().cast()
    }

    /// # Safety
    ///
    /// `ptr` must point at the `ArcInner` backing a `GreenToken` (as produced by
    /// [`GreenToken::into_raw`]); ownership is transferred into the return value.
    #[inline]
    pub(crate) unsafe fn from_raw(ptr: ptr::NonNull<GreenTokenData>) -> GreenToken {
        let arc = unsafe { ThinArc::from_raw_inner(ptr.cast()) };
        GreenToken { ptr: arc }
    }
}

impl ops::Deref for GreenToken {
    type Target = GreenTokenData;

    #[inline]
    fn deref(&self) -> &GreenTokenData {
        // See `GreenNode::deref`.
        unsafe { &*(self.ptr.as_ptr() as *const GreenTokenData) }
    }
}
