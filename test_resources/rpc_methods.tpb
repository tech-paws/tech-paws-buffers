#[namespace = "test"]

fn print_hello_world();

fn hello_world() -> String;

fn say_hello(name: String) -> String;

fn sum(a: i32, b: f32, c: f64);

async fn print_hello_world_async();

async fn hello_world_async() -> String;

async fn say_hello_async(name: String) -> String;

async fn sum_async(a: i32, b: f32, c: f64);

read fn trigger;

read fn theme -> Theme;

read async fn trigger_async;

read async fn theme_async -> Theme;
