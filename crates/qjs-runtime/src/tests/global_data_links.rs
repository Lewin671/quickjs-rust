use crate::{Value, eval};

#[test]
fn global_binding_and_property_alias_share_writes_in_both_directions() {
    assert_eq!(
        eval(
            "var linked = 1; var alias = globalThis; \
             linked = 2; var before = alias.linked; alias.linked = 3; \
             before + ':' + linked + ':' + globalThis.linked;"
        ),
        Ok(Value::String("2:3:3".to_owned().into()))
    );
}

#[test]
fn closures_share_the_global_property_cell() {
    assert_eq!(
        eval(
            "var linked = 1; \
             function read() { return linked; } \
             function write(value) { linked = value; } \
             globalThis.linked = 2; var before = read(); write(3); \
             before + ':' + linked + ':' + globalThis.linked;"
        ),
        Ok(Value::String("2:3:3".to_owned().into()))
    );
}

#[test]
fn alternating_global_cells_remain_independent() {
    assert_eq!(
        eval(
            "var left = 0, right = 0; \
             for (var i = 0; i < 100; i++) { left = i; right = i + 1; } \
             left + ':' + right + ':' + globalThis.left + ':' + globalThis.right;"
        ),
        Ok(Value::String("99:100:99:100".to_owned().into()))
    );
}

#[test]
fn descriptor_value_and_writable_updates_keep_the_cell_live() {
    assert_eq!(
        eval(
            "var linked = 1; \
             Object.defineProperty(globalThis, 'linked', { value: 2 }); \
             var before = linked; \
             Object.defineProperty(globalThis, 'linked', { value: 3, writable: false }); \
             linked = 4; var descriptor = Object.getOwnPropertyDescriptor(globalThis, 'linked'); \
             before + ':' + linked + ':' + globalThis.linked + ':' + descriptor.writable;"
        ),
        Ok(Value::String("2:3:3:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; var linked = 1; \
             Object.defineProperty(globalThis, 'linked', { writable: false }); \
             try { linked = 2; } catch (error) { \
               error.name + ':' + linked + ':' + globalThis.linked; \
             }"
        ),
        Ok(Value::String("TypeError:1:1".to_owned().into()))
    );
}

#[test]
fn seal_and_freeze_preserve_linked_values_and_attributes() {
    assert_eq!(
        eval(
            "var sealed = 1, frozen = 3; \
             Object.seal(globalThis); sealed = 2; \
             var before = sealed + ':' + globalThis.sealed + ':' + Object.isSealed(globalThis); \
             Object.freeze(globalThis); frozen = 4; \
             before + ':' + frozen + ':' + globalThis.frozen + ':' + Object.isFrozen(globalThis);"
        ),
        Ok(Value::String("2:2:true:3:3:true".to_owned().into()))
    );
}

#[test]
fn descriptors_and_enumeration_observe_the_cell_value() {
    assert_eq!(
        eval(
            "var visible = 9; let deleted = delete globalThis.visible; \
             var descriptor = Object.getOwnPropertyDescriptor(globalThis, 'visible'); \
             [deleted, Object.keys(globalThis).includes('visible'), \
              Object.getOwnPropertyNames(globalThis).includes('visible'), \
              descriptor.value, descriptor.enumerable, descriptor.writable, descriptor.configurable].join(':');"
        ),
        Ok(Value::String(
            "false:true:true:9:true:true:false".to_owned().into()
        ))
    );
}

#[test]
fn named_reads_do_not_cache_a_stale_linked_value() {
    assert_eq!(
        eval(
            "var cached = 1; function read(object) { return object.cached; } \
             var first = read(globalThis); cached = 2; var second = read(globalThis); \
             globalThis.cached = 3; first + ':' + second + ':' + read(globalThis);"
        ),
        Ok(Value::String("1:2:3".to_owned().into()))
    );
}

#[test]
fn direct_eval_and_with_keep_dynamic_resolution_authoritative() {
    assert_eq!(
        eval(
            "var linked = 1; \
             function localEval() { let linked = 10; eval('linked = 11'); return linked; } \
             var local = localEval(); eval('linked = 2'); \
             local + ':' + linked + ':' + globalThis.linked;"
        ),
        Ok(Value::String("11:2:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var linked = 1; var scope = { linked: 10 }; \
             with (scope) { linked = 11; } with ({}) { linked = 2; } \
             scope.linked + ':' + linked + ':' + globalThis.linked;"
        ),
        Ok(Value::String("11:2:2".to_owned().into()))
    );
}

#[test]
fn compound_string_writes_update_a_linked_cell_once() {
    assert_eq!(
        eval(
            "var text = 'a'; text += 'b'; var first = globalThis.text; \
             globalThis.text += 'c'; first + ':' + text + ':' + globalThis.text;"
        ),
        Ok(Value::String("ab:abc:abc".to_owned().into()))
    );
}

#[test]
fn read_only_linked_string_slots_preserve_strict_and_sloppy_assignment() {
    assert_eq!(
        eval(
            "var text = 'a'; Object.freeze(globalThis); \
             let result = (text += 'b'); result + ':' + text + ':' + globalThis.text;"
        ),
        Ok(Value::String("ab:a:a".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; var text = 'a'; Object.freeze(globalThis); \
             let outcome = 'none'; try { text += 'b'; } catch (error) { outcome = error.name; } \
             outcome + ':' + text + ':' + globalThis.text;"
        ),
        Ok(Value::String("TypeError:a:a".to_owned().into()))
    );
}

#[test]
fn captured_read_only_string_slots_cannot_bypass_the_descriptor() {
    assert_eq!(
        eval(
            "var text = 'a'; function append() { return text += 'b'; } \
             Object.defineProperty(globalThis, 'text', { writable: false }); \
             let result = append(); result + ':' + text + ':' + globalThis.text;"
        ),
        Ok(Value::String("ab:a:a".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; var text = 'a'; function append() { return text += 'b'; } \
             Object.defineProperty(globalThis, 'text', { writable: false }); \
             let outcome = 'none'; try { append(); } catch (error) { outcome = error.name; } \
             outcome + ':' + text + ':' + globalThis.text;"
        ),
        Ok(Value::String("TypeError:a:a".to_owned().into()))
    );
}

#[test]
fn read_only_numeric_writes_cover_all_operators_scopes_and_modes() {
    let setups = [
        ("freeze", "Object.freeze(globalThis);"),
        (
            "defineProperty",
            "Object.defineProperty(globalThis, 'x', { writable: false });",
        ),
    ];
    for (setup_name, setup) in setups {
        for strict in [false, true] {
            for closure in [false, true] {
                let directive = if strict { "'use strict';" } else { "" };
                let mutations = format!(
                    "{directive} \
                     let assigned = 'none', post = 'none', compound = 'none'; \
                     try {{ assigned = (x = 2); }} catch (error) {{ assigned = error.name; }} \
                     try {{ post = x++; }} catch (error) {{ post = error.name; }} \
                     try {{ compound = (x += 1); }} catch (error) {{ compound = error.name; }} \
                     return [assigned, post, compound, x, globalThis.x].join(':');"
                );
                let source = if closure {
                    format!("var x = 1; function mutate() {{ {mutations} }} {setup} mutate();")
                } else {
                    format!(
                        "{directive} var x = 1; \
                         let assigned = 'none', post = 'none', compound = 'none'; \
                         {setup} \
                         try {{ assigned = (x = 2); }} catch (error) {{ assigned = error.name; }} \
                         try {{ post = x++; }} catch (error) {{ post = error.name; }} \
                         try {{ compound = (x += 1); }} catch (error) {{ compound = error.name; }} \
                         [assigned, post, compound, x, globalThis.x].join(':');"
                    )
                };
                let expected = if strict {
                    "TypeError:TypeError:TypeError:1:1"
                } else {
                    "2:1:2:1:1"
                };
                assert_eq!(
                    eval(&source),
                    Ok(Value::String(expected.to_owned().into())),
                    "setup={setup_name}, strict={strict}, closure={closure}"
                );
            }
        }
    }
}

#[test]
fn proxy_forwarding_preserves_the_link_and_rejects_incompatible_redefinition() {
    assert_eq!(
        eval(
            "var linked = 1; var proxy = new Proxy(globalThis, {}); \
             proxy.linked = 2; var first = linked; \
             Object.defineProperty(proxy, 'linked', { value: 3 }); \
             let deleted = delete proxy.linked; let rejected = 'none'; \
             try { Object.defineProperty(proxy, 'linked', { get: function () { return 9; } }); } \
             catch (error) { rejected = error.name; } \
             var intercepted = new Proxy(globalThis, { set: function () { return true; } }); \
             intercepted.linked = 8; \
             [first, linked, globalThis.linked, deleted, rejected].join(':');"
        ),
        Ok(Value::String("2:3:3:false:TypeError".to_owned().into()))
    );
}
