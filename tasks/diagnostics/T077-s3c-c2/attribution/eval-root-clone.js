// T077 S3c C2 attribution: borrowed root bodies (eval) are deep-cloned per entry by
// `impl From<&Bytecode> for FrameBytecode` (frame_program.rs: Rc::new(body.clone())),
// losing OnceCell caches, so the virtual-object program is re-lowered every call
// (~36 allocations per entry in date-format-tofte). Usage: qjs eval-root-clone.js N
function run(n) { var s = 0; for (var i = 0; i < n; i++) s += eval("var x = i; x + 1"); return s; }
run(Number(scriptArgs[1] || 20000));
