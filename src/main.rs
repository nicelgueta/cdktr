mod executor;
use executor::Executor;

#[tokio::main]
async fn main() {
    let exec = Executor::new(
        "python", Some(vec!["s.py".to_string()])
    );
    exec.run(|x| println!("Got: {}", x)).await;
}
