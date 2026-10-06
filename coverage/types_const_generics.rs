struct Matrix<const R: usize, const C: usize> { data: [[i32; C]; R] }
impl<const R: usize, const C: usize> Matrix<R, C> {
    fn transpose(&self) -> Matrix<C, R> {
        let mut data = [[0; R]; C];
        for i in 0..R { for j in 0..C { data[j][i] = self.data[i][j]; } }
        Matrix { data }
    }
}
fn sum<const N: usize>(a: [i32; N]) -> i32 { a.iter().sum() }
fn main() {
    let m = Matrix { data: [[1, 2, 3], [4, 5, 6]] };
    let t = m.transpose();
    println!("{:?}", t.data);
    println!("{} {}", sum([1, 2, 3]), sum([10; 5]));
}
