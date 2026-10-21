"""Instrument an isolated source archive only; no layout fields/allocator changes."""
from pathlib import Path
import argparse
p=argparse.ArgumentParser();p.add_argument('source',type=Path);a=p.parse_args();root=a.source/'crates/qjs-runtime/src'
def edit(name,old,new):
 f=root/name;s=f.read_text();assert s.count(old)==1,(name,old,s.count(old));f.write_text(s.replace(old,new))
edit('lib.rs','mod diagnostics;','mod diagnostics;\nmod arch_diag;')
(root/'arch_diag.rs').write_text('''//! Diagnostic archive only: post-execution pointer capture, never timing.
#[inline(never)]
pub(crate) fn addr(ctx:usize,label:&str,index:usize,address:usize,bytes:usize) {
    eprintln!("ARCH_ADDR {{\\\"ctx\\\":{},\\\"label\\\":\\\"{}\\\",\\\"index\\\":{},\\\"address\\\":{},\\\"bytes\\\":{}}}",ctx,label,index,address,bytes);
}
''')
edit('function/upvalue.rs','    pub(crate) fn identity(&self) -> usize {','    pub(crate) fn arch_payload(&self) -> usize { self.0.as_ref().as_ptr() as usize }\n\n    pub(crate) fn identity(&self) -> usize {')
f=root/'function/env.rs';f.write_text(f.read_text()+'''
impl CallEnv {
 pub(crate) fn arch_capture(&self,ctx:usize,name:Option<&str>) {
  use crate::arch_diag::addr;
  let realm=&self.scope.realm;
  addr(ctx,"call_env",0,self as *const _ as usize,std::mem::size_of_val(self));
  addr(ctx,"realm_scope",0,Rc::as_ptr(&self.scope) as usize,std::mem::size_of_val(&*self.scope));
  addr(ctx,"realm",0,Rc::as_ptr(realm) as usize,std::mem::size_of_val(&**realm));
  addr(ctx,"global_bindings",0,realm.bindings.identity(),std::mem::size_of_val(&*realm.bindings.0));
  if let Some(Value::Object(object))=&realm.global_this { addr(ctx,"global_object",0,object.address(),0); }
  if let Some(name)=name { if let Some(cell)=realm.bindings.cell(name) {
    addr(ctx,"global_cell",0,cell.identity(),std::mem::size_of::<RefCell<Value>>());
    addr(ctx,"global_payload",0,cell.arch_payload(),std::mem::size_of::<Value>());
    eprintln!("ARCH_GLOBAL {} {:?}",ctx,name);
  }}
 }
}
''')
f=root/'value/js_string.rs';f.write_text(f.read_text()+'''
impl JsString {
 pub(crate) fn arch_addresses(&self)->(usize,usize,usize) {
  let view=self.0.matcher_view.borrow();
  (Rc::as_ptr(&self.0) as usize,self.0.text.as_ptr() as usize,
   view.as_ref().map_or(0,|(_,v)|v.as_ptr() as usize))
 }
}
''')
f=root/'bytecode/named_property_cache.rs';f.write_text(f.read_text()+'''
impl NamedPropertyCache { pub(crate) fn arch_address(&self)->usize { Rc::as_ptr(&self.0) as usize } }
''')
f=root/'bytecode/vm.rs';f.write_text(f.read_text()+'''
impl Vm<'_> {
 pub(super) fn arch_capture_frame(&self,ctx:usize) {
  use crate::arch_diag::addr;
  addr(ctx,"vm",0,self as *const _ as usize,std::mem::size_of_val(self));
  addr(ctx,"vm_frame",0,&self.current as *const _ as usize,std::mem::size_of_val(&self.current));
  addr(ctx,"vm_locals",0,self.locals.as_ptr() as usize,self.locals.len()*std::mem::size_of::<Slot>());
  addr(ctx,"vm_operands",0,self.stack.as_ptr() as usize,self.stack.capacity()*std::mem::size_of::<Value>());
  addr(ctx,"vm_upvalues",0,self.local_upvalues.as_ptr() as usize,self.local_upvalues.len()*std::mem::size_of::<Option<Upvalue>>());
 }
}
''')
edit('bytecode/typed_loop/frame.rs','    /// The environment calls and realm lookups','    fn arch_capture_frame(&self,ctx:usize);\n    /// The environment calls and realm lookups')
edit('bytecode/typed_loop/frame.rs',"impl LoopFrame for Vm<'_> {", "impl LoopFrame for Vm<'_> {\n    fn arch_capture_frame(&self,ctx:usize) { Vm::arch_capture_frame(self,ctx); }")
f=root/'bytecode/compact_fn/wide/loop_frame.rs';s=f.read_text();key="impl LoopFrame for WideLoopFrame<'_> {";assert key in s;s=s.replace(key,key+'''
 fn arch_capture_frame(&self,ctx:usize) {
  use crate::arch_diag::addr;
  addr(ctx,"wide_loop_frame",0,self as *const _ as usize,std::mem::size_of_val(self));
  addr(ctx,"wide_locals",0,self.locals.as_ptr() as usize,self.locals.len()*std::mem::size_of::<Value>());
  addr(ctx,"wide_operands",0,self.stack.as_ptr() as usize,self.stack.len()*std::mem::size_of::<Value>());
  addr(ctx,"bytecode",0,self.bytecode as *const _ as usize,std::mem::size_of_val(self.bytecode));
 }
''');f.write_text(s)
edit('bytecode/typed_loop/execute.rs','    program.recycle_scratch(scratch);','    arch_capture(vm,program,&scratch);\n    program.recycle_scratch(scratch);')
f=root/'bytecode/typed_loop/execute.rs';f.write_text(f.read_text()+'''
#[cold]
#[inline(never)]
fn arch_capture<F:LoopFrame>(vm:&F,program:&TypedLoopProgram,scratch:&TypedLoopScratch) {
 use crate::arch_diag::addr;
 let ctx=program as *const _ as usize;
 addr(ctx,"scalar",0,scratch.registers.as_ptr() as usize,4096);
 addr(ctx,"boxed",0,scratch.boxed.as_ptr() as usize,scratch.boxed.capacity()*std::mem::size_of::<Value>());
 addr(ctx,"receivers",0,scratch.receivers.as_ptr() as usize,scratch.receivers.capacity()*std::mem::size_of::<crate::ArrayRef>());
 addr(ctx,"typed_program",0,ctx,std::mem::size_of_val(program));
 addr(ctx,"typed_ops",0,program.ops.as_ptr() as usize,program.ops.len()*std::mem::size_of::<TypedOp>());
 addr(ctx,"typed_sites",0,program.sites.as_ptr() as usize,program.sites.len()*std::mem::size_of::<DeoptSite>());
 addr(ctx,"typed_constants",0,program.constant_registers.as_ptr() as usize,program.constant_registers.len()*std::mem::size_of::<(u16,Typed)>());
 let caches=program.shape_caches.borrow();
 addr(ctx,"shape_caches",0,caches.as_ptr() as usize,caches.capacity()*std::mem::size_of::<super::ShapeWays>());drop(caches);
 for i in 0..program.register_count {
  addr(ctx,"register_tag",i,&scratch.registers[i] as *const _ as usize,1);
  match &scratch.registers[i] {
   Typed::Number(n)=>addr(ctx,"register_payload",i,n as *const _ as usize,8),
   Typed::Boolean(v)=>addr(ctx,"register_payload",i,v as *const _ as usize,1),_=>{}
  }
 }
 for (i,op) in program.ops.iter().enumerate() {
  addr(ctx,"op_tag",i,op as *const _ as usize,1);
  eprintln!("ARCH_OP {} {} {:?}",ctx,i,op);
  match op {
   TypedOp::Binary{dst,left,right,op:kind}=>{addr(ctx,"op_dst",i,dst as *const _ as usize,2);addr(ctx,"op_left",i,left as *const _ as usize,2);addr(ctx,"op_right",i,right as *const _ as usize,2);addr(ctx,"op_binary",i,kind as *const _ as usize,std::mem::size_of_val(kind));},
   TypedOp::Update{dst,src,..}|TypedOp::ToNumeric{dst,src}|TypedOp::Move{dst,src}=>{addr(ctx,"op_dst",i,dst as *const _ as usize,2);addr(ctx,"op_src",i,src as *const _ as usize,2);},
   TypedOp::Exit{cond,..}|TypedOp::JumpIfFalsy{cond,..}=>addr(ctx,"op_cond",i,cond as *const _ as usize,2),_=>{}
  }
 }
 for (reg,name) in &program.global_reads {
  eprintln!("ARCH_SEED {} {} {:?}",ctx,reg,name);
  vm.loop_env().arch_capture(ctx,Some(name));
 }
 vm.loop_env().arch_capture(ctx,None);
 vm.arch_capture_frame(ctx);
 addr(ctx,"image_symbol",0,crate::arch_diag::addr as *const () as usize,0);
}
''')
edit('bytecode/compact_fn/wide/activation.rs','    storage.frames.clear();','    arch_capture_wide(bytecode,env,arguments,&result,&storage);\n    storage.frames.clear();')
f=root/'bytecode/compact_fn/wide/activation.rs';f.write_text(f.read_text()+'''
#[cold]
#[inline(never)]
fn arch_capture_wide(bytecode:&Bytecode,_env:&CallEnv,args:&[Value],result:&Result<Value,RuntimeError>,storage:&FrameStorage) {
 let Some(program)=super::program_for(bytecode) else { return; };
 if program.probed_backedges.is_empty() { return; }
 let ctx=program as *const _ as usize;
 eprintln!("ARCH_ENV {} {}",ctx,std::rc::Rc::as_ptr(_env.realm()) as usize);
 let arg=args.iter().find_map(|v| if let Value::String(s)=v {Some(s.arch_addresses())}else{None}).unwrap_or((0,0,0));
 let res=if let Ok(Value::String(s))=result {s.arch_addresses()}else{(0,0,0)};
 let cache=program.named_reads.first().map_or(0,|r|r.cache.arch_address());
 eprintln!("ARCH_WIDE {{\\"ctx\\":{},\\"bytecode\\":{},\\"registers\\":{},\\"register_capacity\\":{},\\"frames\\":{},\\"frame_capacity\\":{},\\"frame_size\\":{},\\"ops\\":{},\\"ops_count\\":{},\\"op_size\\":{},\\"constants\\":{},\\"named_reads\\":{},\\"named_writes\\":{},\\"global_cache\\":{},\\"first_cache\\":{},\\"arg_string\\":[{},{},{}],\\"result_string\\":[{},{},{}]}}",ctx,bytecode as *const _ as usize,storage.registers.as_ptr() as usize,storage.registers.capacity(),storage.frames.as_ptr() as usize,storage.frames.capacity(),std::mem::size_of::<WideFrame>(),program.ops.as_ptr() as usize,program.ops.len(),std::mem::size_of::<super::WideOp>(),bytecode.constants.as_ptr() as usize,program.named_reads.as_ptr() as usize,program.named_writes.as_ptr() as usize,program.global_reads.as_ptr() as usize,cache,arg.0,arg.1,arg.2,res.0,res.1,res.2);
}
''')
