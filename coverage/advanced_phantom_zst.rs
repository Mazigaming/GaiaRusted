use std::marker::PhantomData;
#[derive(Debug, Clone, Copy)] struct Meters;
#[derive(Debug, Clone, Copy)] struct Feet;
#[derive(Debug, Clone, Copy)]
struct Length<U> { value: f64, unit: PhantomData<U> }
impl<U> Length<U> { fn new(v: f64) -> Self { Length { value: v, unit: PhantomData } } fn add(self, o: Self) -> Self { Length::new(self.value + o.value) } }
impl Length<Feet> { fn to_meters(self) -> Length<Meters> { Length::new(self.value * 0.3048) } }
fn main() {
    let a: Length<Feet> = Length::new(10.0);
    let b = a.add(Length::new(5.0));
    println!("{:.3}", b.to_meters().value);
    println!("{}", std::mem::size_of::<Length<Meters>>());
    println!("{}", std::mem::size_of::<PhantomData<String>>());
    let units: Vec<()> = vec![(); 3];
    println!("{}", units.len());
}
