use corosensei::*;

fn main() {
    println!("Testing basic coroutine creation...");
    
    let coro = Coroutine::new(|yielder, input: i32| {
        println!("Inside coroutine with input: {}", input);
        yielder.suspend(input * 2);
        println!("Resumed coroutine");
        input * 3
    });
    
    match coro {
        Ok(mut coro) => {
            println!("Coroutine created successfully");
            match coro.resume(5) {
                CoroutineResult::Suspend(val) => {
                    println!("Got suspended value: {}", val);
                    match coro.resume(0) {
                        CoroutineResult::Return(val) => {
                            println!("Got final value: {}", val);
                        }
                        _ => println!("Unexpected result"),
                    }
                }
                _ => println!("Unexpected first result"),
            }
        }
        Err(e) => println!("Failed to create coroutine: {:?}", e),
    }
}
