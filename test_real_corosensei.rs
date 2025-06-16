use std::cell::Cell;
use std::rc::Rc;

fn main() {
    println!("Testing real corosensei on_stack...");
    
    // Set larger stack size to see if it's just stack overflow
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8MB stack
        .spawn(|| {
            let hit = Rc::new(Cell::new(false));
            let hit2 = hit.clone();
            
            let result = corosensei::coroutine::on_stack(
                corosensei::stack::DefaultStack::default(), 
                move || {
                    println!("Inside corosensei closure!");
                    hit2.set(true);
                    "hello".to_string()
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
