use std::hint::black_box;
fn main(){
 let args:Vec<_>=std::env::args().collect();let mode=&args[1];let c:usize=args[2].parse().unwrap();let k:usize=args[3].parse().unwrap();let n:usize=args[4].parse().unwrap();
 let source="g()";let context=qjs_parser::EvalParseContext{in_function:true,..Default::default()};let ast=qjs_parser::parse_direct_eval_script(source,context.clone()).unwrap();
 let mut total=0;
 for _ in 0..n {for _ in 0..c {for _ in 0..k {
  match mode.as_str(){
   "lex"=>total+=black_box(qjs_lexer::lex(black_box(source)).unwrap()).len(),
   "parse"=>{black_box(qjs_parser::parse_direct_eval_script(black_box(source),context.clone()).unwrap());total+=1;},
   "compile"=>{black_box(qjs_runtime::compile_script(black_box(&ast)).unwrap());total+=1;},
   "combined"=>{let ast=qjs_parser::parse_direct_eval_script(black_box(source),context.clone()).unwrap();black_box(qjs_runtime::compile_script(&ast).unwrap());total+=1;},
   _=>panic!("unknown mode"),
  }
 }}}
 println!("OK {}",black_box(total));
}
