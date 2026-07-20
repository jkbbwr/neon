enum Node {
    Leaf,
    Branch(Box<Node>, Box<Node>),
}

fn make(depth: i32) -> Node {
    if depth == 0 {
        Node::Leaf
    } else {
        Node::Branch(Box::new(make(depth - 1)), Box::new(make(depth - 1)))
    }
}

fn check(node: &Node) -> i64 {
    match node {
        Node::Leaf => 1,
        Node::Branch(left, right) => 1 + check(left) + check(right),
    }
}

fn main() {
    let max_depth: i32 = 18;
    let mut total: i64 = 0;

    let stretch = make(max_depth + 1);
    let sc = check(&stretch);
    println!("stretch tree of depth {} check: {}", max_depth + 1, sc);
    total += sc;

    let long_lived = make(max_depth);

    let mut depth = 4;
    while depth <= max_depth {
        let iterations: i64 = 1i64 << (max_depth - depth + 4);
        let mut sum: i64 = 0;
        for _ in 0..iterations {
            let t = make(depth);
            sum += check(&t);
        }
        println!("{} trees of depth {} check: {}", iterations, depth, sum);
        total += sum;
        depth += 2;
    }

    let ll = check(&long_lived);
    println!("long lived tree of depth {} check: {}", max_depth, ll);
    total += ll;

    println!("Result: {}", total);
}
