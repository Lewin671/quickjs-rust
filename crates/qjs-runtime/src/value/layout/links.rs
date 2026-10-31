//! Cold layout graph links. An unobserved bound shape needs no graph allocation.
//! Parents/root links own metadata only; transitions and compact edges are weak.
use super::*;

#[derive(Default)]
pub(super) struct LayoutLinks {
    pub(super) parent: OnceCell<Rc<Layout>>,
    pub(super) incoming: std::cell::Cell<Option<Transition>>,
    pub(super) root: OnceCell<Rc<Layout>>,
    pub(super) transitions: RefCell<transitions::Transitions>,
    pub(super) compact_prototype: RefCell<Weak<crate::Prototype>>,
}
impl Layout {
    pub(super) fn links(&self) -> &LayoutLinks {
        self.links.get_or_init(|| {
            self.budget.add(size_of::<LayoutLinks>());
            Box::new(LayoutLinks::default())
        })
    }
    pub(super) fn parent(&self) -> Option<&Rc<Layout>> {
        self.links.get()?.parent.get()
    }
    pub(super) fn incoming(&self) -> Option<Transition> {
        self.links.get().and_then(|links| links.incoming.get())
    }
    pub(super) fn known_root(&self) -> Option<&Rc<Layout>> {
        self.links.get()?.root.get()
    }
    pub(super) fn set_parent(&self, parent: Rc<Layout>, incoming: Transition) {
        assert!(self.links().parent.set(parent).is_ok());
        self.links().incoming.set(Some(incoming));
    }
    pub(super) fn set_root(&mut self, root: Rc<Layout>) {
        self.links();
        let links = self.links.get_mut().expect("layout links");
        links.root.take();
        assert!(links.root.set(root).is_ok());
    }
    pub(super) fn transition(&self, label: Transition) -> Option<Rc<Layout>> {
        self.links.get()?.transitions.borrow().get(label)
    }
    pub(super) fn has_transition(&self, label: Transition) -> bool {
        self.links
            .get()
            .is_some_and(|links| links.transitions.borrow().contains_live(label))
    }
    pub(super) fn transitions_empty(&self) -> bool {
        self.links
            .get()
            .is_none_or(|links| links.transitions.borrow().is_empty())
    }
    pub(super) fn next_order(&self) -> u32 {
        if self.dictionary {
            self.data.borrow().next_order
        } else {
            self.len as u32
        }
    }
}
