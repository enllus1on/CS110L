use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    

    for i in 1..12 {
        list.push_front(i.to_string());
    }

    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display

    let new = list.clone();
    println!("{}", new);

    if new == list {
        println!("PartialEq succ");
    }
    
    println!("IntoIterator trait for list");
    for val in list {
        print!("{} ", val);
    }
    println!("");

    println!("IntoIterator trait for &list");
    for val in &new {
        print!("{} ", val);

    }
    println!("");

}
