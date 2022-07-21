use std::{cell::{RefCell, Ref}, rc::Rc};



pub struct LinkedList<T: Default> {
    head: Link<T>,
    tail: Link<T>,
}

impl<T: Default> LinkedList<T> {
    pub fn new() -> Self {
        let dumy_head = Node::new(T::default());
        let dumy_tail = Node::new(T::default());

        dumy_head.borrow_mut().next = Some(dumy_tail.clone());
        dumy_tail.borrow_mut().prev = Some(dumy_head.clone());
        Self { head: dumy_head, tail: dumy_tail }
    }

    pub fn push_front(&mut self, val: T) {
        let new_node = Node::new(val);
        let old_head = self.head.borrow_mut().next.replace(new_node.clone());
        new_node.borrow_mut().next = old_head.clone();
        new_node.borrow_mut().prev = Some(self.head.clone());
        old_head.as_ref().unwrap().borrow_mut().prev = Some(new_node);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.head
            .borrow()
            .next
            .as_ref()
            .unwrap()
            .borrow()
            .next.is_some() {

                let node = self.head.borrow_mut().next.take().unwrap();
                let successor = node.borrow_mut().next.take();
                self.head.borrow_mut().next = successor.clone();
                successor.as_ref().unwrap().borrow_mut().prev = Some(self.head.clone());

                return Some(
                    Rc::try_unwrap(node)
                        .ok()
                        .unwrap()
                        .into_inner()
                        .value
                );
        }
        None
    }

    pub fn push_back(&mut self, val: T) {
        let new_node = Node::new(val);
        let old_tail = self.tail.borrow_mut().prev.replace(new_node.clone());
        new_node.borrow_mut().prev = old_tail.clone();
        new_node.borrow_mut().next = Some(self.tail.clone());
        old_tail.as_ref().unwrap().borrow_mut().next = Some(new_node);       
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.tail
            .borrow()
            .prev
            .as_ref()
            .unwrap()
            .borrow()
            .prev.is_some() {

                let node = self.tail.borrow_mut().prev.take().unwrap();
                let successor = node.borrow_mut().prev.take();
                self.tail.borrow_mut().prev = successor.clone();
                successor.as_ref().unwrap().borrow_mut().next = Some(self.tail.clone());

                return Some(
                    Rc::try_unwrap(node)
                        .ok()
                        .unwrap()
                        .into_inner()
                        .value
                );
        }
        None
    }

    pub fn peek_front(&mut self) -> Option<Ref<T>> {
        // TODO
        None
    }
}

impl<T: Default> Drop for LinkedList<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {} 
        self.head.borrow_mut().next = None;
        self.tail.borrow_mut().prev = None;
    }
}

type Link<T> = Rc<RefCell<Node<T>>>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Node<T> {
    value: T,
    prev: Option<Link<T>>,
    next: Option<Link<T>>,
}

impl <T>Node<T> {
    pub fn new(val: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node { 
            value: val,
            prev: None, 
            next: None, 
        }))
    }
}


#[cfg(test)]
mod test {
    use super::LinkedList;

    #[test]
    fn basics() {
        let mut list = LinkedList::new();

        assert_eq!(list.pop_front(), None);

        list.push_front(1);
        list.push_front(2);
        list.push_front(3);

        assert_eq!(list.pop_front(), Some(3));
        assert_eq!(list.pop_front(), Some(2));

        list.push_front(4);
        list.push_front(5);

        assert_eq!(list.pop_front(), Some(5));
        assert_eq!(list.pop_front(), Some(4));

        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);


        list.push_back(1);
        list.push_back(2);

        assert_eq!(list.pop_back(), Some(2));

        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);
        assert_eq!(list.pop_front(), None);

    }
}