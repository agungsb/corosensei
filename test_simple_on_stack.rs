use std::cell::Cell;
use std::rc::Rc;

fn main() {
    println!("Testing simple on_stack...");
    
    // Increase stack size to avoid overflow
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8MB stack
        .spawn(|| {
            let hit = Rc::new(Cell::new(false));
            let hit2 = hit.clone();
            
            let result = corosensei::on_stack(
                corosensei::stack::DefaultStack::default(), 
                move || {
                    println!("Inside on_stack closure!");
                    hit2.set(true);
                    "hello"
                }
            );
            
            println!("Result: {}", result);
            println!("Hit: {}", hit.get());
        })
        .unwrap()
        .join()
        .unwrap();
        
    println!("Test completed!");
}
