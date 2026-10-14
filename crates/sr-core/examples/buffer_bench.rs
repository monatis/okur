//! Times building a virtual buffer for a large, deep page (scratch benchmark).
use sr_core::{AomNode, Role, VirtualBuffer};

fn page(depth: usize, breadth: usize, next: &mut usize) -> AomNode {
    *next += 1;
    let mut node = AomNode::new(
        *next,
        if depth == 0 {
            Role::Paragraph
        } else {
            Role::Group
        },
    );
    if depth == 0 {
        return node.with_text("A message in a long chat log, with some words in it.");
    }
    let children = (0..breadth)
        .map(|_| page(depth - 1, breadth, next))
        .collect();
    node = node.with_children(children);
    node
}

fn main() {
    for (depth, breadth) in [(6, 4), (12, 2), (20, 1), (8, 3)] {
        let mut next = 0;
        // A chain of wrappers above the content, as web apps nest divs.
        let mut root = page(depth, breadth, &mut next);
        for _ in 0..25 {
            next += 1;
            root = AomNode::new(next, Role::Group).with_children(vec![root]);
        }
        let doc = AomNode::new(0, Role::Document).with_children(vec![root]);
        let started = std::time::Instant::now();
        let buffer = VirtualBuffer::from_aom(&doc);
        println!(
            "nodes={next} depth={} lines={} built in {:?}",
            depth + 25,
            buffer.lines.len(),
            started.elapsed()
        );
    }
}
