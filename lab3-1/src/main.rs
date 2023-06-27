use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Instant,
    vec,
};

use clap::Parser;
use itertools::Itertools;

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
struct Args {
    #[arg()]
    input: Vec<i32>,
}

fn main() {
    let args = Args::parse();
    let len = args.input.len();

    let max_threads = 32;

    for nthread in 1..=max_threads {
        let nums = args.input.clone();
        let ops = &vec![
            Operation::Sum,
            Operation::Sub,
            Operation::Div,
            Operation::Mul,
        ];

        let number_permutations = Arc::new(nums.into_iter().permutations(len).collect::<Vec<_>>());

        let results = Arc::new(Mutex::new(BTreeSet::<String>::new()));

        // Start block calculation
        let time = Instant::now();

        std::thread::scope(|s| {
            let range = number_permutations.len() / nthread;

            for thread in 0..nthread {
                let number_permutations = number_permutations.clone();
                let results = results.clone();

                s.spawn(move || {
                    let thead_range = (range * thread)..if thread + 1 == nthread {
                        number_permutations.len()
                    } else {
                        range * (thread + 1)
                    };

                    for numbers in &number_permutations.as_slice()[thead_range] {
                        let operation_comb = permutations_with_replacement(ops, len - 1);

                        for ops in operation_comb {
                            if let Some(10) = calculate(numbers, &ops) {
                                let string = convert_combination(numbers, &ops);
                                results.lock().unwrap().insert(string);
                            }
                        }
                    }
                });
            }
        });

        println!(
            "nthreads with blocks:\t\t {}, t: {:?}, size: {}",
            nthread,
            time.elapsed(),
            results.lock().unwrap().len()
        );

        {
            results.lock().unwrap().clear();
        }

        // Start interleaved
        let time = Instant::now();

        std::thread::scope(|s| {
            for thread in 0..nthread {
                let number_permutations = number_permutations.clone();
                let results = results.clone();

                s.spawn(move || {
                    let thread_range = (0 + thread..number_permutations.len()).step_by(nthread);

                    let numbers = number_permutations.as_slice();
                    for index in thread_range {
                        let operation_comb = permutations_with_replacement(&ops, len - 1);

                        for ops in operation_comb {
                            if let Some(10) = calculate(&numbers[index], &ops) {
                                let string = convert_combination(&numbers[index], &ops);
                                results.lock().unwrap().insert(string);
                            }
                        }
                    }
                });
            }
        });

        println!(
            "nthreads with interleaved:\t {}, t: {:?}, size: {}",
            nthread,
            time.elapsed(),
            results.lock().unwrap().len()
        );
    }
}

fn convert_combination(nums: &Vec<i32>, ops: &Vec<&Operation>) -> String {
    let mut nums = nums.iter();
    let ops = ops.iter();
    let mut result = nums.next().unwrap().to_string();

    nums.zip(ops)
        .for_each(|(num, op)| result += &format!(" {} {}", op.to_string(), num));

    result
}

fn permutations_with_replacement<T: Copy>(
    items: &Vec<T>,
    k: usize,
) -> impl Iterator<Item = Vec<&T>> {
    std::iter::repeat(items.iter())
        .take(k)
        .multi_cartesian_product()
}

fn calculate(nums: &Vec<i32>, ops: &Vec<&Operation>) -> Option<i32> {
    let mut nums = nums.iter();
    let mut partial = *nums.next()?;

    for (num, op) in nums.zip(ops.iter()) {
        match op {
            Operation::Div => {
                if *num == 0 {
                    return None;
                }
                partial = partial / (*num);
            }
            Operation::Mul => partial = partial * (*num),
            Operation::Sub => partial = partial - (*num),
            Operation::Sum => partial = partial + (*num),
        }
    }

    return Some(partial);
}
