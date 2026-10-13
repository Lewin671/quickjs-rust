use super::{Value, eval};

#[test]
fn derived_eval_program_escaping_callbacks_keep_activation_cells_distinct() {
    assert_eq!(
        eval(include_str!(
            "../../../../../tests/fixtures/compare-qjs/derived-eval-derived_callback_super_effects_survive_eval_success_and_exception.js"
        )),
        Ok(Value::String(
            "base|field|after-bind|base|field|after-bind;4;4;7;7;true".into()
        ))
    );
}

#[test]
fn derived_native_eval_super_initializes_each_activation() {
    assert_eq!(
        eval(
            "class B { constructor() { this.x = 4; } }
              class D extends B { constructor() { eval('super()'); } }
              [new D().x, new D().x].join(',');"
        ),
        Ok(Value::String("4,4".into()))
    );
}

#[test]
fn derived_native_eval_this_preserves_cold_and_warm_tdz() {
    assert_eq!(
        eval(
            "class B {}
              class D extends B { constructor() {
                var ok=[];
                try { eval('this'); } catch(e) { ok.push(e instanceof ReferenceError); }
                try { eval('this'); } catch(e) { ok.push(e instanceof ReferenceError); }
                super(); this.ok=ok.join(',');
              }}
              [new D().ok,new D().ok].join('|');"
        ),
        Ok(Value::String("true,true|true,true".into()))
    );
}

#[test]
fn derived_native_unbound_completion_throws_but_object_return_is_legal() {
    assert_eq!(
        eval(
            "class B {}
              class Unbound extends B { constructor(){ eval('1'); } }
              class ObjectReturn extends B { constructor(){ eval('1'); return {}; } }
              var error; try { new Unbound(); } catch(e) { error=e instanceof ReferenceError; }
              error && typeof new ObjectReturn() === 'object';"
        ),
        Ok(Value::Boolean(true))
    );
}

fn assert_string(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())));
}

#[test]
fn derived_duplicate_super_performs_base_effects_before_rejecting_binding() {
    assert_string(
        "var events=[];
         class B { constructor(label){events.push('base:'+label);this.x=4;} }
         class D extends B { field=(events.push('field'),1); constructor(){
           eval(\"super((events.push('arg:1'),'one'))\");
           try {eval(\"super((events.push('arg:2'),'two'))\");}
           catch(e){events.push(e.name);}
           events.push('this:'+eval('this').x);
         }}
         new D(); new D(); events.join('|');",
        "arg:1|base:one|field|arg:2|base:two|ReferenceError|this:4|arg:1|base:one|field|arg:2|base:two|ReferenceError|this:4",
    );
}

#[test]
fn derived_base_throw_preserves_tdz_and_allows_a_later_binding() {
    assert_string(
        "var events=[];
         class B { constructor(fail){events.push('base:'+fail);if(fail)throw Error('base-failure');this.x=4;} }
         class D extends B { field=(events.push('field'),1); constructor(){
           var read=()=>eval('this');
           try{eval('super(true)');}catch(e){events.push(e.message);}
           try{read();}catch(e){events.push(e.name);}
           eval('super(false)');events.push('this:'+read().x);
         }}
         new D();events.join('|');",
        "base:true|base-failure|ReferenceError|base:false|field|this:4",
    );
}

#[test]
fn derived_field_throw_preserves_binding_and_duplicate_super_order() {
    assert_string(
        "var events=[];
         function fail(){events.push('field');throw Error('field-failure');}
         class B {constructor(){events.push('base');this.x=4;}}
         class D extends B {field=fail();constructor(){
           var read=()=>eval('this');
           try{eval('super()');}catch(e){events.push(e.message);}
           events.push('this:'+read().x);
           try{eval('super()');}catch(e){events.push(e.name);}
         }}
         var d=new D();events.join('|')+';'+d.x+';'+('field' in d);",
        "base|field|field-failure|this:4|base|ReferenceError;4;false",
    );
}

#[test]
fn derived_escaping_nested_arrows_retain_cells_owner_and_new_target() {
    assert_string(
        "var events=[];
         class B {constructor(label){events.push('B:'+label);this.label=label;}}
         class C {constructor(label){events.push('C:'+label);this.label=label;}}
         class D extends B {field=(events.push('D-field'),7);constructor(){
           return {init:label=>eval('super(label)'),read:()=>eval('this'),
                   nested:()=>()=>eval('this'),target:()=>eval('new.target')};
         }}
         function Other(){}
         var a=Reflect.construct(D,[],Other),b=Reflect.construct(D,[],Other),
             ra=a.nested(),rb=b.nested();
         try{ra();}catch(e){events.push(e.name);}
         try{rb();}catch(e){events.push(e.name);}
         Object.setPrototypeOf(D,C);
         a.init('a');b.init('b');
         [events.join('|'),ra()===a.read(),rb()===b.read(),ra()!==rb(),
          ra().label,rb().label,ra().field,rb().field,
          Object.getPrototypeOf(ra())===Other.prototype,a.target()===Other].join(';');",
        "ReferenceError|ReferenceError|C:a|D-field|C:b|D-field;true;true;true;a;b;7;7;true;true",
    );
}

#[test]
fn derived_reentrant_base_callback_binds_the_same_cell_once() {
    assert_string(
        "var hits=0,events=[],init;
         class B {constructor(){var n=++hits;events.push('B'+n);if(n===1)init();this.id=n;}}
         class D extends B {field=(events.push('field'),8);constructor(){
           init=()=>eval('super()');
           try{eval('super()');}catch(e){events.push(e.name);}
           events.push('this:'+eval('this').id);
         }}
         var d=new D();[events.join('|'),hits,d.id,d.field].join(';');",
        "B1|B2|field|ReferenceError|this:2;2;2;8",
    );
}

#[test]
fn derived_callback_super_effects_survive_eval_success_and_exception() {
    let result = eval(
        "var events=[];
         class B {constructor(){events.push('base');this.x=4;}}
         class D extends B {field=(events.push('field'),7);constructor(){
           return {init:()=>{super();throw Error('after-bind');},read:()=>eval('this')};
         }}
         function invoke(init){try{eval('init()');}catch(e){events.push(e.message);}}
         var a=new D(),b=new D();invoke(a.init);invoke(b.init);
         [events.join('|'),a.read().x,b.read().x,a.read().field,b.read().field,
          a.read()!==b.read()].join(';');",
    );
    assert_eq!(
        result,
        Ok(Value::String(
            "base|field|after-bind|base|field|after-bind;4;4;7;7;true".into()
        ))
    );
}

#[test]
fn derived_strict_eval_declarations_do_not_capture_implicit_this_writeback() {
    assert_string(
        "class B {constructor(){this.x=4;}}
         class D extends B {constructor(){var read=()=>this;
           eval('var hidden=1; super()');
           this.ok=read()===this && typeof hidden==='undefined';
         }}
         [new D().ok,new D().ok].join(',');",
        "true,true",
    );
}

#[test]
fn derived_this_tdz_precedes_super_getter_and_does_not_use_global_names() {
    assert_string(
        "var root=globalThis,events=[];
         root.this=99;root.globalThis={spoof:true};
         class B {get value(){events.push('getter');return 4;}}
         class D extends B {constructor(){
           for(var source of ['this','typeof this','super.value']){
             try{eval(source);}catch(e){events.push(e.name);}
           }
           super();events.push(eval('super.value'));
         }}
         new D();root.globalThis=root;events.join('|');",
        "ReferenceError|ReferenceError|ReferenceError|getter|4",
    );
}

#[test]
fn nonderived_undefined_and_lexical_this_do_not_acquire_caller_tdz() {
    assert_string(
        "function strictRead(){'use strict';return eval('this')===undefined;}
         var receiver={tag:7,make:function(){return ()=>this;}},read=receiver.make();
         class B {}
         class D extends B {constructor(){var good=read()===receiver && strictRead();
           var global=(0,eval)('this')===globalThis;
           super();this.good=good;this.global=global;
         }}
         var d=new D();[d.good,d.global,typeof eval('this')==='object'].join(',');",
        "true,true,true",
    );
}

#[test]
fn derived_eval_created_arrows_share_the_original_this_cell() {
    assert_string(
        "class B {constructor(){this.x=4;}}
         class D extends B {constructor(){
           var init=eval('()=>super()'),read=eval('()=>this'),error;
           try{read();}catch(e){error=e instanceof ReferenceError;}
           init();this.ok=error && read()===this;
         }}
         [new D().ok,new D().ok].join(',');",
        "true,true",
    );
}

#[test]
fn derived_escaping_super_uses_private_field_owner_not_new_target_owner() {
    assert_string(
        "var events=[],finish;
         class B {constructor(){events.push('B');this.base='B';}}
         class Wrong {constructor(){events.push('wrong');this.base='wrong';}}
         class D extends B {#secret=(events.push('D-field'),'D');read(){return this.#secret;}
           constructor(){finish=()=>()=>super();return {};}}
         function Other(){} Object.setPrototypeOf(Other,Wrong);
         Reflect.construct(D,[],Other);var instance=finish()();
         [instance.base,D.prototype.read.call(instance),
          Object.getPrototypeOf(instance)===Other.prototype,events.join(',')].join('|');",
        "B|D|true|B,D-field",
    );
}

#[test]
fn derived_nested_eval_two_activations() {
    assert_string(
        "var events=[];class B{constructor(){events.push('base');this.x=4;}}\nclass D extends B{field=(events.push('field'),7);constructor(){\ntry{eval(\"eval('this')\");}catch(e){events.push(e.name);}\neval(\"eval('super()')\");events.push(eval(\"eval('this')\").x+':'+this.field);}}\nnew D();new D();events.join('|');",
        "ReferenceError|base|field|4:7|ReferenceError|base|field|4:7",
    );
}

#[test]
fn derived_parameter_default_eval_arrow() {
    assert_string(
        "var events=[];class B{constructor(){events.push('base');this.x=4;}}\nclass D extends B{field=(events.push('field'),7);constructor(read=()=>eval('this'),init=()=>eval('super()')){\ntry{read();}catch(e){events.push(e.name);}\ninit();this.ok=read()===this;events.push(this.x+':'+this.field+':'+this.ok);}}\nnew D();new D();events.join('|');",
        "ReferenceError|base|field|4:7:true|ReferenceError|base|field|4:7:true",
    );
}

#[test]
fn derived_with_created_derived_class() {
    assert_string(
        "var events=[];function make(){with({label:'scope'}){return class D extends Object{field=(events.push(label),7);constructor(){\nvar read=()=>eval('this');try{read();}catch(e){events.push(e.name);}\neval('super()');events.push(eval('label')+':'+read().field);}}}}\nvar D=make();new D();new D();events.join('|');",
        "ReferenceError|scope|scope:7|ReferenceError|scope|scope:7",
    );
}
