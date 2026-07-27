//! Miri aliasing regression tests for the reference-counted green tree.
//!
//! Run under Tree Borrows (`MIRIFLAGS=-Zmiri-tree-borrows`). Default Stacked
//! Borrows rejects the `HeaderSlice<H, [T; 0]>` fake-slice `Deref` used while
//! building any green node, so a plain `cargo miri test` aborts during tree
//! construction before these run.

use rowan::{GreenNode, GreenNodeBuilder, Language, SyntaxKind, SyntaxNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TestLang;
impl Language for TestLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: SyntaxKind) -> SyntaxKind {
        raw
    }

    fn kind_to_raw(kind: SyntaxKind) -> SyntaxKind {
        kind
    }
}

type Node = SyntaxNode<TestLang>;

/// A tree with two sibling child nodes, needed to exercise the cursor sibling
/// walk in `cursor_traversal_dealloc_ub`.
fn tree() -> GreenNode {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind(0));

    builder.start_node(SyntaxKind(1));
    builder.token(SyntaxKind(11), "x");
    builder.finish_node();

    builder.start_node(SyntaxKind(2));
    builder.token(SyntaxKind(12), "y");
    builder.finish_node();

    builder.finish_node();
    builder.finish()
}

/// Dropping a cursor root runs `GreenNode::into_raw`/`from_raw` and the `Arc`
/// refcount free path.
#[test]
fn green_raw_roundtrip_is_sound() {
    drop(Node::new_root(tree()));
}

/// `GreenTokenData::to_owned` clones the green from a `&GreenTokenData` shared
/// borrow.
#[test]
fn to_owned_refcount_ub() {
    let root = Node::new_root(tree());
    let token = root.first_token().expect("tree has a token");
    let _owned = token.green().to_owned();
}

/// Cursor traversal deallocates `Box<NodeData>` in `cursor::free` through the
/// pointer reconstructed in `parent_node`.
#[test]
fn cursor_traversal_dealloc_ub() {
    let root = Node::new_root(tree());
    root.descendants_with_tokens().for_each(drop);
}

/// A green obtained by cloning a shared/borrowed green must be sound to
/// deallocate as the final owner. Here the `to_owned` clone outlives both the
/// cursor token and the root, becoming the last owner of the green allocation.
#[test]
fn to_owned_clone_is_last_owner() {
    let root = Node::new_root(tree());
    let token = root.first_token().expect("tree has a token");
    let owned = token.green().to_owned();
    drop(token);
    drop(root);
    drop(owned);
}

/// `clone_for_update` on a root must rebuild/own its green with
/// whole-allocation provenance so both roots free soundly.
#[test]
fn clone_for_update_root_free_is_sound() {
    let root = Node::new_root(tree());
    let m = root.clone_for_update();
    drop(root);
    drop(m);
}
