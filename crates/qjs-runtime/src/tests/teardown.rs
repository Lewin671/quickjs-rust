use crate::{Value, eval};

// Each script releases a 100,000-link chain; dropping it used to recurse once
// per link and overflow the native stack.

#[test]
fn releases_long_object_chains() {
    assert_eq!(
        eval(
            "var head = null;
             for (var i = 0; i < 100000; i++) head = { next: head };
             head = null;
             function Node(next) { this.next = next; this.id = 1; }
             for (var i = 0; i < 100000; i++) head = new Node(head);
             head = null;
             'done';"
        ),
        Ok(Value::String("done".to_owned().into()))
    );
}

#[test]
fn releases_long_array_and_mixed_chains() {
    assert_eq!(
        eval(
            "var head = null;
             for (var i = 0; i < 100000; i++) head = [head];
             head = null;
             for (var i = 0; i < 100000; i++) head = { a: 1, b: [{ next: head }] };
             head = null;
             'done';"
        ),
        Ok(Value::String("done".to_owned().into()))
    );
}

#[test]
fn releasing_a_chain_keeps_links_still_referenced_elsewhere() {
    assert_eq!(
        eval(
            "var head = null, middle;
             for (var i = 0; i < 1000; i++) {
                 head = { next: head, index: i };
                 if (i === 499) middle = head;
             }
             head = null;
             var count = 0;
             for (var node = middle; node; node = node.next) count++;
             count;"
        ),
        Ok(Value::Number(500.0))
    );
}
