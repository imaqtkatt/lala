use std::{fs::File, io::Read};

use eval::Env;
use lexer::Lexer;
use parser::Parser;

pub mod ast;
pub mod compile;
pub mod desugar;
pub mod eval;
pub mod lexer;
pub mod parser;

use desugar::Desugar;

fn main() -> std::io::Result<()> {
  // let lay = std::alloc::Layout::array::<u64>(2).unwrap();
  // let m = unsafe { std::alloc::alloc(lay) as *mut u64 };
  // unsafe { *m.offset(0) = 42 };
  // unsafe { *m.offset(1) = 43 };

  // println!("{m:?}");
  // println!("{:?}", unsafe { *m.offset(0) });

  // unsafe { std::alloc::dealloc(m as *mut u8, lay) };
  // Ok(())

  let mut args = std::env::args();
  if let Some(file_path) = args.nth(1) {
    let mut file = File::open(file_path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    let mut parser = Parser::new(Lexer::new(&buf));
    let program = parser.program().map_err(std::io::Error::other)?;
    let program = program.desugar().map_err(std::io::Error::other)?;
    let mut env = Env::from_program(program);
    loop {
      let mut buf = String::new();
      let _ = std::io::stdin().read_line(&mut buf)?;
      let mut parser = Parser::new(Lexer::new(&buf));
      let expr = parser.expression().map_err(std::io::Error::other)?;
      let res = env.eval(
        expr
          .desugar()
          .map_err(|_| std::io::Error::other("Desugar expr"))?,
      );
      println!("{res:?}");
    }
  } else {
    println!("Hello, world!");
    Ok(())
  }
}
