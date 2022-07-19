


pub struct List<T> {
    head: Link<T>,
}
/// implement List
impl <T>List<T> {
    pub fn new() -> Self {
        Self { head: None }
    }
    pub fn push(&mut self, val: T) {
        let node = Node::new(val, self.head.take());
        self.head = Some(Box::new(node));
    }

    pub fn pop(&mut self) -> Option<T> {
        self.head.take().map(|mut node| {
            self.head = node.next.take();
            node.value
        })
    }

    pub fn peek(&self) -> Option<&T> {
        self.head.as_ref().map(|node| {
            &node.value
        })
    }
}

/// implement IntoIterator for List
pub struct IntoIter<T>(List<T>);

impl<T> IntoIterator for List<T>{
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

/// implement Iter for List
pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

impl<T> List<T> {
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            next: self.head.as_deref(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = node.next.as_deref();
            &node.value
        })
    }
}
/// implement IterMut
pub struct IterMut<'a, T> {
    next: Option<&'a mut Node<T>>
}


impl<T> List<T> {
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { next: self.head.as_deref_mut() }
    }
}


impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = node.next.as_deref_mut();
            &mut node.value
        })
    }
}



type Link<T> = Option<Box<Node<T>>>;

struct Node<T> {
    value: T,
    next: Link<T>,
}

impl <T>Node<T> {
    pub fn new(val: T, next: Link<T>) -> Self {
        Self { value: val, next: next }
    }
}

#[cfg(test)]
mod test {
    use super::List;

    #[test]
    fn test() {
        let mut list = List::new();
        list.push(5.5);
        list.push(4.0);
        list.push(3.);
        list.push(2.);

        assert_eq!(list.peek(), Some(&2.));

        assert_eq!(list.pop(), Some(2.));
        assert_eq!(list.pop(), Some(3.));
        assert_eq!(list.pop(), Some(4.));
        assert_ne!(list.pop(), Some(5.3));
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn test_into_iter() {
        let mut list = List::new();
        list.push(5.5);
        list.push(4.0);
        list.push(3.);
        list.push(2.);

        let mut iter = list.into_iter();

        assert_eq!(iter.next(), Some(2.));
        assert_eq!(iter.next(), Some(3.));
        assert_eq!(iter.next(), Some(4.));
        assert_eq!(iter.next(), Some(5.5));
        // assert_eq!(list.pop(), None);
        // assert_eq!(list.pop(), Some(2.));
        // assert_eq!(list.pop(), Some(3.));
        // assert_eq!(list.pop(), Some(4.));
        // assert_ne!(list.pop(), Some(5.3));
        // assert_eq!(list.pop(), None);
    }

    #[test]
    fn test_iter() {
        let mut list = List::new();
        list.push(5.5);
        list.push(4.0);
        list.push(3.);
        list.push(2.);

        let mut iter = list.iter();

        assert_eq!(iter.next(), Some(&2.));
        assert_eq!(iter.next(), Some(&3.));
        assert_eq!(iter.next(), Some(&4.));
        assert_eq!(iter.next(), Some(&5.5));
        // assert_eq!(list.pop(), None);
        assert_eq!(list.pop(), Some(2.));
        assert_eq!(list.pop(), Some(3.));
        assert_eq!(list.pop(), Some(4.));
        assert_ne!(list.pop(), Some(5.3));
        assert_eq!(list.pop(), None);
    }


    #[test]
    fn test_iter_mut() {
        let mut list = List::new();
        list.push(5.5);
        list.push(4.0);
        list.push(3.);
        list.push(2.);

        let mut iter = list.iter_mut();

        assert_eq!(iter.next(), Some(&mut 2.));
        assert_eq!(iter.next(), Some(&mut 3.));
        assert_eq!(iter.next(), Some(&mut 4.));
        // assert_eq!(iter.next(), Some(&5.5));
        let five = iter.next();
        if let Some(val) = five {
            *val = 5.3;
        }
        
        assert_eq!(list.pop(), Some(2.));
        assert_eq!(list.pop(), Some(3.));
        assert_eq!(list.pop(), Some(4.));
        assert_eq!(list.pop(), Some(5.3));
        assert_eq!(list.pop(), None);
    }

}