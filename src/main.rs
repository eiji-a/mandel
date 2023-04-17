//
use mandel::run;
use std::str::FromStr;


fn main() {
  let numbers = if std::env::args().len() <= 1 {
    let default = vec![100u32, 100u32];
    println!("No numbers were provided, defaulting to {:?}", default);
    default
  } else {
    std::env::args()
      .skip(1)
      .map(|s| u32::from_str(&s).expect("You must pass a list of positive integers!"))
      .collect()
  };
    
  pollster::block_on(run(&numbers[0], &numbers[1]));
}

