

pub struct  Stack {
    head: Node,
}

struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

impl Node {
    pub fn new(val: i32, next: Option<Box<Node>>) -> Self {
        Self { value: val, next: next }
    }
}

impl Stack {
    pub fn new() -> Self {
        Self { head: Node::new(0, None) }
    }

    pub fn push(&mut self, val: i32) {
        let node = Node::new(val, self.head.next.take());
        self.head.next = Some(Box::new(node));
    }

    pub fn pop(&mut self) -> Option<i32> {
        if let Some(mut node) = self.head.next.take() {
            self.head.next = node.next.take();
            return Some(node.value);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::Stack;

    #[test]
    fn test() {
        let mut stack = Stack::new();
        stack.push(5);
        stack.push(4);
        stack.push(3);
        stack.push(2);

        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(4));
        assert_eq!(stack.pop(), Some(5));
        assert_eq!(stack.pop(), None);
    }

}