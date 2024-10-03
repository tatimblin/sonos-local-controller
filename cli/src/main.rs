use sdk::helloworld::HelloWorld;

fn main() {
    let hw = HelloWorld {
        name: "Tristan".to_string(),
    };

    println!("{}", hw.greeting());
}