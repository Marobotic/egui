//! A window's first accessibility tree, given at once when an assistive technology first asks
//! for it.
//!
//! An AccessKit adapter made with an event loop proxy can't answer the first request itself:
//! it hands the platform a placeholder (on Windows, the window with no content: UI Automation
//! sees the title bar only) and the real tree arrives with the next frame. A client that reads
//! the window once, or before that frame, gets the placeholder. So each window keeps the last
//! tree its frames produced (egui writes the whole tree each frame while AccessKit is on), and
//! the activation handler answers with it. A new frame is asked for as before, so the tree is
//! brought up to date right after.

use std::sync::Arc;

use egui::accesskit::{ActivationHandler, TreeUpdate};
use egui::mutex::Mutex;

/// The last accessibility tree a window's frames produced, shared with its activation handler
/// (which platform adapters may call on another thread).
#[derive(Clone, Default)]
pub(crate) struct LastTree(Arc<Mutex<Option<TreeUpdate>>>);

impl LastTree {
    /// Keep `update` (a frame's whole tree) for the next first request.
    pub(crate) fn keep(&self, update: TreeUpdate) {
        *self.0.lock() = Some(update);
    }

    fn take(&self) -> Option<TreeUpdate> {
        self.0.lock().take()
    }
}

/// Answers a platform's first request with the window's last tree (none before its first frame
/// with AccessKit on: the platform then uses its placeholder until the next frame), and asks
/// for a new frame either way (`request_frame`: AccessKit turned on, the window repainted).
pub(crate) struct InitialTree {
    pub(crate) last: LastTree,
    pub(crate) request_frame: Box<dyn FnMut() + Send>,
}

impl ActivationHandler for InitialTree {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        (self.request_frame)();
        self.last.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};
    use egui::accesskit::{Node, NodeId, Role, TreeId, TreeInfo};

    fn tree(label: &str) -> TreeUpdate {
        let mut root = Node::new(Role::Window);
        root.set_children(vec![NodeId(2)]);
        let mut button = Node::new(Role::Button);
        button.set_label(label);
        TreeUpdate {
            nodes: vec![(NodeId(1), root), (NodeId(2), button)],
            tree: Some(TreeInfo::new(NodeId(1))),
            tree_id: TreeId::ROOT,
            focus: NodeId(1),
        }
    }

    fn handler(last: &LastTree, frames: &Arc<AtomicUsize>) -> InitialTree {
        let frames = Arc::clone(frames);
        InitialTree {
            last: last.clone(),
            request_frame: Box::new(move || {
                frames.fetch_add(1, Ordering::SeqCst);
            }),
        }
    }

    /// An idle window, first read after its last frame: the reader gets that frame's whole
    /// tree at once, not a placeholder.
    #[test]
    fn an_idle_window_answers_its_first_request_with_its_last_tree() {
        let last = LastTree::default();
        let frames = Arc::new(AtomicUsize::new(0));
        last.keep(tree("old"));
        last.keep(tree("Start"));
        let first = handler(&last, &frames)
            .request_initial_tree()
            .expect("a tree");
        assert_eq!(first.nodes.len(), 2);
        assert_eq!(
            first.nodes[1].1.label(),
            Some("Start"),
            "the latest frame's"
        );
        assert_eq!(
            frames.load(Ordering::SeqCst),
            1,
            "a fresh frame is asked for too"
        );
    }

    /// Before its first frame with AccessKit on there is no tree: the platform's placeholder,
    /// and the frame that brings the real one.
    #[test]
    fn a_window_without_a_frame_yet_asks_for_one() {
        let last = LastTree::default();
        let frames = Arc::new(AtomicUsize::new(0));
        assert!(handler(&last, &frames).request_initial_tree().is_none());
        assert_eq!(frames.load(Ordering::SeqCst), 1);
    }
}
