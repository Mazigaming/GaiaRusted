// Tuple structs and tuple variants are functions of their fields, so they
// can be passed wherever a function is expected.
#[derive(Debug, Clone, PartialEq)]
struct Meters(f64);

#[derive(Debug)]
enum Token {
    Num(i64),
    Word(String),
    Pair(char, char),
}

fn apply<T, U>(value: T, f: fn(T) -> U) -> U {
    f(value)
}

fn main() {
    let some: Vec<Option<i32>> = (1..4).map(Some).collect();
    let lengths: Vec<Meters> = [1.5, 2.0].into_iter().map(Meters).collect();
    let numbers: Vec<Token> = vec![3, 4].into_iter().map(Token::Num).collect();
    let words: Vec<Token> = "a bc".split(' ').map(String::from).map(Token::Word).collect();
    let oks: Vec<Result<i32, ()>> = (0..2).map(Ok).collect();
    let pair = Token::Pair;
    println!("{:?} {:?} {:?} {:?}", some, lengths, numbers, words);
    println!("{:?} {:?}", oks, pair('a', 'b'));
    println!("{:?} {:?}", apply(7, Some), apply(2.5, Meters));
    let boxed: Vec<Box<i32>> = vec![1, 2].into_iter().map(Box::new).collect();
    println!("{:?} {:?}", boxed, Some(5).map(Box::new));
}
