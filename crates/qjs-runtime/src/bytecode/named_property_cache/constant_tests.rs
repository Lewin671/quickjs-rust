//! Prototype constants invalidate at every write boundary and own no heap value.
use super::tests::object;
use super::*;
use crate::property::atom::AtomDomain;
use crate::{Function, NativeFunction};
use std::collections::HashMap;

#[test]
fn prototype_overwrites_transition_once_to_live_slots() {
    let domain = AtomDomain::new();
    let holder = object(&domain, "x", 0.0);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let site = NamedPropertyCache::default();
    for value in 1..=4 {
        assert_eq!(
            site.read_object(&receiver, "x"),
            Some(Value::Number((value - 1) as f64))
        );
        holder.set("x".into(), Value::Number(value as f64));
        assert!(site.get(&receiver).is_none());
    }
    assert!(!holder.with_named(|s| s.prototype_values_stable()));
    assert_eq!(site.read_object(&receiver, "x"), Some(Value::Number(4.0)));
    for value in 5..12 {
        holder.set("x".into(), Value::Number(value as f64));
        assert_eq!(site.get(&receiver), Some(Value::Number(value as f64)));
    }
    holder.set("new".into(), Value::Null);
    assert!(
        site.get(&receiver).is_none(),
        "unstable holders still invalidate structural edits"
    );
}

#[test]
fn every_slot_mutator_invalidates_an_inherited_constant() {
    for route in 0..6 {
        let domain = AtomDomain::new();
        let holder = object(&domain, "x", 1.0);
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
        let site = NamedPropertyCache::default();
        site.read_object(&receiver, "x");
        let key = domain.intern("x");
        holder.with_named_mut(|s| match route {
            0 => {
                s.overwrite_guarded_data(0, Value::Number(2.0));
            }
            1 => {
                s.overwrite_guarded_number(0, 2.0);
            }
            2 => {
                *s.data_value_mut(0).unwrap() = Value::Number(2.0);
            }
            3 => {
                s.write_existing_key(&key, &Value::Number(2.0), |_| true);
            }
            4 => {
                s.lookup(&key)
                    .unwrap()
                    .write_existing(&Value::Number(2.0))
                    .ok()
                    .unwrap();
            }
            _ => {
                s.define(&key, Property::data(Value::Number(2.0), true, true, true))
                    .unwrap();
            }
        });
        assert!(site.get(&receiver).is_none(), "route {route}");
        assert_eq!(site.read_object(&receiver, "x"), Some(Value::Number(2.0)));
    }
}

#[test]
fn accessor_replacement_invalidates_both_cached_halves() {
    let domain = AtomDomain::new();
    let holder = object(&domain, "x", 1.0);
    holder.define_property("x".into(), Property::accessor(None, None, true, true));
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let value = Value::Object(receiver);
    let site = NamedPropertyCache::default();
    assert!(matches!(
        site.read_outcome(&value, "x"),
        Some(ReadOutcome::Accessor(_))
    ));
    let method =
        Function::new_native_in_domain(Some("method"), 1, NativeFunction::MathAbs, false, &domain);
    holder.define_property(
        "x".into(),
        Property::accessor(Some(Value::Function(method.clone())), None, true, true),
    );
    assert!(site.accessor(&value).is_none());
    let Some(ReadOutcome::Accessor(property)) = site.read_outcome(&value, "x") else {
        panic!("accessor")
    };
    assert_eq!(property.getter(), Some(&Value::Function(method)));
}

#[test]
fn weak_constants_do_not_pin_functions_holders_or_recycled_keys() {
    let domain = AtomDomain::limited_for_test(1);
    let holder = object(&domain, "x", 1.0);
    let method = Function::new_native_in_domain(None, 0, NativeFunction::MathAbs, false, &domain);
    let weak_method = method.downgrade();
    holder.set("x".into(), Value::Function(method.clone()));
    let weak_holder = holder.downgrade();
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let site = NamedPropertyCache::default();
    let id = domain.find("x").unwrap().id();
    assert_eq!(
        site.read_object(&receiver, "x"),
        Some(Value::Function(method.clone()))
    );
    let weak_constant = site
        .first
        .get()
        .unwrap()
        .chain
        .as_ref()
        .unwrap()
        .constant
        .clone()
        .unwrap();
    drop(method);
    drop(receiver);
    drop(holder);
    assert!(
        weak_constant.data().is_none(),
        "dead weak constant must miss"
    );
    assert!(weak_method.upgrade().is_none());
    assert!(weak_holder.upgrade().is_none());
    let key = domain.try_intern("y").unwrap();
    assert_eq!(key.id(), id);
    let other = object(&domain, "y", 7.0);
    assert!(site.get(&other).is_none());
}

#[test]
fn foreign_domain_prototype_constants_follow_the_holder_cell() {
    let a = AtomDomain::new();
    let b = AtomDomain::new();
    let holder = object(&a, "x", 1.0);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &b);
    let site = NamedPropertyCache::default();
    assert_eq!(site.read_object(&receiver, "x"), Some(Value::Number(1.0)));
    holder.set("x".into(), Value::Number(2.0));
    assert!(site.get(&receiver).is_none());
    assert_eq!(site.read_object(&receiver, "x"), Some(Value::Number(2.0)));
}

#[test]
fn warmed_prototype_methods_observe_javascript_write_routes_and_string_append() {
    assert_eq!(
        crate::eval(
            r#"
      var p={m:function(){return 1}, text:'a'}, o=Object.create(p);
      function read(){return o.m()}; function text(){return o.text};
      function warm(){for(var i=0;i<50;i++){read();text()}};
      warm(); p.m=function(){return 2}; var result=read();
      warm(); Object.defineProperty(p,'m',{value:function(){return 3}}); result+=read();
      warm(); Reflect.set(p,'m',function(){return 4}); result+=read();
      warm(); Object.assign(p,{m:function(){return 5}}); result+=read();
      warm(); Object.defineProperties(p,{m:{value:function(){return 6}}}); result+=read();
      warm(); p.text+='b'; result===20 && text()==='ab';
    "#
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn marked_foreign_realm_method_updates_invalidate_warmed_sites() {
    assert_eq!(
        crate::eval(
            r#"
      var realm = Object.create(globalThis); realm.globalThis=realm;
      var __quickjsRustDynamicFunctionRealm=realm;
      globalThis.__quickjsRustDynamicFunctionRealm=realm;
      var make=Function('return {m:function(){return 1}}');
      __quickjsRustDynamicFunctionRealm=undefined;
      globalThis.__quickjsRustDynamicFunctionRealm=undefined;
      var p=make(), o=Object.create(p); function read(){return o.m()};
      for(var i=0;i<100;i++) read();
      p.m=function(){return 2}; read()===2;
    "#
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn typed_array_length_proof_observes_shadowing_and_replaced_getters_once() {
    assert_eq!(
        crate::eval(
            r#"
      var a=new Uint8Array(7), p=Object.getPrototypeOf(Uint8Array.prototype);
      var original=Object.getOwnPropertyDescriptor(p,'length'), calls=0;
      function read(x){return x.length}; function warm(){for(var i=0;i<100;i++)read(a)};
      warm(); var result=read(a)===7;
      Object.defineProperty(a,'length',{value:11,configurable:true});
      warm(); result=result && read(a)===11;
      delete a.length; warm(); result=result && read(a)===7;
      Object.defineProperty(p,'length',{get:function(){calls++;return 13},configurable:true});
      result=result && read(a)===13 && calls===1;
      Object.defineProperty(p,'length',original);
      result && read(a)===7;
    "#
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn weak_accessor_constants_release_the_pair_and_its_getter() {
    let domain = AtomDomain::new();
    let holder = object(&domain, "x", 1.0);
    let getter = Function::new_native_in_domain(None, 0, NativeFunction::MathAbs, false, &domain);
    let weak = getter.downgrade();
    holder.define_property(
        "x".into(),
        Property::accessor(Some(Value::Function(getter)), None, true, true),
    );
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let site = NamedPropertyCache::default();
    assert!(matches!(
        site.read_outcome(&Value::Object(receiver.clone()), "x"),
        Some(ReadOutcome::Accessor(_))
    ));
    let constant = site
        .first
        .get()
        .unwrap()
        .chain
        .as_ref()
        .unwrap()
        .constant
        .clone()
        .unwrap();
    drop(receiver);
    drop(holder);
    assert!(weak.upgrade().is_none());
    assert!(constant.fetch().is_none());
}
