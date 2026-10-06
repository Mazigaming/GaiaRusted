// Stable sorting: equal elements keep their order, at every length the
// merge sort and its insertion sort cut-over handle.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Item { key: u32, label: String }
fn main() {
    let mut seed = 7u64;
    let mut next = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1); (seed >> 33) as u32 };
    for len in [0usize, 1, 2, 3, 19, 20, 21, 40, 41, 100, 1000, 5000] {
        let mut items: Vec<Item> = (0..len).map(|i| Item { key: next() % 50, label: format!("i{}", i) }).collect();
        let mut expected = items.clone();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        expected.sort_by_key(|item| item.key);
        
        let sorted = items.windows(2).all(|w| w[0].key <= w[1].key);
        
        
        println!("{} {} {} {:?}", len, sorted, items == expected, items.first().map(|i| (&i.label, i.key)));
    }
    let mut words: Vec<String> = "the quick brown fox jumps over the lazy dog again and again".split(' ').map(String::from).collect();
    words.sort();
    println!("{:?}", words);
    words.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    println!("{:?}", words);
    let mut nums: Vec<i64> = (0..10000).map(|i| (i * 7919 % 10007) as i64 - 5000).collect();
    nums.sort_unstable();
    println!("{} {} {:?}", nums[0], nums[9999], &nums[5000..5003]);
    let mut sorted_already: Vec<u32> = (0..100000).collect();
    sorted_already.sort();
    let mut reversed: Vec<u32> = (0..100000).rev().collect();
    reversed.sort();
    println!("{} {}", sorted_already[99999], reversed[0]);
    let mut pairs = vec![(2, "b"), (1, "z"), (2, "a"), (1, "y")];
    pairs.sort_by_key(|p| p.0);
    println!("{:?}", pairs);
}
