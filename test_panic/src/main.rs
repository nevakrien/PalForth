use std::panic::{self, PanicHookInfo};

fn main() {
    // Set custom panic hook
    panic::set_hook(Box::new(|info: &PanicHookInfo| {
        println!("First panic: {}", info);
        // Trigger a second panic inside the panic hook
        panic!("Second panic inside panic hook!");
    }));

    // This panic will trigger the hook
    panic!("Initial panic");
}
