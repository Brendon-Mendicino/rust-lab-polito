use std::{vec, collections::BTreeSet, time::Instant, sync::Mutex};

use clap::Parser;
use itertools::{Itertools, Permutations};

#[derive(Debug, Clone, Copy)]
enum Operation {
    Sum,
    Sub,
    Div,
    Mul,
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            Operation::Sum => "+".into(),
            Operation::Sub => "-".into(),
            Operation::Div => "/".into(),
            Operation::Mul => "*".into(),
        }
    }
}

#[derive(Debug, Parser)]
struct Args  {
    #[arg()]
    input: Vec<i32>,
}


fn main() {
    let args = Args::parse();
    let len = args.input.len();

    let max_threads = 32;
    
    for nthread in 1..=max_threads {
        let nums = args.input.clone();
        let ops = &vec![Operation::Sum, Operation::Sub, Operation::Div, Operation::Mul];
        let mut number_perm = permutations(&nums, len);

        let results = Mutex::new(BTreeSet::<String>::new());
        
        let time = Instant::now();

        std::thread::scope(|s| {
            for thread in 1..=nthread {
                s.spawn(|| {
                    for numbers in number_perm.by_ref().skip(len/nthread).take(len/nthread) {
                        let operation_comb = permutations_with_replacement(&ops, len - 1);
                        for ops in operation_comb {
                            if let Some(10) = calculate(&numbers, &ops) {
                                let string = convert_combination(&numbers, &ops);
                                results.lock().unwrap().insert(string);
                            }
                        }
                    }

                });
            }
            // s.spawn(|| {
            //     for numbers in number_perm.skip(len/nthread).take(len/nthread) {
            //         let operation_comb = permutations_with_replacement(&ops, len - 1);
            //         for ops in operation_comb {
            //             if let Some(10) = calculate(&numbers, &ops) {
            //                 let string = convert_combination(&numbers, &ops);
            //                 results.lock().unwrap().insert(string);
            //             }
            //         }
            //     }
            // });
        });

        println!("nthreads: {}, t: {:?}", nthread, time.elapsed());
    }
}

fn convert_combination(nums: &Vec<&i32>, ops: &Vec<&Operation>) -> String {
    let mut nums = nums.iter();
    let ops = ops.iter();
    let mut result = nums.next().unwrap().to_string();

    nums.zip(ops).for_each(|(num, op)| result += &format!(" {} {}", op.to_string(), num));

    result
}

fn permutations_with_replacement<T: Copy>(items: &Vec<T>, k: usize) -> impl Iterator<Item = Vec<&T>> {
    std::iter::repeat(items.iter()).take(k).multi_cartesian_product()
}

fn permutations<T: Copy>(items: &Vec<T>, k: usize) -> impl Iterator<Item = Vec<&T>> {
    items.iter().permutations(k)
}

fn calculate(nums: &Vec<&i32>, ops: &Vec<&Operation>) -> Option<i32> {
    let mut nums = nums.iter();
    let mut prev = **nums.next().unwrap();

    for (num, op) in nums.zip(ops.iter()) {
        match op {
            Operation::Div => {
                if **num == 0 {
                    return None;
                }
                prev = prev / (**num);
            },
            Operation::Mul => prev = prev * (**num),
            Operation::Sub => prev = prev - (**num),
            Operation::Sum => prev = prev + (**num),
        }
    }
    
    return Some(prev);
}