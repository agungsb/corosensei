use std::cell::Cell;
use std::rc::Rc;

fn main() {
    println!("Testing basic on_stack functionality...");
    
    let hit = Rc::new(Cell::new(false));
    let hit2 = hit.clone();
    
    unsafe {
        // Test calling our on_stack function
        corosensei::coroutine::on_stack(corosensei::stack::DefaultStack::default(), move || {
            println!("Inside function on new stack!");
            hit2.set(true);
            "hello"
        });
    }
    
    println!("Test completed, hit = {}", hit.get());
}
