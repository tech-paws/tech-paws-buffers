#[derive(Debug, Clone, PartialEq)]
pub enum MyEnum {
    Idle,
    Move {
        x: f64,
        y: f64,
    },
    Update(
        f64,
        f64,
        String,
    ),
}